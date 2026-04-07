// CLEAN ARCHITECTURE
mod adapters;
mod entities;
mod ports;
mod use_cases;

pub(crate) use adapters::*;
pub(crate) use entities::*;
use log::error;
pub use ports::*;
use tokio::spawn;

static SURVEY_CONFIG: &str = include_str!("./surveys/survey_spacex_starship.json");

use std::sync::mpsc::{self, Receiver, Sender};

#[cfg(debug_assertions)]
pub use use_cases::*;

#[cfg(debug_assertions)]
use crate::repo::Db;

/// Combined trait for types that implement both `Surveys` and `Users`.
trait SurveysAndUsers: repo::Surveys + repo::Users + Send + Sync {}
impl<T: repo::Surveys + repo::Users + Send + Sync> SurveysAndUsers for T {}

/// A small sync adapter that exposes the old sync `Prono`-style behaviour while
/// performing async work on a background thread. Requests are sent to the
/// background thread via `std::sync::mpsc::Sender` and per-request response
/// channels are used to deliver results. Callers can `try_recv` on the
/// returned receiver to avoid blocking the GUI thread.
pub struct SyncPronoAdapter {
    req_tx: Sender<Request>,
    #[cfg_attr(debug_assertions, allow(dead_code))]
    device_id: String,
    startup_warning: Option<String>,
}

enum Request {
    AddUser {
        user: String,
        device_id: String,
        resp: Sender<PronoResult<()>>,
    },
    AddAnswer {
        user: String,
        question_id: String,
        answer: Answer,
        resp: Sender<PronoResult<()>>,
    },
    Response {
        user: String,
        survey_id: u64,
        resp: Sender<Option<Survey>>,
    },
    AllAnswers {
        question_id: String,
        resp: Sender<Vec<(String, Answer)>>,
    },
}

impl SyncPronoAdapter {
    // The adapter is constructed using `new_with_db_config`, which initializes
    // the concrete `repo::Db` implementation on the adapter's background
    // runtime so all async work (such as DB connection setup) runs on the
    // same runtime used to service requests.

    /// Construct the adapter and initialize a concrete `repo::Db` implementation
    /// on the adapter's background runtime using the provided `config`.
    ///
    /// Call sites supply the concrete DB implementation type as a type
    /// parameter, e.g. `SyncPronoAdapter::new_with_db_config::<prono_db::MysqlDb>(cfg)`.
    ///
    /// # Errors
    ///
    /// This function will return an error if the database initialization fails.
    pub async fn new_with_db_config<D>(config: D::Config, device_id: String) -> PronoResult<Self>
    where
        D: repo::Db + 'static,
        D::Config: Send + 'static,
    {
        let (req_tx, req_rx) = mpsc::channel::<Request>();

        #[allow(unused_mut)]
        let mut startup_warning = None;
        let db: Box<dyn SurveysAndUsers> = match D::init(config).await {
            Ok(db) => Box::new(db),
            #[allow(unused)]
            Err(err) => {
                #[cfg(not(debug_assertions))]
                {
                    startup_warning = Some(err.to_string());
                    return Ok(Self {
                        req_tx,
                        device_id,
                        startup_warning,
                    });
                }
                #[cfg(debug_assertions)]
                {
                    let msg = format!("{err}. Using in-memory fake database.");
                    error!("{msg}");
                    startup_warning = Some(msg);
                    Box::new(fake_db::FakeRepo::init(()).await?)
                }
            }
        };

        // Task not 100% needed if the app requires a database connection
        spawn(async move {
            for req in req_rx {
                match req {
                    Request::AddUser { user, device_id, resp } => {
                        let result = db.add_user(&user, &device_id).await;
                        if let Err(ref e) = result {
                            error!("Failed to add user {user}: {e}");
                        }
                        let _ = resp.send(result);
                    }
                    Request::AddAnswer {
                        user,
                        question_id,
                        answer,
                        resp,
                    } => {
                        let result = db.add_answer(&user, question_id, answer.into()).await;
                        if let Err(ref e) = result {
                            error!("Failed to add answer for user {user}: {e}");
                        }
                        let _ = resp.send(result);
                    }
                    Request::Response { user, survey_id, resp } => {
                        let result = db.response(&user, survey_id).await.map(Into::into);
                        let _ = resp.send(result);
                    }
                    Request::AllAnswers { question_id, resp } => {
                        let result = db.all_answers(question_id).await;
                        let converted = result.into_iter().map(|(u, a)| (u, a.into())).collect();
                        let _ = resp.send(converted);
                    }
                }
            }
        });

        Ok(Self {
            req_tx,
            device_id,
            startup_warning,
        })
    }

    /// Returns a warning message if the database connection failed at startup
    /// and a fallback was used (debug builds only).
    #[must_use]
    pub fn startup_warning(&self) -> Option<&str> {
        self.startup_warning.as_deref()
    }

