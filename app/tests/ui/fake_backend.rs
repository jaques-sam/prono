//! An in-memory stand-in for the Prono backend: the real use cases from the
//! `prono` crate running on a `MemoryRepo`, so the rules (one answer per
//! name, names bound to devices) are the server's, without any HTTP or
//! database. Which surveys a pass phrase opens, and which submissions are
//! accepted, follows the pass phrases of the repository.

use std::cell::{Cell, RefCell};

use futures::FutureExt as _;
use prono::{MemoryRepo, Repo as _};
use prono_api::{Answer, AnswerKind, Question, Results, Score, Submission, Survey, samples};
use prono_app::client::{Backend, Pending};

/// The device the app under test runs on.
pub const MY_DEVICE: &str = "my-device";

/// The pass phrase of the Starship survey and the quiz.
pub const PASS_PHRASE: &str = "open sesame";
/// The pass phrase of the course survey.
pub const COURSE_PASS_PHRASE: &str = "rust rocks";
pub const COURSE_ID: u64 = 2;

/// The scored survey: both capitals are right in the options, the answers
/// are not shown anywhere in the app.
pub const QUIZ_ID: u64 = 3;
pub const QUIZ_DESCRIPTION: &str = "Capitals quiz";
const QUIZ_JSON: &str = r#"{
    "survey_id": 3,
    "description": "Capitals quiz",
    "questions": [
        {"question": "Capital of Belgium", "answer_type": "poll",
         "possible_answers": ["Brussels", "Paris"], "correct_answer": "Brussels"},
        {"question": "Capital of France", "answer_type": "poll",
         "possible_answers": ["Brussels", "Paris"], "other_label": "Another city:", "correct_answer": "Paris"}
    ]
}"#;

/// How the fake server reacts to requests.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Health {
    Online,
    /// Requests fail as if the server could not be reached.
    Unreachable,
    /// Requests never get an answer.
    Hanging,
}

pub struct FakeBackend {
    repo: MemoryRepo,
    pub submitting: Cell<Health>,
    pub results: Cell<Health>,
    /// The name of each check whether a user answered, in order.
    pub name_checks: RefCell<Vec<String>>,
    /// The pass phrase of each request for surveys, in order.
    pub survey_requests: RefCell<Vec<Option<String>>>,
    /// The user of each submission the app sent, in order.
    pub submitted_by: RefCell<Vec<String>>,
}

impl FakeBackend {
    pub fn new() -> Self {
        Self {
            repo: Self::repo(),
            submitting: Cell::new(Health::Online),
            results: Cell::new(Health::Online),
            name_checks: RefCell::default(),
            survey_requests: RefCell::default(),
            submitted_by: RefCell::default(),
        }
    }

    /// A repository with the sample surveys and the quiz, each with its pass
    /// phrase.
    fn repo() -> MemoryRepo {
        let repo = MemoryRepo::with_surveys([samples::STARSHIP_JSON, samples::COURSE_JSON, QUIZ_JSON]);
        for survey in Self::now(prono::surveys(&repo)).expect("the surveys are valid") {
            Self::now(repo.set_pass_phrase(survey.id, pass_phrase_of(survey.id))).expect("setting succeeds");
        }
        repo
    }

    /// Replaces the pass phrase of survey `survey_id` on the server.
    pub fn change_pass_phrase(&self, survey_id: u64, pass_phrase: &str) {
        Self::now(self.repo.set_pass_phrase(survey_id, pass_phrase)).expect("setting succeeds");
    }

    /// Lets every request fail like an unreachable server.
    pub fn go_offline(&self) {
        self.submitting.set(Health::Unreachable);
        self.results.set(Health::Unreachable);
    }

    /// Lets every request succeed again.
    pub fn go_online(&self) {
        self.submitting.set(Health::Online);
        self.results.set(Health::Online);
    }

    /// Lets every request wait forever for an answer.
    pub fn hang(&self) {
        self.submitting.set(Health::Hanging);
        self.results.set(Health::Hanging);
    }

    /// `user` answered every question of survey `survey_id` earlier, from
    /// `device_id`: dates predict `month`/`year`, polls pick their first
    /// option, text questions say "text".
    pub fn seed(&self, survey_id: u64, user: &str, device_id: &str, month: u8, year: u16) {
        self.seed_answers(survey_id, user, device_id, |question| match question.kind {
            AnswerKind::PredictionDate => Answer::PredictionDate { day: None, month, year },
            AnswerKind::Text => Answer::Text("text".into()),
            AnswerKind::Poll => Answer::Poll(question.possible_answers[0].clone()),
        });
    }

    /// `user` answered every question of survey `survey_id` earlier, from
    /// `device_id`, with the answer `answer_to` gives for each question.
    pub fn seed_answers(
        &self,
        survey_id: u64,
        user: &str,
        device_id: &str,
        mut answer_to: impl FnMut(&Question) -> Answer,
    ) {
        let survey = Self::now(prono::survey(&self.repo, survey_id)).expect("the seeded survey exists");
        let answers: Vec<_> = survey.questions.iter().map(&mut answer_to).collect();
        let submission = Submission::new(user, &survey, &answers).with_pass_phrase(pass_phrase_of(survey_id));
        Self::now(prono::submit(&self.repo, survey_id, device_id, &submission)).expect("seeding succeeds");
    }

    /// The result of a repository use case. The in-memory repository never
    /// waits, so the future is ready at once.
    fn now<T>(use_case: impl Future<Output = prono::Result<T>>) -> prono::Result<T> {
        use_case.now_or_never().expect("the in-memory repository never waits")
    }

    /// The request's outcome given `health`; `serve` answers when online.
    fn respond<T>(health: Health, serve: impl FnOnce() -> prono::Result<T>) -> Pending<T> {
        match health {
            Health::Online => Pending::ready(serve().map_err(|err| err.to_string())),
            Health::Unreachable => Pending::ready(Err("Cannot reach the server (connection refused)".into())),
            Health::Hanging => Pending::never(),
        }
    }
}

impl Backend for FakeBackend {
    fn submit(&self, survey_id: u64, submission: &Submission) -> Pending<()> {
        self.submitted_by.borrow_mut().push(submission.user.clone());
        Self::respond(self.submitting.get(), || {
            Self::now(prono::submit(&self.repo, survey_id, MY_DEVICE, submission))
        })
    }

    fn open_surveys(&self, pass_phrase: Option<&str>) -> Pending<Vec<Survey>> {
        self.survey_requests.borrow_mut().push(pass_phrase.map(str::to_string));
        // Always answers: the scenarios about a server that fails to deliver
        // the results still start a survey.
        Pending::ready(Self::now(prono::open_surveys(&self.repo, pass_phrase)).map_err(|err| err.to_string()))
    }

    fn results(&self, survey_id: u64) -> Pending<Results> {
        Self::respond(self.results.get(), || Self::now(prono::results(&self.repo, survey_id)))
    }

    fn scores(&self, survey_id: u64) -> Pending<Vec<Score>> {
        Self::respond(self.results.get(), || Self::now(prono::scores(&self.repo, survey_id)))
    }

    fn has_answered(&self, user: &str) -> Pending<bool> {
        self.name_checks.borrow_mut().push(user.to_string());
        Self::respond(self.results.get(), || {
            Self::now(prono::has_answered(&self.repo, MY_DEVICE, user))
        })
    }
}

/// The pass phrase of survey `survey_id`: the course has one of its own.
pub fn pass_phrase_of(survey_id: u64) -> &'static str {
    if survey_id == COURSE_ID {
        COURSE_PASS_PHRASE
    } else {
        PASS_PHRASE
    }
}
