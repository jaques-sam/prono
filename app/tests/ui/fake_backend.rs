//! An in-memory stand-in for the Prono backend: the real use cases from the
//! `prono` crate running on a `MemoryRepo`, so the rules (one answer per
//! name, names bound to devices) are the server's, without any HTTP or
//! database. Which surveys a name is offered, and may answer, follows the
//! surveys' `access` and the name lists of the repository.

use std::cell::{Cell, RefCell};

use futures::FutureExt as _;
use prono::{MemoryRepo, Repo as _};
use prono_api::{Access, Answer, AnswerKind, Question, Results, Submission, UserStatus, samples};
use prono_app::client::{Backend, Pending};

/// The device the app under test runs on.
pub const MY_DEVICE: &str = "my-device";

/// The pass phrase of the surveys that have one.
pub const PASS_PHRASE: &str = "open sesame";

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
            submitted_by: RefCell::default(),
        }
    }

    /// A repository with the sample surveys, and the pass
    /// phrase of those that have one.
    fn repo() -> MemoryRepo {
        let repo = MemoryRepo::with_surveys([samples::STARSHIP_JSON, samples::COURSE_JSON]);
        for survey in Self::now(prono::surveys(&repo)).expect("the surveys are valid") {
            if survey.access == Access::PassPhrase {
                Self::now(repo.set_pass_phrase(survey.id, PASS_PHRASE)).expect("setting succeeds");
            }
        }
        repo
    }

    /// Puts `user` ("First Last") on the name list of every survey that has
    /// one.
    pub fn put_on_name_list(&self, user: &str) {
        let surveys = Self::now(prono::surveys(&self.repo)).expect("the surveys are valid");
        for survey in surveys.iter().filter(|survey| survey.access == Access::NameList) {
            self.allow(survey.id, user);
        }
    }

    fn allow(&self, survey_id: u64, user: &str) {
        let (first, last) = user.split_once(' ').expect("a first and a last name");
        Self::now(self.repo.allow_user(survey_id, first, last)).expect("listing succeeds");
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
    pub fn seed_answers(&self, survey_id: u64, user: &str, device_id: &str, answer_to: impl Fn(&Question) -> Answer) {
        let survey = Self::now(prono::survey(&self.repo, survey_id)).expect("the seeded survey exists");
        let answers: Vec<_> = survey.questions.iter().map(answer_to).collect();
        let mut submission = Submission::new(user, &survey, &answers);
        match survey.access {
            Access::NameList => self.allow(survey_id, user),
            Access::PassPhrase => submission = submission.with_pass_phrase(PASS_PHRASE),
        }
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

    fn results(&self, survey_id: u64) -> Pending<Results> {
        Self::respond(self.results.get(), || Self::now(prono::results(&self.repo, survey_id)))
    }

    fn user_status(&self, user: &str) -> Pending<UserStatus> {
        self.name_checks.borrow_mut().push(user.to_string());
        Self::respond(self.results.get(), || {
            Self::now(prono::user_status(&self.repo, MY_DEVICE, user))
        })
    }
}
