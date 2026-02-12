// CLEAN ARCHITECTURE
mod entities;
mod ports;
mod use_cases;

pub use entities::*;
use log::error;
pub use ports::*;
use tokio::spawn;

static SURVEY_CONFIG: &str = include_str!("./surveys/survey_spacex_starship.json");

use std::sync::mpsc::{self, Receiver, Sender};

/// A small sync adapter that exposes the old sync `Prono`-style behaviour while
/// performing async work on a background thread. Requests are sent to the
/// background thread via `std::sync::mpsc::Sender` and per-request response
/// channels are used to deliver results. Callers can `try_recv` on the
/// returned receiver to avoid blocking the GUI thread.
pub struct SyncPronoAdapter {
    req_tx: Sender<Request>,
}

enum Request {
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
    pub async fn new_with_db_config<D>(config: D::Config) -> PronoResult<Self>
    where
        D: repo::Db + 'static,
        D::Config: Send + 'static,
    {
        let (req_tx, req_rx) = mpsc::channel::<Request>();

        let db = D::init(config).await?;

        // Task not 100% needed if the app requires a database connection
        spawn(async move {
            let async_api: Box<dyn repo::Surveys + Send + Sync> = Box::new(db);

            for req in req_rx {
                match req {
                    Request::AddAnswer {
                        user,
                        question_id,
                        answer,
                        resp,
                    } => {
                        let result = async_api.add_answer(&user, question_id, answer.into()).await;
                        if let Err(ref e) = result {
                            error!("Failed to add answer for user {user}: {e}");
                        }
                        let _ = resp.send(result);
                    }
                    Request::Response { user, survey_id, resp } => {
                        let result = async_api.response(&user, survey_id).await.map(Into::into);
                        let _ = resp.send(result);
                    }
                }
            }
        });

        Ok(Self { req_tx })
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
}

// It will issue requests to the background thread and try to `try_recv` the per-call
// response channel. If the response isn't ready yet the method returns `None`.
// This keeps the GUI thread non-blocking while allowing callers to poll for results.
impl prono_api::Surveys for SyncPronoAdapter {
    fn empty_survey(&self) -> prono_api::Survey {
        let survey: Survey = FileSurvey::create_from_file(SURVEY_CONFIG).into();

        survey.into()
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
}
