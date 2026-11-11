use prono_api::{Access, Answer, Invalid, Results, Score, Submission, Survey, UserStatus};
use serde::{Deserialize, Serialize};

use crate::client::{Backend, Pending};

/// Seconds the name must stay unchanged before the backend is asked about it.
pub const NAME_CHECK_DELAY: f64 = 0.5;

/// Whether answers may be reset. Release builds, which real participants use,
/// never allow it: a filled-in prono stays filled in. Debug builds keep it for
/// development.
pub const CAN_RESET: bool = cfg!(debug_assertions);

/// What the participant is doing.
#[derive(Default, Serialize, Deserialize)]
pub enum Phase {
    /// Entering a name and picking a survey.
    #[default]
    Start,
    /// Filling in `survey`; `answers` follow its question order.
    Answering { survey: Survey, answers: Vec<Answer> },
    /// Submitted `survey`; showing everyone's answers.
    Done { survey: Survey },
}

/// Something the participant asked for in the UI.
#[derive(Clone, Copy)]
pub enum Action {
    StartSurvey(u64),
    ResetAnswers,
    Submit,
    ChooseAnotherSurvey,
    /// Back from the questions to the name fields, e.g. after the name was
    /// refused.
    ChangeName,
    DismissError,
}

/// The surveys the start screen offers.
pub enum SurveyList<'a> {
    /// There is no valid first and last name yet.
    NeedName,
    /// The backend has not told yet which surveys the name may answer.
    Waiting,
    /// The backend could not tell; the error is in [`State::error`]. Changing
    /// the name asks again.
    Failed,
    /// The surveys the name may answer, by id.
    Ready(&'a [Survey]),
}

/// The app's state and flow, without any drawing: the UI shows it, edits the
/// answers in place, and turns clicks into [`Action`]s.
#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub struct State {
    pub first_name: String,
    pub last_name: String,
    /// For a survey with [`Access::PassPhrase`]. A secret: never persisted.
    #[serde(skip)]
    pub pass_phrase: String,
    pub phase: Phase,
    #[serde(skip)]
    pub results: Option<Results>,
    /// The scores of a scored survey, once fetched.
    #[serde(skip)]
    pub scores: Option<Vec<Score>>,
    /// Shown to the participant until dismissed.
    #[serde(skip)]
    pub error: Option<String>,
    #[serde(skip)]
    request: Option<Request>,
    /// The earlier answers of a returning participant, being fetched. Apart
    /// from `request` because it must not keep the participant from answering.
    #[serde(skip)]
    earlier: Option<Earlier>,
    #[serde(skip)]
    name_check: NameCheck,
}

/// Looking up whether `user` answered a survey before.
struct Earlier {
    user: String,
    results: Pending<Results>,
}

enum Request {
    /// Waiting for the backend to accept the pass phrase before `survey`
    /// opens for `user`.
    Opening {
        survey: Survey,
        user: String,
        pending: Pending<()>,
    },
    Submit(Pending<()>),
    Results(Pending<Results>),
    Scores(Pending<Vec<Score>>),
}

/// Finding out, while the name is typed, what the backend knows about it:
/// whether it already filled in a survey, and which surveys it may answer.
#[derive(Default)]
struct NameCheck {
    /// The full name as last seen, and when it changed (in seconds).
    typed: String,
    changed_at: f64,
    /// The name being asked about, and the answer to come.
    pending: Option<(String, Pending<UserStatus>)>,
    /// The last answer: a name and what the backend said about it, `None` if
    /// the lookup failed.
    answer: Option<(String, Option<UserStatus>)>,
}

impl State {
    /// Drops restored state whose answers do not fit their survey. A survey
    /// that changed on the backend since is only found out when submitting.
    #[must_use]
    pub fn restored(mut self) -> Self {
        let fits = match &self.phase {
            Phase::Start | Phase::Done { .. } => true,
            Phase::Answering { survey, answers } => {
                survey.questions.len() == answers.len()
                    && survey.questions.iter().zip(answers).all(|(q, a)| q.kind == a.kind())
            }
        };
        if !fits {
            self.phase = Phase::Start;
        }
        self
    }

    /// The survey being answered or shown, if any.
    pub fn survey(&self) -> Option<&Survey> {
        match &self.phase {
            Phase::Start => None,
            Phase::Answering { survey, .. } | Phase::Done { survey } => Some(survey),
        }
    }

    /// The participant's name as it is stored, "First Last".
    ///
    /// # Errors
    ///
    /// [`Invalid::UserName`] unless both parts are filled in.
    pub fn full_name(&self) -> Result<String, Invalid> {
        prono_api::check_name_parts(&self.first_name, &self.last_name)
    }

    /// Whether a request is in flight.
    pub fn is_busy(&self) -> bool {
        self.request.is_some()
    }

