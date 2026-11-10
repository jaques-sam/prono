use prono_api::{Answer, Results, Submission, Survey};
use serde::{Deserialize, Serialize};

use crate::client::{Backend, Pending};

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
}

enum Request {
    Submit(Pending<()>),
    Results(Pending<Results>),
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
            Action::StartSurvey(survey_id) => self.start(survey_id),
            Action::ResetAnswers => {
                let survey = self.survey();
                if let (Phase::Answering { answers, .. }, Some(survey)) = (&mut self.phase, survey) {
                    *answers = survey.empty_answers();
                }
            }
            Action::Submit => self.submit(backend),
            Action::ChooseAnotherSurvey => {
                self.phase = Phase::Start;
                self.results = None;
                self.request = None;
            }
            Action::DismissError => self.error = None,
        }
    }

    /// Takes in finished requests, and fetches results when they are
    /// missing. Call it every frame.
    pub fn poll(&mut self, backend: &impl Backend) {
        match self.request.take() {
            Some(Request::Submit(pending)) => match pending.take() {
                None => self.request = Some(Request::Submit(pending)),
                Some(Ok(())) => {
                    if let Phase::Answering { survey_id, .. } = self.phase {
                        self.phase = Phase::Done { survey_id };
                        self.results = None;
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

    fn start(&mut self, survey_id: u64) {
        if let Err(err) = prono_api::check_user_name(self.user_name.trim()) {
            self.error = Some(err.to_string());
        } else if let Some(survey) = prono_api::survey(survey_id) {
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

    use super::*;

    const SURVEY_ID: u64 = 1;

    /// Answers requests immediately with `submit_result`, or never if `None`.
    struct FakeBackend {
        submit_result: Option<Result<(), String>>,
        submissions: RefCell<Vec<Submission>>,
        results_requests: Cell<usize>,
    }

    impl FakeBackend {
        fn new(submit_result: Option<Result<(), String>>) -> Self {
            Self {
                submit_result,
                submissions: RefCell::default(),
                results_requests: Cell::default(),
            }
        }
    }

    impl Backend for FakeBackend {
        fn submit(&self, _survey_id: u64, submission: &Submission) -> Pending<()> {
            self.submissions.borrow_mut().push(submission.clone());
            self.submit_result.clone().map_or_else(Pending::never, Pending::ready)
        }

        fn results(&self, _survey_id: u64) -> Pending<Results> {
            self.results_requests.set(self.results_requests.get() + 1);
            Pending::ready(Ok(Results::new()))
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
}