    /// Request to add a user with device registration; returns a receiver.
    #[must_use]
    pub fn request_add_user(&self, user: &str) -> Receiver<PronoResult<()>> {
        let (tx, rx) = mpsc::channel();

        // In debug mode, generate a unique device_id per user so "Survey again"
        // with a different username can create a new user on the same device.
        #[cfg(debug_assertions)]
        let device_id = uuid::Uuid::new_v4().to_string();
        #[cfg(not(debug_assertions))]
        let device_id = self.device_id.clone();

        let _ = self.req_tx.send(Request::AddUser {
            user: user.to_string(),
            device_id,
            resp: tx,
        });
        rx
    }

    /// Request response (alias); returns a receiver you can `try_recv` on.
    #[must_use]
    pub fn request_response(&self, user: &str, survey_id: u64) -> Receiver<Option<Survey>> {
        let (tx, rx) = mpsc::channel();
        let _ = self.req_tx.send(Request::Response {
            user: user.to_string(),
            survey_id,
            resp: tx,
        });
        rx
    }

    /// Request to add an answer; returns a receiver you can `try_recv` on.
    #[must_use]
    pub fn request_add_answer(&self, user: &str, question_id: String, answer: Answer) -> Receiver<PronoResult<()>> {
        let (tx, rx) = mpsc::channel();
        let _ = self.req_tx.send(Request::AddAnswer {
            user: user.to_string(),
            question_id,
            answer,
            resp: tx,
        });
        rx
    }

    /// Request all responses (alias); returns a receiver you can `try_recv` on.
    #[must_use]
    pub fn request_all_answers(&self, question_id: String) -> Receiver<Vec<(String, Answer)>> {
        let (tx, rx) = mpsc::channel();
        let _ = self.req_tx.send(Request::AllAnswers { question_id, resp: tx });
        rx
    }
}

/// Returns an empty survey template parsed from the embedded survey JSON.
#[must_use]
pub fn empty_survey() -> prono_api::Survey {
    let survey: Survey = FileSurvey::create_from_file(SURVEY_CONFIG).into();
    survey.into()
}

// It will issue requests to the background thread and try to `try_recv` the per-call
// response channel. If the response isn't ready yet the method returns `None`.
// This keeps the GUI thread non-blocking while allowing callers to poll for results.
impl prono_api::Surveys for SyncPronoAdapter {
    fn empty_survey(&self) -> prono_api::Survey {
        let survey: Survey = FileSurvey::create_from_file(SURVEY_CONFIG).into();

        survey.into()
    }

    fn add_user(&mut self, user: &str) {
        let rx = self.request_add_user(user);
        if let Ok(Err(e)) = rx.try_recv() {
            error!("Failed to add user: {e}");
        }
    }

    fn add_answer(&mut self, user: &str, question_id: String, answer: prono_api::Answer) {
        let rx = self.request_add_answer(user, question_id, answer.into());
        if let Ok(Err(e)) = rx.try_recv() {
            error!("Failed to add answer: {e}");
        }
    }

    fn response(&self, user: &str, id: u64) -> Option<prono_api::Survey> {
        let rx = self.request_response(user, id);
        match rx.try_recv() {
            Ok(opt) => opt.map(Into::into),
            _ => None,
        }
    }

    fn all_answers(&self, question_id: String) -> Vec<(String, prono_api::Answer)> {
        // For simplicity, this method is implemented synchronously by blocking on the async API.
        // In a real application, you might want to implement this more efficiently.
        let rx = self.request_all_answers(question_id);
        match rx.recv() {
            Ok(answers) => answers
                .into_iter()
                .map(|(u, a)| {
                    log::debug!("Retrieved answer for user {u}: {a:?}");
                    (u, a.into())
                })
                .collect(),
            Err(e) => {
                error!("Failed to retrieve all answers: {e}");
                Vec::new()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_survey_returns_non_empty_survey() {
        let survey = empty_survey();
        assert!(!survey.description.is_empty());
        assert!(!survey.questions.is_empty());
    }

    #[cfg(debug_assertions)]
    #[tokio::test]
    async fn test_sync_prono_adapter_with_fake_db() {
        let adapter = SyncPronoAdapter::new_with_db_config::<fake_db::FakeRepo>((), "test-device".to_string())
            .await
            .unwrap();

        assert!(adapter.startup_warning().is_none());

        let survey = prono_api::Surveys::empty_survey(&adapter);
        assert!(!survey.questions.is_empty());
    }

    #[cfg(debug_assertions)]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_sync_prono_adapter_add_and_retrieve() {
        let mut adapter = SyncPronoAdapter::new_with_db_config::<fake_db::FakeRepo>((), "test-device".to_string())
            .await
            .unwrap();

        let survey = prono_api::Surveys::empty_survey(&adapter);
        let qid = survey.questions[0].id.clone();

        prono_api::Surveys::add_answer(
            &mut adapter,
            "testuser",
            qid.clone(),
            prono_api::Answer::Text("hello".to_string()),
        );

        // Give the background task time to process
        std::thread::sleep(std::time::Duration::from_millis(50));

        let answers = prono_api::Surveys::all_answers(&adapter, qid);
        assert_eq!(answers.len(), 1);
        assert_eq!(answers[0].0, "testuser");
    }
}