    pub fn apply(&mut self, action: Action, backend: &impl Backend) {
        match action {
            Action::StartSurvey(survey_id) => self.start(survey_id, backend),
            Action::ResetAnswers => {
                if CAN_RESET && let Phase::Answering { survey, answers } = &mut self.phase {
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
        self.scores = None;
        self.request = None;
        self.earlier = None;
        self.pass_phrase.clear();
    }

    /// Takes in finished requests, and fetches results when they are
    /// missing. Call it every frame.
    pub fn poll(&mut self, backend: &impl Backend) {
        self.poll_earlier();
        match self.request.take() {
            Some(Request::Opening { survey, user, pending }) => match pending.take() {
                None => self.request = Some(Request::Opening { survey, user, pending }),
                Some(Ok(())) => self.open(survey, user, backend),
                Some(Err(err)) => self.error = Some(err),
            },
            Some(Request::Submit(pending)) => match pending.take() {
                None => self.request = Some(Request::Submit(pending)),
                Some(Ok(())) => match std::mem::take(&mut self.phase) {
                    Phase::Answering { survey, .. } => {
                        self.phase = Phase::Done { survey };
                        self.results = None;
                        self.scores = None;
                        self.mark_answered();
                    }
                    other => self.phase = other,
                },
                Some(Err(err)) => self.error = Some(err),
            },
            Some(Request::Results(pending)) => match pending.take() {
                None => self.request = Some(Request::Results(pending)),
                Some(Ok(results)) => self.results = Some(results),
                Some(Err(err)) => self.error = Some(err),
            },
            Some(Request::Scores(pending)) => match pending.take() {
                None => self.request = Some(Request::Scores(pending)),
                Some(Ok(scores)) => self.scores = Some(scores),
                Some(Err(err)) => self.error = Some(err),
            },
            None => {}
        }

        // After an error, wait until it is dismissed before retrying. The
        // scores of a scored survey follow the results.
        if let Phase::Done { survey } = &self.phase
            && self.request.is_none()
            && self.error.is_none()
        {
            if self.results.is_none() {
                self.request = Some(Request::Results(backend.results(survey.id)));
            } else if survey.scored && self.scores.is_none() {
                self.request = Some(Request::Scores(backend.scores(survey.id)));
            }
        }
    }

    /// The name has just filled in a survey: what is known about it changes,
    /// but not the surveys it is offered.
    fn mark_answered(&mut self) {
        let name = prono_api::full_name(&self.first_name, &self.last_name);
        if let Some((asked, Some(status))) = &mut self.name_check.answer
            && *asked == name
        {
            status.answered = true;
        }
    }

    /// Shows everyone's answers instead of the questions once it turns out
    /// the participant answered the survey before: answers are final. The
    /// lookup is a convenience: when it fails or finds nothing, the
    /// participant just answers (the backend refuses a second submission
    /// anyway).
    fn poll_earlier(&mut self) {
        let Some(earlier) = self.earlier.take() else { return };
        let Some(found) = earlier.results.take() else {
            self.earlier = Some(earlier);
            return;
        };
        let (Phase::Answering { survey, .. }, Ok(results)) = (&self.phase, found) else {
            return;
        };
        if survey.answers_of(&results, &earlier.user).is_some() {
            self.phase = Phase::Done { survey: survey.clone() };
            self.results = Some(results);
            self.pass_phrase.clear();
        }
    }

    /// Asks the backend what it knows about the typed name (whether it
    /// already filled in a survey, which surveys it may answer), once the
    /// name stayed unchanged for [`NAME_CHECK_DELAY`] (`now` is in seconds).
    /// It only asks for a changed, valid name, so moving the cursor or
    /// clicking around costs nothing. A failed lookup is reported in
    /// [`State::error`] and not repeated until the name changes. Call it
    /// every frame.
    ///
    /// Returns how many seconds to wait before it must be called again, if
    /// it is waiting for the typing to pause.
    pub fn check_name(&mut self, backend: &impl Backend, now: f64) -> Option<f64> {
        if let Some((name, pending)) = self.name_check.pending.take() {
            match pending.take() {
                None => self.name_check.pending = Some((name, pending)),
                Some(Ok(status)) => self.name_check.answer = Some((name, Some(status))),
                Some(Err(err)) => {
                    self.name_check.answer = Some((name, None));
                    self.error = Some(err);
                }
            }
        }
        if !matches!(self.phase, Phase::Start) {
            return None;
        }

        let name = prono_api::full_name(&self.first_name, &self.last_name);
        let valid = self.full_name().is_ok();
        let check = &mut self.name_check;
        if name != check.typed {
            check.typed.clone_from(&name);
            check.changed_at = now;
        }
        let known = |asked: &String| *asked == name;
        if !valid
            || check.answer.as_ref().is_some_and(|(asked, _)| known(asked))
            || check.pending.as_ref().is_some_and(|(asked, _)| known(asked))
        {
            return None;
        }

        let wait = NAME_CHECK_DELAY - (now - check.changed_at);
        if wait > 0.0 {
            return Some(wait);
        }
        let status = backend.user_status(&name);
        check.pending = Some((name, status));
        None
    }

    /// What the backend said about the typed name, if it has by now.
    fn status(&self) -> Option<&UserStatus> {
        let name = self.full_name().ok()?;
        match &self.name_check.answer {
            Some((asked, Some(status))) if *asked == name => Some(status),
            _ => None,
        }
    }

    /// A warning for the participant if the typed name already filled in a
    /// survey.
    pub fn name_warning(&self) -> Option<String> {
        let name = prono_api::full_name(&self.first_name, &self.last_name);
        self.status()
            .is_some_and(|status| status.answered)
            .then(|| format!("\"{name}\" already filled in a survey. If that was not you, please use another name."))
    }

    /// The surveys the start screen offers: none before there is a valid
    /// name the backend answered for.
    pub fn survey_list(&self) -> SurveyList<'_> {
        let Ok(name) = self.full_name() else {
            return SurveyList::NeedName;
        };
        match &self.name_check.answer {
            Some((asked, Some(status))) if *asked == name => SurveyList::Ready(&status.surveys),
            Some((asked, None)) if *asked == name => SurveyList::Failed,
            _ => SurveyList::Waiting,
        }
    }

    /// The surveys the typed name may answer, by id; `None` while that is
    /// not known.
    pub fn available_surveys(&self) -> Option<&[Survey]> {
        match self.survey_list() {
            SurveyList::Ready(surveys) => Some(surveys),
            _ => None,
        }
    }

    fn start(&mut self, survey_id: u64, backend: &impl Backend) {
        if self.is_busy() {
            return;
        }
        let user = match self.full_name() {
            Ok(user) => user,
            Err(err) => {
                self.error = Some(err.to_string());
                return;
            }
        };
        let survey = self
            .available_surveys()
            .and_then(|surveys| surveys.iter().find(|survey| survey.id == survey_id))
            .cloned();
        let Some(survey) = survey else {
            self.error = Some("This survey is not available for you".into());
            return;
        };
        self.error = None;
        if survey.access == Access::PassPhrase {
            // A wrong pass phrase is refused here, before the questions open.
            let pass_phrase = prono_api::clean_pass_phrase(&self.pass_phrase);
            self.request = Some(Request::Opening {
                pending: backend.check_pass_phrase(survey_id, pass_phrase.as_deref()),
                survey,
                user,
            });
        } else {
            self.open(survey, user, backend);
        }
    }

    /// Shows the questions of `survey` to `user`.
    fn open(&mut self, survey: Survey, user: String, backend: &impl Backend) {
        // Everyone's answers are public, so a returning participant's are
        // found among them: no endpoint of their own is needed.
        self.earlier = Some(Earlier {
            user,
            results: backend.results(survey.id),
        });
        self.phase = Phase::Answering {
            answers: survey.empty_answers(),
            survey,
        };
    }

    fn submit(&mut self, backend: &impl Backend) {
        let Phase::Answering { survey, answers } = &self.phase else {
            return;
        };
        if self.is_busy() {
            return;
        }
        let user = match self.full_name() {
            Ok(user) => user,
            Err(err) => {
                self.error = Some(err.to_string());
                return;
            }
        };
        let submission = Self::submission(&user, &self.pass_phrase, survey, answers);
        match submission.validate(survey) {
            Ok(()) => {
                self.error = None;
                self.request = Some(Request::Submit(backend.submit(survey.id, &submission)));
            }
            Err(err) => self.error = Some(err.to_string()),
        }
    }

    /// The answers of `user` to `survey`, with the pass phrase if the survey
    /// asks for one.
    fn submission(user: &str, pass_phrase: &str, survey: &Survey, answers: &[Answer]) -> Submission {
        let submission = Submission::new(user, survey, answers);
        if survey.access == Access::PassPhrase {
            submission.with_pass_phrase(pass_phrase)
        } else {
            submission
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use prono_api::{AnswerKind, Question, UserAnswer, samples};

    use super::*;

    /// The survey with a pass phrase.
    const SURVEY_ID: u64 = 1;
    /// The survey that only a name list may answer.
    const LISTED_ID: u64 = 2;

    /// The sample survey `id`.
    fn sample(id: u64) -> Survey {
        match id {
            SURVEY_ID => samples::starship(),
            LISTED_ID => samples::course(),
            _ => panic!("no sample survey {id}"),
        }
    }

    /// Answers requests immediately with `submit_result`, or never if `None`.
    struct FakeBackend {
        submit_result: Option<Result<(), String>>,
        submissions: RefCell<Vec<Submission>>,
        /// What `check_pass_phrase` says, or never answers if `None`.
        pass_phrase_answer: Option<Result<(), String>>,
        /// The pass phrase of each check, in order.
        pass_phrase_checks: RefCell<Vec<Option<String>>>,
        results_requests: Cell<usize>,
        /// What `scores` returns, or never answers if `None`.
        scores_answer: Option<Result<Vec<Score>, String>>,
        scores_requests: Cell<usize>,
        /// What `user_status` returns, or never answers if `None`.
        name_answer: Option<Result<UserStatus, String>>,
        name_requests: RefCell<Vec<String>>,
        /// What `results` returns, or never answers if `None`.
        results_answer: Option<Result<Results, String>>,
    }

    /// A status offering the sample surveys with the given ids.
    fn status(answered: bool, surveys: &[u64]) -> UserStatus {
        UserStatus {
            answered,
            surveys: surveys.iter().map(|id| sample(*id)).collect(),
        }
    }

    impl FakeBackend {
        fn new(submit_result: Option<Result<(), String>>) -> Self {
            Self {
                submit_result,
                submissions: RefCell::default(),
                pass_phrase_answer: Some(Ok(())),
                pass_phrase_checks: RefCell::default(),
                results_requests: Cell::default(),
                scores_answer: Some(Ok(Vec::new())),
                scores_requests: Cell::default(),
                name_answer: Some(Ok(status(false, &[SURVEY_ID, LISTED_ID]))),
                name_requests: RefCell::default(),
                results_answer: Some(Ok(Results::new())),
            }
        }

        fn with_pass_phrase_answer(mut self, answer: Option<Result<(), String>>) -> Self {
            self.pass_phrase_answer = answer;
            self
        }

        fn with_results(mut self, answer: Option<Result<Results, String>>) -> Self {
            self.results_answer = answer;
            self
        }

        fn with_scores(mut self, answer: Option<Result<Vec<Score>, String>>) -> Self {
            self.scores_answer = answer;
            self
        }

        fn with_name_answer(mut self, answer: Option<Result<UserStatus, String>>) -> Self {
            self.name_answer = answer;
            self
        }

        /// Everyone already filled in a survey (or nobody did).
        fn with_answered(self, answered: bool) -> Self {
            self.with_name_answer(Some(Ok(status(answered, &[SURVEY_ID, LISTED_ID]))))
        }
    }

    impl Backend for FakeBackend {
        fn submit(&self, _survey_id: u64, submission: &Submission) -> Pending<()> {
            self.submissions.borrow_mut().push(submission.clone());
            self.submit_result.clone().map_or_else(Pending::never, Pending::ready)
        }

        fn check_pass_phrase(&self, _survey_id: u64, pass_phrase: Option<&str>) -> Pending<()> {
            self.pass_phrase_checks.borrow_mut().push(pass_phrase.map(Into::into));
            self.pass_phrase_answer
                .clone()
                .map_or_else(Pending::never, Pending::ready)
        }

        fn results(&self, _survey_id: u64) -> Pending<Results> {
            self.results_requests.set(self.results_requests.get() + 1);
            self.results_answer.clone().map_or_else(Pending::never, Pending::ready)
        }

        fn scores(&self, _survey_id: u64) -> Pending<Vec<Score>> {
            self.scores_requests.set(self.scores_requests.get() + 1);
            self.scores_answer.clone().map_or_else(Pending::never, Pending::ready)
        }

        fn user_status(&self, user: &str) -> Pending<UserStatus> {
            self.name_requests.borrow_mut().push(user.into());
            self.name_answer.clone().map_or_else(Pending::never, Pending::ready)
        }
    }

    /// A state with `name` ("First Last", the last part may be missing) typed.
    fn typing(name: &str) -> State {
        let (first, last) = name.trim().split_once(' ').unwrap_or((name, ""));
        State {
            first_name: first.into(),
            last_name: last.into(),
            ..State::default()
        }
    }

    /// Lets the typing of `state` pause from `from` on, until the backend's
    /// answer is in.
    fn look_up(state: &mut State, backend: &FakeBackend, from: f64) {
        state.check_name(backend, from);
        state.check_name(backend, from + 1.0); // asks
        state.check_name(backend, from + 1.1); // answer in
    }

    /// A state whose typed name the backend answered for.
    fn known(name: &str, backend: &FakeBackend) -> State {
        let mut state = typing(name);
        look_up(&mut state, backend, 0.0);
        state
    }

    /// Starts survey `survey_id`, and takes in the backend's answer about the
    /// pass phrase if the survey has one.
    fn start(state: &mut State, survey_id: u64, backend: &FakeBackend) {
        state.apply(Action::StartSurvey(survey_id), backend);
        if state.is_busy() {
            state.poll(backend);
        }
    }

    fn answering(name: &str) -> State {
        let backend = FakeBackend::new(None);
        let mut state = known(name, &backend);
        start(&mut state, SURVEY_ID, &backend);
        state
    }

    fn ids(surveys: Option<&[Survey]>) -> Option<Vec<u64>> {
        surveys.map(|surveys| surveys.iter().map(|survey| survey.id).collect())
    }

    #[test]
    fn full_name_needs_a_first_and_a_last_name() {
        let padded = State {
            first_name: "  Sam ".into(),
            last_name: " Smith  ".into(),
            ..State::default()
        };
        assert_eq!(padded.full_name().as_deref(), Ok("Sam Smith"));
        assert_eq!(typing("Sam").full_name(), Err(Invalid::UserName));
        assert_eq!(typing("  Smith").full_name(), Err(Invalid::UserName));
        assert_eq!(typing("   ").full_name(), Err(Invalid::UserName));
        let long = "a".repeat(prono_api::MAX_USER_NAME_LEN);
        assert_eq!(typing(&format!("{long} b")).full_name(), Err(Invalid::UserName));
        assert!(typing(&format!("{} b", &long[2..])).full_name().is_ok());
    }

    #[test]
    fn starting_requires_a_full_name() {
        for name in ["Sam", "  "] {
            let mut state = typing(name);
            state.apply(Action::StartSurvey(SURVEY_ID), &FakeBackend::new(None));
            assert!(matches!(state.phase, Phase::Start));
            assert_eq!(
                state.error.as_deref(),
                Some("Please fill in your first and last name (at most 64 characters)")
            );
        }
    }

    #[test]
    fn starting_shows_blank_answers() {
        let state = answering("Sam Smith");
        let Phase::Answering { survey, answers } = &state.phase else {
            panic!("not answering");
        };
        assert_eq!(survey.id, SURVEY_ID);
        assert_eq!(*answers, sample(SURVEY_ID).empty_answers());
    }

    #[test]
    fn a_survey_with_a_pass_phrase_opens_once_the_backend_accepted_it() {
        let backend = FakeBackend::new(None).with_pass_phrase_answer(None);
        let mut state = known("Sam Smith", &backend);
        state.pass_phrase = "  open sesame ".into();

        start(&mut state, SURVEY_ID, &backend);
        assert!(matches!(state.phase, Phase::Start), "still waiting for the backend");
        assert!(state.is_busy());
        assert_eq!(*backend.pass_phrase_checks.borrow(), [Some("open sesame".to_string())]);
        assert_eq!(backend.results_requests.get(), 0);

        // Nothing else starts while the backend is asked.
        start(&mut state, LISTED_ID, &backend);
        assert_eq!(backend.pass_phrase_checks.borrow().len(), 1);

        let backend = FakeBackend::new(None);
        let mut state = known("Sam Smith", &backend);
        start(&mut state, SURVEY_ID, &backend);
        assert!(matches!(state.phase, Phase::Answering { .. }));
        assert!(!state.is_busy());
        assert!(state.error.is_none());
    }

    #[test]
    fn a_refused_pass_phrase_keeps_the_survey_closed_and_shows_the_error() {
        let backend = FakeBackend::new(None).with_pass_phrase_answer(Some(Err("Wrong pass phrase".into())));
        for typed in ["wrong", "", "   "] {
            let mut state = known("Sam Smith", &backend);
            state.pass_phrase = typed.into();
            start(&mut state, SURVEY_ID, &backend);
            assert!(matches!(state.phase, Phase::Start), "opened with \"{typed}\"");
            assert!(!state.is_busy());
            assert_eq!(state.error.as_deref(), Some("Wrong pass phrase"));
        }
        assert_eq!(
            *backend.pass_phrase_checks.borrow(),
            [Some("wrong".to_string()), None, None],
            "a blank phrase is sent as none"
        );
        assert_eq!(backend.results_requests.get(), 0);
    }

    #[test]
    fn a_survey_for_a_name_list_asks_for_no_pass_phrase() {
        let backend = FakeBackend::new(None);
        let mut state = known("Sam Smith", &backend);
        start(&mut state, LISTED_ID, &backend);
        assert!(matches!(state.phase, Phase::Answering { .. }));
        assert!(backend.pass_phrase_checks.borrow().is_empty());
    }

    #[test]
    fn surveys_are_unknown_until_the_backend_answered() {
        let backend = FakeBackend::new(None).with_name_answer(None);
        let mut state = typing("Sam");
        assert!(matches!(state.survey_list(), SurveyList::NeedName));
        assert_eq!(ids(state.available_surveys()), None);

        state.last_name = "Smith".into();
        assert!(matches!(state.survey_list(), SurveyList::Waiting));
        look_up(&mut state, &backend, 0.0);
        assert!(matches!(state.survey_list(), SurveyList::Waiting), "never answered");
        assert_eq!(ids(state.available_surveys()), None);

        // An answer for another name does not count.
        let backend = FakeBackend::new(None);
        let mut state = known("Sam Smith", &backend);
        state.first_name = "Tom".into();
        assert!(matches!(state.survey_list(), SurveyList::Waiting));
        state.last_name.clear();
        assert!(matches!(state.survey_list(), SurveyList::NeedName));
    }

    #[test]
    fn only_the_surveys_the_backend_names_are_available() {
        let all = FakeBackend::new(None);
        assert_eq!(
            ids(known("Sam Smith", &all).available_surveys()),
            Some(vec![SURVEY_ID, LISTED_ID])
        );

        let open_only = FakeBackend::new(None).with_name_answer(Some(Ok(status(false, &[SURVEY_ID]))));
        assert_eq!(
            ids(known("Sam Smith", &open_only).available_surveys()),
            Some(vec![SURVEY_ID])
        );

        let none = FakeBackend::new(None).with_name_answer(Some(Ok(status(false, &[]))));
        assert_eq!(ids(known("Sam Smith", &none).available_surveys()), Some(vec![]));
    }

    #[test]
    fn a_survey_that_is_not_available_cannot_be_started() {
        let backend = FakeBackend::new(None).with_name_answer(Some(Ok(status(false, &[SURVEY_ID]))));
        let mut state = known("Sam Smith", &backend);
        start(&mut state, LISTED_ID, &backend);
        assert!(matches!(state.phase, Phase::Start));
        assert_eq!(state.error.as_deref(), Some("This survey is not available for you"));
        assert_eq!(backend.results_requests.get(), 0);

        start(&mut state, SURVEY_ID, &backend);
        assert!(matches!(state.phase, Phase::Answering { .. }));
        assert!(state.error.is_none());
    }

    #[test]
    fn a_survey_cannot_be_started_before_the_backend_answered() {
        let backend = FakeBackend::new(None);
        let mut state = typing("Sam Smith");
        start(&mut state, SURVEY_ID, &backend);
        assert!(matches!(state.phase, Phase::Start));
        assert!(state.error.is_some());

        // The answer is for the old name once the name changed.
        let mut state = known("Sam Smith", &backend);
        state.last_name = "Jones".into();
        start(&mut state, SURVEY_ID, &backend);
        assert!(matches!(state.phase, Phase::Start));
        assert!(state.error.is_some());
    }

    #[test]
    fn reset_restores_blank_answers_only_where_allowed() {
        let mut state = answering("Sam Smith");
        let filled = Answer::PredictionDate {
            day: None,
            month: 1,
            year: 2099,
        };
        if let Phase::Answering { answers, .. } = &mut state.phase {
            answers[0] = filled.clone();
        }
        state.apply(Action::ResetAnswers, &FakeBackend::new(None));
        let Phase::Answering { answers, .. } = &state.phase else {
            panic!("not answering");
        };
        if CAN_RESET {
            assert_eq!(*answers, sample(SURVEY_ID).empty_answers());
        } else {
            assert_eq!(answers[0], filled);
        }
    }

    #[test]
    fn successful_submit_shows_results() {
        let backend = FakeBackend::new(Some(Ok(())));
        let mut state = answering(" Sam  Smith ");
        state.apply(Action::Submit, &backend);
        assert!(state.is_busy());
        assert_eq!(backend.submissions.borrow()[0].user, "Sam Smith");

        state.poll(&backend); // submit done, results requested
        assert!(matches!(&state.phase, Phase::Done { survey } if survey.id == SURVEY_ID));
        state.poll(&backend); // results arrived
        assert_eq!(state.results, Some(Results::new()));
        assert!(!state.is_busy());

        state.poll(&backend);
        assert_eq!(backend.results_requests.get(), 1);
    }

    #[test]
    fn failed_submit_keeps_answers_and_shows_error() {
        let backend = FakeBackend::new(Some(Err("You already answered this survey".into())));
        let mut state = answering("Sam Smith");
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
        let mut state = answering("Sam Smith");
        state.apply(Action::Submit, &backend);
        state.apply(Action::Submit, &backend);
        state.poll(&backend);
        assert_eq!(backend.submissions.borrow().len(), 1);
        assert!(state.is_busy());
    }

    #[test]
    fn submit_without_a_full_name_shows_an_error() {
        // E.g. a state restored from before the name had two parts.
        let backend = FakeBackend::new(None);
        let mut state = answering("Sam Smith");
        state.last_name.clear();
        state.apply(Action::Submit, &backend);
        assert!(state.error.is_some());
        assert!(backend.submissions.borrow().is_empty());
    }

    #[test]
    fn choosing_another_survey_returns_to_start() {
        let mut state = State {
            first_name: "Sam".into(),
            last_name: "Smith".into(),
            pass_phrase: "secret".into(),
            phase: Phase::Done {
                survey: sample(SURVEY_ID),
            },
            results: Some(Results::new()),
            ..State::default()
        };
        state.apply(Action::ChooseAnotherSurvey, &FakeBackend::new(None));
        assert!(matches!(state.phase, Phase::Start));
        assert!(state.results.is_none());
        assert_eq!((state.first_name.as_str(), state.last_name.as_str()), ("Sam", "Smith"));
        assert_eq!(state.pass_phrase, "");
    }

    #[test]
    fn restored_answers_must_fit_their_survey() {
        let stale_answers = State {
            phase: Phase::Answering {
                survey: sample(SURVEY_ID),
                answers: vec![Answer::Text("old".into())],
            },
            ..State::default()
        };
        assert!(matches!(stale_answers.restored().phase, Phase::Start));

        let wrong_kind = State {
            phase: Phase::Answering {
                survey: sample(SURVEY_ID),
                answers: vec![Answer::Text("old".into()); 4],
            },
            ..State::default()
        };
        assert!(matches!(wrong_kind.restored().phase, Phase::Start));

        let valid = answering("Sam Smith").restored();
        assert!(matches!(valid.phase, Phase::Answering { .. }));
        let done = State {
            phase: Phase::Done {
                survey: sample(SURVEY_ID),
            },
            ..State::default()
        };
        assert!(matches!(done.restored().phase, Phase::Done { .. }));
    }

    #[test]
    fn the_phase_survives_persisting_without_the_correct_answers() {
        let state = answering("Sam Smith");
        let json = serde_json::to_string(&state).unwrap();
        let restored: State = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.survey(), state.survey());
        assert!(matches!(restored.restored().phase, Phase::Answering { .. }));
    }

    #[test]
    fn state_persisted_with_the_old_single_name_still_restores() {
        use serde::de::value::{Error, MapDeserializer};

        let old = MapDeserializer::<_, Error>::new([("user_name", "Sam Smith")].into_iter());
        let restored = State::deserialize(old).unwrap();
        assert!(restored.first_name.is_empty() && restored.last_name.is_empty());
        assert!(matches!(restored.phase, Phase::Start));
        assert!(matches!(restored.survey_list(), SurveyList::NeedName));
    }

    #[test]
    fn name_is_checked_once_typing_pauses() {
        let backend = FakeBackend::new(None);
        let mut state = typing("Sam Smith");

        let wait = state.check_name(&backend, 10.0);
        assert_eq!(wait, Some(NAME_CHECK_DELAY));
        assert!(backend.name_requests.borrow().is_empty());

        // Still typing: the name changes, the delay starts over.
        state.last_name.push('!');
        assert!(state.check_name(&backend, 10.0 + NAME_CHECK_DELAY / 2.0).is_some());
        assert!(backend.name_requests.borrow().is_empty());

        // Typing paused.
        assert_eq!(state.check_name(&backend, 10.0 + NAME_CHECK_DELAY * 2.0), None);
        assert_eq!(*backend.name_requests.borrow(), ["Sam Smith!"]);
    }

    #[test]
    fn unchanged_name_is_not_asked_again() {
        // Moving the cursor or clicking around does not change the name.
        let backend = FakeBackend::new(None);
        let mut state = typing(" Sam  Smith ");
        state.check_name(&backend, 0.0);
        for frame in 1..=20 {
            state.check_name(&backend, f64::from(frame));
        }
        assert_eq!(*backend.name_requests.borrow(), ["Sam Smith"]);
    }

    #[test]
    fn warns_while_the_name_is_taken() {
        let backend = FakeBackend::new(None).with_answered(true);
        let mut state = typing("Sam Smith");
        assert!(state.name_warning().is_none());

        state.check_name(&backend, 0.0);
        state.check_name(&backend, 1.0); // asks
        assert!(state.name_warning().is_none(), "no answer yet");
        state.check_name(&backend, 1.1); // answer in
        assert!(state.name_warning().is_some_and(|warning| warning.contains("already")));

        state.last_name = "Smith2".into();
        assert!(state.name_warning().is_none());
        state.last_name = " Smith ".into();
        assert!(state.name_warning().is_some());
    }

    #[test]
    fn no_warning_for_a_new_name() {
        let backend = FakeBackend::new(None).with_answered(false);
        let state = known("Sam Smith", &backend);
        assert!(state.name_warning().is_none());
        assert!(state.error.is_none());
    }

    #[test]
    fn failed_lookup_shows_its_error_once() {
        let backend = FakeBackend::new(None).with_name_answer(Some(Err("Cannot reach the server".into())));
        let mut state = known("Sam Smith", &backend);
        assert_eq!(state.error.as_deref(), Some("Cannot reach the server"));
        assert!(state.name_warning().is_none());
        assert!(matches!(state.survey_list(), SurveyList::Failed));
        assert_eq!(ids(state.available_surveys()), None);

        // Not asked again every frame while the name stays the same, and the
        // dismissed error does not come back.
        state.apply(Action::DismissError, &backend);
        for frame in 2..10 {
            state.check_name(&backend, f64::from(frame));
        }
        assert_eq!(backend.name_requests.borrow().len(), 1);
        assert!(state.error.is_none());

        // A new name is asked about.
        state.last_name = "Jones".into();
        look_up(&mut state, &backend, 20.0);
        assert_eq!(*backend.name_requests.borrow(), ["Sam Smith", "Sam Jones"]);
    }

    #[test]
    fn invalid_names_are_not_asked() {
        let backend = FakeBackend::new(None);
        for name in ["  ", "Sam", "Smith "] {
            let mut state = typing(name);
            state.check_name(&backend, 0.0);
            assert_eq!(state.check_name(&backend, 5.0), None);
        }
        let mut state = typing(&format!("{} b", "x".repeat(prono_api::MAX_USER_NAME_LEN)));
        state.check_name(&backend, 6.0);
        assert_eq!(state.check_name(&backend, 11.0), None);
        assert!(backend.name_requests.borrow().is_empty());
    }

    #[test]
    fn answer_of_an_older_name_is_dropped() {
        let backend = FakeBackend::new(None).with_name_answer(None);
        let mut state = typing("Sam Smith");
        state.check_name(&backend, 0.0);
        state.check_name(&backend, 1.0); // asks about "Sam Smith", never answered
        state.first_name = "Tom".into();
        state.check_name(&backend, 2.0);
        state.check_name(&backend, 3.0); // asks about "Tom Smith"
        assert_eq!(*backend.name_requests.borrow(), ["Sam Smith", "Tom Smith"]);
    }

    #[test]
    fn name_is_only_checked_on_the_start_screen() {
        let backend = FakeBackend::new(None);
        let mut state = answering("Sam Smith");
        state.check_name(&backend, 0.0);
        assert_eq!(state.check_name(&backend, 5.0), None);
        assert!(backend.name_requests.borrow().is_empty());
    }

    #[test]
    fn submitting_marks_the_name_answered_and_keeps_the_survey_list() {
        let backend = FakeBackend::new(Some(Ok(())));
        let mut state = known("Sam Smith", &backend);
        assert!(state.name_warning().is_none());
        start(&mut state, SURVEY_ID, &backend);
        state.apply(Action::Submit, &backend);
        state.poll(&backend); // submitted
        state.apply(Action::ChooseAnotherSurvey, &backend);

        // No flicker: the list stays, no new request is needed.
        assert_eq!(ids(state.available_surveys()), Some(vec![SURVEY_ID, LISTED_ID]));
        assert!(state.name_warning().is_some(), "the name has answered now");
        state.check_name(&backend, 5.0);
        assert_eq!(backend.name_requests.borrow().len(), 1);
    }

    fn survey_with(access: Access) -> Survey {
        Survey {
            id: 99,
            description: "A survey".into(),
            link: None,
            access,
            scored: false,
            questions: vec![Question {
                id: uuid::Uuid::nil(),
                text: "Why".into(),
                kind: AnswerKind::Text,
                possible_answers: vec![],
                other_label: None,
                correct_answer: None,
            }],
        }
    }

    #[test]
    fn pass_phrase_is_only_sent_for_a_survey_that_asks_for_one() {
        let answers = [Answer::Text("because".into())];
        let submit = |access, pass_phrase| {
            State::submission("Sam Smith", pass_phrase, &survey_with(access), &answers).pass_phrase
        };
        assert_eq!(
            submit(Access::PassPhrase, " open sesame ").as_deref(),
            Some("open sesame")
        );
        assert_eq!(submit(Access::PassPhrase, "  "), None);
        assert_eq!(submit(Access::NameList, "open sesame"), None);
    }

    #[test]
    fn submitting_a_name_list_survey_sends_no_pass_phrase() {
        let backend = FakeBackend::new(None);
        let mut state = known("Sam Smith", &backend);
        start(&mut state, LISTED_ID, &backend);
        state.pass_phrase = "left over".into();
        if let Phase::Answering { answers, .. } = &mut state.phase {
            *answers = samples::course_answers();
        }
        state.apply(Action::Submit, &backend);
        assert_eq!(backend.submissions.borrow()[0].pass_phrase, None);
    }

    fn date(month: u8, year: u16) -> Answer {
        Answer::PredictionDate { day: None, month, year }
    }

    /// Everyone's results where `user` predicted `month`/`year` everywhere.
    fn results_of(user: &str, month: u8, year: u16) -> Results {
        sample(SURVEY_ID)
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
        let mut state = known(user, backend);
        start(&mut state, SURVEY_ID, backend);
        state
    }

    fn blank(state: &State) -> bool {
        answers_of(state) == sample(SURVEY_ID).empty_answers()
    }

    #[test]
    fn earlier_answers_lead_to_the_results() {
        let results = results_of("Sam Smith", 4, 2033);
        let backend = FakeBackend::new(None).with_results(Some(Ok(results.clone())));
        let mut state = started(" Sam  Smith ", &backend);
        assert!(blank(&state), "the questions show until the lookup is in");

        state.poll(&backend);
        assert!(matches!(&state.phase, Phase::Done { survey } if survey.id == SURVEY_ID));
        assert_eq!(state.results, Some(results));
        assert!(!state.is_busy(), "the results are already there");
        assert!(state.error.is_none());
    }

    #[test]
    fn earlier_answers_win_over_what_was_typed_meanwhile() {
        let backend = FakeBackend::new(None).with_results(Some(Ok(results_of("Sam Smith", 4, 2033))));
        let mut state = started("Sam Smith", &backend);
        if let Phase::Answering { answers, .. } = &mut state.phase {
            answers[0] = date(1, 2099);
        }
        state.poll(&backend);
        assert!(matches!(state.phase, Phase::Done { .. }));
    }

    #[test]
    fn answers_of_other_names_do_not_lead_to_the_results() {
        let backend = FakeBackend::new(None).with_results(Some(Ok(results_of("Bob Brown", 4, 2033))));
        let mut state = started("Sam Smith", &backend);
        state.poll(&backend);
        assert!(blank(&state));
    }

    #[test]
    fn lookup_waits_for_the_answer_and_happens_once() {
        let backend = FakeBackend::new(None).with_results(None);
        let mut state = started("Sam Smith", &backend);
        state.poll(&backend);
        state.poll(&backend);
        assert_eq!(backend.results_requests.get(), 1);
        assert!(blank(&state));
    }

    #[test]
    fn failed_lookup_is_ignored() {
        let backend = FakeBackend::new(None).with_results(Some(Err("Cannot reach the server".into())));
        let mut state = started("Sam Smith", &backend);
        state.poll(&backend);
        assert!(blank(&state));
        assert!(state.error.is_none());
    }

    #[test]
    fn lookup_does_not_outlive_the_questions() {
        let backend = FakeBackend::new(None).with_results(Some(Ok(results_of("Sam Smith", 4, 2033))));
        let mut state = started("Sam Smith", &backend);
        state.apply(Action::ChangeName, &backend);
        state.poll(&backend);
        assert!(matches!(state.phase, Phase::Start));
        start(&mut state, SURVEY_ID, &backend);
        state.apply(Action::ChooseAnotherSurvey, &backend);
        state.poll(&backend);
        assert!(matches!(state.phase, Phase::Start));
    }

    #[test]
    fn changing_the_name_goes_back_to_the_start() {
        let backend = FakeBackend::new(Some(Err("This name is registered on another device".into())));
        let mut state = answering("Sam Smith");
        state.pass_phrase = "secret".into();
        state.apply(Action::Submit, &backend);
        state.poll(&backend);
        assert!(state.error.is_some());

        state.apply(Action::ChangeName, &backend);
        assert!(matches!(state.phase, Phase::Start));
        assert!(state.error.is_none());
        assert_eq!((state.first_name.as_str(), state.last_name.as_str()), ("Sam", "Smith"));
        assert_eq!(state.pass_phrase, "");
    }

    #[test]
    fn changing_the_name_is_ignored_while_submitting() {
        let backend = FakeBackend::new(None);
        let mut state = answering("Sam Smith");
        state.apply(Action::Submit, &backend);
        state.apply(Action::ChangeName, &backend);
        assert!(matches!(state.phase, Phase::Answering { .. }));
        assert!(state.is_busy());
    }

    /// The Starship survey, but scored.
    fn scored_survey() -> Survey {
        let mut survey = sample(SURVEY_ID);
        survey.scored = true;
        survey
    }

    fn done_with(survey: Survey) -> State {
        State {
            phase: Phase::Done { survey },
            ..State::default()
        }
    }

    fn score(user: &str, points: usize) -> Score {
        Score {
            user: user.into(),
            points,
        }
    }

    #[test]
    fn a_scored_survey_fetches_the_scores_after_the_results() {
        let ranking = vec![score("Ann", 2), score("Bob", 1)];
        let backend = FakeBackend::new(None).with_scores(Some(Ok(ranking.clone())));
        let mut state = done_with(scored_survey());

        state.poll(&backend); // results requested
        assert_eq!(backend.scores_requests.get(), 0, "the results come first");
        state.poll(&backend); // results in, scores requested
        assert!(state.results.is_some() && state.scores.is_none());
        assert_eq!(backend.scores_requests.get(), 1);
        state.poll(&backend); // scores in
        assert_eq!(state.scores, Some(ranking));
        assert!(!state.is_busy());

        state.poll(&backend);
        assert_eq!((backend.results_requests.get(), backend.scores_requests.get()), (1, 1));
    }

    #[test]
    fn a_survey_without_scores_never_asks_for_them() {
        let backend = FakeBackend::new(None);
        let mut state = done_with(sample(SURVEY_ID));
        for _ in 0..4 {
            state.poll(&backend);
        }
        assert!(state.results.is_some());
        assert_eq!(backend.scores_requests.get(), 0);
        assert_eq!(state.scores, None);
    }

    #[test]
    fn failed_scores_are_reported_and_not_retried_until_dismissed() {
        let backend = FakeBackend::new(None).with_scores(Some(Err("Cannot reach the server".into())));
        let mut state = done_with(scored_survey());
        for _ in 0..3 {
            state.poll(&backend);
        }
        assert_eq!(state.error.as_deref(), Some("Cannot reach the server"));
        state.poll(&backend);
        assert_eq!(backend.scores_requests.get(), 1);

        state.apply(Action::DismissError, &backend);
        state.poll(&backend);
        assert_eq!(backend.scores_requests.get(), 2);
    }

    #[test]
    fn submitting_a_scored_survey_forgets_old_scores_and_going_back_clears_them() {
        let backend = FakeBackend::new(Some(Ok(())));
        let mut state = known("Sam Smith", &backend);
        start(&mut state, SURVEY_ID, &backend);
        if let Phase::Answering { survey, .. } = &mut state.phase {
            survey.scored = true;
        }
        state.scores = Some(vec![score("Old", 1)]);
        state.apply(Action::Submit, &backend);
        state.poll(&backend);
        assert_eq!(state.scores, None);

        state.scores = Some(vec![score("Ann", 1)]);
        state.apply(Action::ChooseAnotherSurvey, &backend);
        assert_eq!(state.scores, None);
    }
}
