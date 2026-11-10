use prono_api::{Answer, Results, Submission, Survey};
use serde::{Deserialize, Serialize};

use crate::client::{Backend, Pending};

/// Seconds the name must stay unchanged before the backend is asked about it.
pub const NAME_CHECK_DELAY: f64 = 0.5;

/// What the participant is doing.
#[derive(Default, Serialize, Deserialize)]
pub enum Phase {
    /// Entering a name and picking a survey.
    #[default]
    Start,
    /// Filling in a survey; `answers` follow the survey's question order.
    Answering { survey_id: u64, answers: Vec<Answer> },
    /// Submitted; showing everyone's answers.
    Done { survey_id: u64 },
}

/// Something the participant asked for in the UI.
#[derive(Clone, Copy)]
pub enum Action {
    StartSurvey(u64),
    ResetAnswers,
    Submit,
    ChooseAnotherSurvey,
    /// Back from the questions to the name field, e.g. after the name was
    /// refused.
    ChangeName,
    DismissError,
}

/// The app's state and flow, without any drawing: the UI shows it, edits the
/// answers in place, and turns clicks into [`Action`]s.
#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub struct State {
    pub user_name: String,
    pub phase: Phase,
    #[serde(skip)]
    pub results: Option<Results>,
    /// Shown to the participant until dismissed.
    #[serde(skip)]
    pub error: Option<String>,
    #[serde(skip)]
    request: Option<Request>,
    /// The earlier answers of a returning participant, being fetched. Apart
    /// from `request` because it must not keep the participant from answering.
    #[serde(skip)]
    refill: Option<Refill>,
    #[serde(skip)]
    name_check: NameCheck,
}

/// Looking up what `user` answered to a survey before.
struct Refill {
    user: String,
    results: Pending<Results>,
}

enum Request {
    Submit(Pending<()>),
    Results(Pending<Results>),
}

/// Finding out, while the name is typed, whether it already filled in a
/// survey.
#[derive(Default)]
struct NameCheck {
    /// The trimmed name as last seen, and when it changed (in seconds).
    typed: String,
    changed_at: f64,
    /// The name being asked about, and the answer to come.
    pending: Option<(String, Pending<bool>)>,
    /// The last answer: a name and whether it already filled in a survey.
    answer: Option<(String, bool)>,
}

impl State {
    /// Drops restored state that no longer fits the survey catalog.
    #[must_use]
    pub fn restored(mut self) -> Self {
        let fits = match &self.phase {
            Phase::Start => true,
            Phase::Answering { survey_id, answers } => prono_api::survey(*survey_id).is_some_and(|survey| {
                survey.questions.len() == answers.len()
                    && survey.questions.iter().zip(answers).all(|(q, a)| q.kind == a.kind())
            }),
            Phase::Done { survey_id } => prono_api::survey(*survey_id).is_some(),
        };
        if !fits {
            self.phase = Phase::Start;
        }
        self
    }

