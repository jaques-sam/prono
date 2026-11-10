//! An in-memory stand-in for the Prono backend: the real use cases from the
//! `prono` crate running on a `MemoryRepo`, so the rules (one answer per
//! name, names bound to devices) are the server's, without any HTTP or
//! database.

use std::cell::{Cell, RefCell};

use futures::FutureExt as _;
use prono::MemoryRepo;
use prono_api::{Answer, Results, Submission};
use prono_app::client::{Backend, Pending};

/// The device the app under test runs on.
pub const MY_DEVICE: &str = "my-device";

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
            repo: MemoryRepo::default(),
            submitting: Cell::new(Health::Online),
            results: Cell::new(Health::Online),
            name_checks: RefCell::default(),
            submitted_by: RefCell::default(),
        }
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
    /// `device_id`, predicting `month`/`year`.
    pub fn seed(&self, survey_id: u64, user: &str, device_id: &str, month: u8, year: u16) {
        let survey = prono_api::survey(survey_id).expect("the seeded survey exists");
        let answers = vec![Answer::PredictionDate { day: None, month, year }; survey.questions.len()];
        let submission = Submission::new(user, survey, &answers);
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

    fn user_answered(&self, user: &str) -> Pending<bool> {
        self.name_checks.borrow_mut().push(user.to_string());
        Self::respond(self.results.get(), || {
            Self::now(prono::has_answered(&self.repo, MY_DEVICE, user))
        })
    }
}