    /// The survey being answered or shown, if any.
    pub fn survey(&self) -> Option<&'static Survey> {
        match self.phase {
            Phase::Start => None,
            Phase::Answering { survey_id, .. } | Phase::Done { survey_id } => prono_api::survey(survey_id),
        }
    }

    /// Whether a request is in flight.
    pub fn is_busy(&self) -> bool {
        self.request.is_some()
    }

    pub fn apply(&mut self, action: Action, backend: &impl Backend) {
        match action {
            Action::StartSurvey(survey_id) => self.start(survey_id, backend),
            Action::ResetAnswers => {
                let survey = self.survey();
                if let (Phase::Answering { answers, .. }, Some(survey)) = (&mut self.phase, survey) {
                    *answers = survey.empty_answers();
                }
            }
            Action::Submit => self.submit(backend),
            Action::ChooseAnotherSurvey => self.back_to_start(),
            Action::ChangeName => {
                // Only while nothing is in flight: a submission must not
                // finish unseen.
                if !self.is_busy() {
                    self.back_to_start();
                    self.error = None;
                }
            }
            Action::DismissError => self.error = None,
        }
    }

    fn back_to_start(&mut self) {
        self.phase = Phase::Start;
        self.results = None;
        self.request = None;
        self.refill = None;
    }

    /// Takes in finished requests, and fetches results when they are
    /// missing. Call it every frame.
    pub fn poll(&mut self, backend: &impl Backend) {
        self.poll_refill();
        match self.request.take() {
            Some(Request::Submit(pending)) => match pending.take() {
                None => self.request = Some(Request::Submit(pending)),
                Some(Ok(())) => {
                    if let Phase::Answering { survey_id, .. } = self.phase {
                        self.phase = Phase::Done { survey_id };
                        self.results = None;
                        // The name has just filled in a survey.
                        self.name_check.answer = None;
                    }
                }
                Some(Err(err)) => self.error = Some(err),
            },
            Some(Request::Results(pending)) => match pending.take() {
                None => self.request = Some(Request::Results(pending)),
                Some(Ok(results)) => self.results = Some(results),
                Some(Err(err)) => self.error = Some(err),
            },
            None => {}
        }

        // After an error, wait until it is dismissed before retrying.
        if let Phase::Done { survey_id } = self.phase
            && self.results.is_none()
            && self.request.is_none()
            && self.error.is_none()
        {
            self.request = Some(Request::Results(backend.results(survey_id)));
        }
    }

    /// Fills the questions with the earlier answers of the participant, once
    /// they arrive. The lookup is a convenience: when it fails or finds
    /// nothing, the participant just answers.
    fn poll_refill(&mut self) {
        let Some(refill) = self.refill.take() else { return };
        let Some(found) = refill.results.take() else {
            self.refill = Some(refill);
            return;
        };
        let survey = self.survey();
        let (Phase::Answering { answers, .. }, Some(survey), Ok(results)) = (&mut self.phase, survey, found) else {
            return;
        };
        // Answers the participant has already changed are theirs to keep.
        if *answers == survey.empty_answers()
            && let Some(earlier) = survey.answers_of(&results, &refill.user)
        {
            *answers = earlier;
        }
    }

    /// Asks the backend whether the typed name already filled in a survey,
    /// once the name stayed unchanged for [`NAME_CHECK_DELAY`] (`now` is in
    /// seconds). It only asks for a changed, valid name, so moving the cursor
    /// or clicking around costs nothing. Call it every frame.
    ///
    /// Returns how many seconds to wait before it must be called again, if
    /// it is waiting for the typing to pause.
    pub fn check_name(&mut self, backend: &impl Backend, now: f64) -> Option<f64> {
        let check = &mut self.name_check;
        if let Some((name, pending)) = check.pending.take() {
            match pending.take() {
                None => check.pending = Some((name, pending)),
                Some(Ok(answered)) => check.answer = Some((name, answered)),
                // Only a hint: a real problem shows when submitting. Do not
                // ask again until the name changes.
                Some(Err(_)) => check.answer = Some((name, false)),
            }
        }
        if !matches!(self.phase, Phase::Start) {
            return None;
        }

        let name = self.user_name.trim();
        if name != check.typed {
            check.typed = name.to_string();
            check.changed_at = now;
        }
        let known = |asked: &str| asked == name;
        if prono_api::check_user_name(name).is_err()
            || check.answer.as_ref().is_some_and(|(asked, _)| known(asked))
            || check.pending.as_ref().is_some_and(|(asked, _)| known(asked))
        {
            return None;
        }

        let wait = NAME_CHECK_DELAY - (now - check.changed_at);
        if wait > 0.0 {
            return Some(wait);
        }
        check.pending = Some((name.to_string(), backend.user_answered(name)));
        None
    }

    /// A warning for the participant if the typed name already filled in a
    /// survey.
    pub fn name_warning(&self) -> Option<String> {
        let name = self.user_name.trim();
        self.name_check
            .answer
            .as_ref()
            .is_some_and(|(asked, answered)| *answered && asked == name)
            .then(|| format!("\"{name}\" already filled in a survey. If that was not you, please use another name."))
    }

    fn start(&mut self, survey_id: u64, backend: &impl Backend) {
        let user = self.user_name.trim();
        if let Err(err) = prono_api::check_user_name(user) {
            self.error = Some(err.to_string());
        } else if let Some(survey) = prono_api::survey(survey_id) {
            // Everyone's answers are public, so a returning participant's
            // are found among them: no endpoint of their own is needed.
            self.refill = Some(Refill {
                user: user.to_string(),
                results: backend.results(survey_id),
            });
            self.error = None;
            self.phase = Phase::Answering {
                survey_id,
                answers: survey.empty_answers(),
            };
        }
    }

    fn submit(&mut self, backend: &impl Backend) {
        let (Phase::Answering { survey_id, answers }, Some(survey)) = (&self.phase, self.survey()) else {
            return;
        };
        if self.is_busy() {
            return;
        }
        let submission = Submission::new(&self.user_name, survey, answers);
        match submission.validate(survey) {
            Ok(()) => {
                self.error = None;
                self.request = Some(Request::Submit(backend.submit(*survey_id, &submission)));
            }
            Err(err) => self.error = Some(err.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use prono_api::UserAnswer;

    use super::*;

    const SURVEY_ID: u64 = 1;

    /// Answers requests immediately with `submit_result`, or never if `None`.
    struct FakeBackend {
        submit_result: Option<Result<(), String>>,
        submissions: RefCell<Vec<Submission>>,
        results_requests: Cell<usize>,
        /// What `user_answered` returns, or never answers if `None`.
        name_answer: Option<Result<bool, String>>,
        name_requests: RefCell<Vec<String>>,
        /// What `results` returns, or never answers if `None`.
        results_answer: Option<Result<Results, String>>,
    }

    impl FakeBackend {
        fn new(submit_result: Option<Result<(), String>>) -> Self {
            Self {
                submit_result,
                submissions: RefCell::default(),
                results_requests: Cell::default(),
                name_answer: Some(Ok(false)),
                name_requests: RefCell::default(),
                results_answer: Some(Ok(Results::new())),
            }
        }

        fn with_results(mut self, answer: Option<Result<Results, String>>) -> Self {
            self.results_answer = answer;
            self
        }

        fn with_name_answer(mut self, answer: Option<Result<bool, String>>) -> Self {
            self.name_answer = answer;
            self
        }
    }

    impl Backend for FakeBackend {
        fn submit(&self, _survey_id: u64, submission: &Submission) -> Pending<()> {
            self.submissions.borrow_mut().push(submission.clone());
            self.submit_result.clone().map_or_else(Pending::never, Pending::ready)
        }

        fn results(&self, _survey_id: u64) -> Pending<Results> {
            self.results_requests.set(self.results_requests.get() + 1);
            self.results_answer.clone().map_or_else(Pending::never, Pending::ready)
        }

        fn user_answered(&self, user: &str) -> Pending<bool> {
            self.name_requests.borrow_mut().push(user.into());
            self.name_answer.clone().map_or_else(Pending::never, Pending::ready)
        }
    }

    fn answering(user: &str) -> State {
        let mut state = State {
            user_name: user.into(),
            ..State::default()
        };
        state.apply(Action::StartSurvey(SURVEY_ID), &FakeBackend::new(None));
        state
    }

    #[test]
    fn starting_requires_a_name() {
        let state = answering("  ");
        assert!(matches!(state.phase, Phase::Start));
        assert!(state.error.is_some());
    }

    #[test]
    fn starting_shows_blank_answers() {
        let state = answering("Sam");
        let Phase::Answering { survey_id, answers } = &state.phase else {
            panic!("not answering");
        };
        assert_eq!(*survey_id, SURVEY_ID);
        assert_eq!(*answers, prono_api::survey(SURVEY_ID).unwrap().empty_answers());
    }

    #[test]
    fn reset_restores_blank_answers() {
        let mut state = answering("Sam");
        if let Phase::Answering { answers, .. } = &mut state.phase {
            answers[0] = Answer::PredictionDate {
                day: None,
                month: 1,
                year: 2099,
            };
        }
        state.apply(Action::ResetAnswers, &FakeBackend::new(None));
        let Phase::Answering { answers, .. } = &state.phase else {
            panic!("not answering");
        };
        assert_eq!(*answers, prono_api::survey(SURVEY_ID).unwrap().empty_answers());
    }

    #[test]
    fn successful_submit_shows_results() {
        let backend = FakeBackend::new(Some(Ok(())));
        let mut state = answering(" Sam ");
        state.apply(Action::Submit, &backend);
        assert!(state.is_busy());
        assert_eq!(backend.submissions.borrow()[0].user, "Sam");

        state.poll(&backend); // submit done, results requested
        assert!(matches!(state.phase, Phase::Done { survey_id: SURVEY_ID }));
        state.poll(&backend); // results arrived
        assert_eq!(state.results, Some(Results::new()));
        assert!(!state.is_busy());

        state.poll(&backend);
        assert_eq!(backend.results_requests.get(), 1);
    }

    #[test]
    fn failed_submit_keeps_answers_and_shows_error() {
        let backend = FakeBackend::new(Some(Err("You already answered this survey".into())));
        let mut state = answering("Sam");
        state.apply(Action::Submit, &backend);
        state.poll(&backend);

        assert!(matches!(state.phase, Phase::Answering { .. }));
        assert_eq!(state.error.as_deref(), Some("You already answered this survey"));
        state.apply(Action::DismissError, &backend);
        assert!(state.error.is_none());
    }

    #[test]
    fn submit_is_ignored_while_busy() {
        let backend = FakeBackend::new(None);
        let mut state = answering("Sam");
        state.apply(Action::Submit, &backend);
        state.apply(Action::Submit, &backend);
        state.poll(&backend);
        assert_eq!(backend.submissions.borrow().len(), 1);
        assert!(state.is_busy());
    }

    #[test]
    fn choosing_another_survey_returns_to_start() {
        let mut state = State {
            user_name: "Sam".into(),
            phase: Phase::Done { survey_id: SURVEY_ID },
            results: Some(Results::new()),
            ..State::default()
        };
        state.apply(Action::ChooseAnotherSurvey, &FakeBackend::new(None));
        assert!(matches!(state.phase, Phase::Start));
        assert!(state.results.is_none());
        assert_eq!(state.user_name, "Sam");
    }

    #[test]
    fn restored_state_must_fit_the_catalog() {
        let unknown = State {
            phase: Phase::Done { survey_id: 999 },
            ..State::default()
        };
        assert!(matches!(unknown.restored().phase, Phase::Start));

        let stale_answers = State {
            phase: Phase::Answering {
                survey_id: SURVEY_ID,
                answers: vec![Answer::Text("old".into())],
            },
            ..State::default()
        };
        assert!(matches!(stale_answers.restored().phase, Phase::Start));

        let valid = answering("Sam").restored();
        assert!(matches!(valid.phase, Phase::Answering { .. }));
    }

    fn typing(name: &str) -> State {
        State {
            user_name: name.into(),
            ..State::default()
        }
    }

    #[test]
    fn name_is_checked_once_typing_pauses() {
        let backend = FakeBackend::new(None).with_name_answer(Some(Ok(true)));
        let mut state = typing("Sam");

        let wait = state.check_name(&backend, 10.0);
        assert_eq!(wait, Some(NAME_CHECK_DELAY));
        assert!(backend.name_requests.borrow().is_empty());

        // Still typing: the name changes, the delay starts over.
        state.user_name.push('!');
        assert!(state.check_name(&backend, 10.0 + NAME_CHECK_DELAY / 2.0).is_some());
        assert!(backend.name_requests.borrow().is_empty());

        // Typing paused.
        assert_eq!(state.check_name(&backend, 10.0 + NAME_CHECK_DELAY * 2.0), None);
        assert_eq!(*backend.name_requests.borrow(), ["Sam!"]);
    }

    #[test]
    fn unchanged_name_is_not_asked_again() {
        // Moving the cursor or clicking around does not change the name.
        let backend = FakeBackend::new(None);
        let mut state = typing(" Sam ");
        state.check_name(&backend, 0.0);
        for frame in 1..=20 {
            state.check_name(&backend, f64::from(frame));
        }
        assert_eq!(*backend.name_requests.borrow(), ["Sam"]);
    }

    #[test]
    fn warns_while_the_name_is_taken() {
        let backend = FakeBackend::new(None).with_name_answer(Some(Ok(true)));
        let mut state = typing("Sam");
        assert!(state.name_warning().is_none());

        state.check_name(&backend, 0.0);
        state.check_name(&backend, 1.0); // asks
        assert!(state.name_warning().is_none(), "no answer yet");
        state.check_name(&backend, 1.1); // answer in
        assert!(state.name_warning().is_some_and(|warning| warning.contains("already")));

        state.user_name = "Sam2".into();
        assert!(state.name_warning().is_none());
        state.user_name = " Sam ".into();
        assert!(state.name_warning().is_some());
    }

    #[test]
    fn no_warning_for_a_new_name() {
        let backend = FakeBackend::new(None).with_name_answer(Some(Ok(false)));
        let mut state = typing("Sam");
        state.check_name(&backend, 0.0);
        state.check_name(&backend, 1.0);
        state.check_name(&backend, 1.1);
        assert!(state.name_warning().is_none());
        assert!(state.error.is_none());
    }

    #[test]
    fn failed_check_is_ignored() {
        // It is only a hint: a real problem shows when submitting.
        let backend = FakeBackend::new(None).with_name_answer(Some(Err("Cannot reach the server".into())));
        let mut state = typing("Sam");
        state.check_name(&backend, 0.0);
        state.check_name(&backend, 1.0);
        state.check_name(&backend, 1.1);
        assert!(state.name_warning().is_none());
        assert!(state.error.is_none());

        // Not asked again every frame while the name stays the same.
        for frame in 2..10 {
            state.check_name(&backend, f64::from(frame));
        }
        assert_eq!(backend.name_requests.borrow().len(), 1);
    }

    #[test]
    fn invalid_names_are_not_asked() {
        let backend = FakeBackend::new(None);
        let mut state = typing("  ");
        state.check_name(&backend, 0.0);
        assert_eq!(state.check_name(&backend, 5.0), None);
        state.user_name = "x".repeat(prono_api::MAX_USER_NAME_LEN + 1);
        state.check_name(&backend, 6.0);
        assert_eq!(state.check_name(&backend, 11.0), None);
        assert!(backend.name_requests.borrow().is_empty());
    }

    #[test]
    fn answer_of_an_older_name_is_dropped() {
        let backend = FakeBackend::new(None).with_name_answer(None);
        let mut state = typing("Sam");
        state.check_name(&backend, 0.0);
        state.check_name(&backend, 1.0); // asks about "Sam", never answered
        state.user_name = "Tom".into();
        state.check_name(&backend, 2.0);
        state.check_name(&backend, 3.0); // asks about "Tom"
        assert_eq!(*backend.name_requests.borrow(), ["Sam", "Tom"]);
    }

    #[test]
    fn name_is_only_checked_on_the_start_screen() {
        let backend = FakeBackend::new(None);
        let mut state = answering("Sam");
        state.check_name(&backend, 0.0);
        assert_eq!(state.check_name(&backend, 5.0), None);
        assert!(backend.name_requests.borrow().is_empty());
    }

    #[test]
    fn name_is_asked_again_after_submitting() {
        let backend = FakeBackend::new(Some(Ok(())));
        let mut state = typing("Sam");
        state.check_name(&backend, 0.0);
        state.check_name(&backend, 1.0);
        state.check_name(&backend, 1.1);
        assert_eq!(backend.name_requests.borrow().len(), 1);

        state.apply(Action::StartSurvey(SURVEY_ID), &backend);
        state.apply(Action::Submit, &backend);
        state.poll(&backend); // submitted
        state.apply(Action::ChooseAnotherSurvey, &backend);

        state.check_name(&backend, 2.0);
        assert_eq!(
            backend.name_requests.borrow().len(),
            2,
            "the answer changed with the submission"
        );
    }

    fn date(month: u8, year: u16) -> Answer {
        Answer::PredictionDate { day: None, month, year }
    }

    /// Everyone's results where `user` predicted `month`/`year` everywhere.
    fn results_of(user: &str, month: u8, year: u16) -> Results {
        prono_api::survey(SURVEY_ID)
            .unwrap()
            .questions
            .iter()
            .map(|question| {
                let answer = UserAnswer {
                    user: user.into(),
                    answer: date(month, year),
                };
                (question.id, vec![answer])
            })
            .collect()
    }

    fn answers_of(state: &State) -> &[Answer] {
        match &state.phase {
            Phase::Answering { answers, .. } => answers,
            _ => panic!("not answering"),
        }
    }

    fn started(user: &str, backend: &FakeBackend) -> State {
        let mut state = typing(user);
        state.apply(Action::StartSurvey(SURVEY_ID), backend);
        state
    }

    #[test]
    fn earlier_answers_refill_the_questions() {
        let backend = FakeBackend::new(None).with_results(Some(Ok(results_of("Sam", 4, 2033))));
        let mut state = started(" Sam ", &backend);
        assert_eq!(
            answers_of(&state),
            prono_api::survey(SURVEY_ID).unwrap().empty_answers()
        );

        state.poll(&backend);
        assert!(answers_of(&state).iter().all(|answer| *answer == date(4, 2033)));
        assert!(!state.is_busy(), "looking them up never keeps the participant waiting");
        assert!(state.error.is_none());
    }

    #[test]
    fn answers_of_other_names_do_not_refill() {
        let backend = FakeBackend::new(None).with_results(Some(Ok(results_of("Bob", 4, 2033))));
        let mut state = started("Sam", &backend);
        state.poll(&backend);
        assert_eq!(
            answers_of(&state),
            prono_api::survey(SURVEY_ID).unwrap().empty_answers()
        );
    }

    #[test]
    fn refill_waits_for_the_answer_and_happens_once() {
        let backend = FakeBackend::new(None).with_results(None);
        let mut state = started("Sam", &backend);
        state.poll(&backend);
        state.poll(&backend);
        assert_eq!(backend.results_requests.get(), 1);

        // Whatever the participant answered meanwhile stays.
        let backend = FakeBackend::new(None).with_results(Some(Ok(results_of("Sam", 4, 2033))));
        let mut state = started("Sam", &backend);
        if let Phase::Answering { answers, .. } = &mut state.phase {
            answers[0] = date(1, 2099);
        }
        state.poll(&backend);
        assert_eq!(answers_of(&state)[0], date(1, 2099));
        assert_ne!(answers_of(&state)[1], date(4, 2033));
    }

    #[test]
    fn failed_refill_is_ignored() {
        let backend = FakeBackend::new(None).with_results(Some(Err("Cannot reach the server".into())));
        let mut state = started("Sam", &backend);
        state.poll(&backend);
        assert_eq!(
            answers_of(&state),
            prono_api::survey(SURVEY_ID).unwrap().empty_answers()
        );
        assert!(state.error.is_none());
    }

    #[test]
    fn refill_does_not_outlive_the_questions() {
        let backend = FakeBackend::new(None).with_results(Some(Ok(results_of("Sam", 4, 2033))));
        let mut state = started("Sam", &backend);
        state.apply(Action::ChangeName, &backend);
        state.poll(&backend);
        assert!(matches!(state.phase, Phase::Start));
        state.apply(Action::StartSurvey(SURVEY_ID), &backend);
        state.apply(Action::ChooseAnotherSurvey, &backend);
        state.poll(&backend);
        assert!(matches!(state.phase, Phase::Start));
    }

    #[test]
    fn changing_the_name_goes_back_to_the_start() {
        let backend = FakeBackend::new(Some(Err("This name is registered on another device".into())));
        let mut state = answering("Sam");
        state.apply(Action::Submit, &backend);
        state.poll(&backend);
        assert!(state.error.is_some());

        state.apply(Action::ChangeName, &backend);
        assert!(matches!(state.phase, Phase::Start));
        assert!(state.error.is_none());
        assert_eq!(state.user_name, "Sam");
    }

    #[test]
    fn changing_the_name_is_ignored_while_submitting() {
        let backend = FakeBackend::new(None);
        let mut state = answering("Sam");
        state.apply(Action::Submit, &backend);
        state.apply(Action::ChangeName, &backend);
        assert!(matches!(state.phase, Phase::Answering { .. }));
        assert!(state.is_busy());
    }
}
