use std::sync::mpsc::{self, Receiver, Sender};

use log::error;
use tokio::spawn;

use crate::entities::{Answer, Survey};
use crate::ports::{PronoResult, repo};
use crate::use_cases::submit_answers;

/// Combined trait for types that implement both `Surveys` and `Users`.
trait SurveysAndUsers: repo::Surveys + repo::Users + Send + Sync {}
impl<T: repo::Surveys + repo::Users + Send + Sync> SurveysAndUsers for T {}

/// A small sync adapter that exposes a sync `Surveys`-style API while performing
/// async work on a background Tokio task. Requests are sent through
/// `std::sync::mpsc` and per-request response channels are used to deliver
/// results, so the GUI thread can `try_recv` without blocking.
pub struct SyncPronoAdapter {
    req_tx: Sender<Request>,
    #[cfg_attr(debug_assertions, allow(dead_code))]
    device_id: String,
    startup_warning: Option<String>,
}

enum Request {
    SubmitAnswers {
        user: String,
        device_id: String,
        answers: Vec<(String, Answer)>,
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
    /// Construct the adapter and initialize a concrete `repo::Db` implementation
    /// on the adapter's background runtime using the provided `config`.
    ///
    /// Call sites supply the concrete DB implementation type as a type
    /// parameter, e.g. `SyncPronoAdapter::new_with_db_config::<prono_db::MysqlDb>(cfg, device_id)`.
    ///
    /// # Errors
    ///
    /// Returns an error in release builds if the database cannot be initialized.
    /// In debug builds, it falls back to an in-memory fake and records a
    /// startup warning instead of failing.
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
                    use crate::adapters::fake_db::FakeRepo;
                    use crate::ports::repo::Db as _;
                    let msg = format!("{err}. Using in-memory fake database.");
                    error!("{msg}");
                    startup_warning = Some(msg);
                    Box::new(FakeRepo::init(()).await?)
                }
            }
        };

        spawn(async move {
            for req in req_rx {
                match req {
                    Request::SubmitAnswers {
                        user,
                        device_id,
                        answers,
                        resp,
                    } => {
                        let converted_answers = answers.into_iter().map(|(q, a)| (q, a.into())).collect();
                        let result = submit_answers(&*db, &*db, &user, &device_id, converted_answers).await;
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

    /// In debug builds we generate a fresh device id per submission so the
    /// "Survey again" flow on a single dev machine can re-register without
    /// hitting the device-mismatch check. In release we use the persistent id.
    #[cfg(debug_assertions)]
    fn submission_device_id(&self) -> String {
        uuid::Uuid::new_v4().to_string()
    }

    #[cfg(not(debug_assertions))]
    fn submission_device_id(&self) -> String {
        self.device_id.clone()
    }

    /// Submit a batch of answers (registering the user if necessary). Returns a
    /// receiver that completes once the background task processes the request.
    #[must_use]
    pub fn request_submit_answers(&self, user: &str, answers: Vec<(String, Answer)>) -> Receiver<PronoResult<()>> {
        let (tx, rx) = mpsc::channel();
        let _ = self.req_tx.send(Request::SubmitAnswers {
            user: user.to_string(),
            device_id: self.submission_device_id(),
            answers,
            resp: tx,
        });
        rx
    }

    /// Request the stored response for `user` & `survey_id`.
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

    /// Request all answers for a question across users.
    #[must_use]
    pub fn request_all_answers(&self, question_id: String) -> Receiver<Vec<(String, Answer)>> {
        let (tx, rx) = mpsc::channel();
        let _ = self.req_tx.send(Request::AllAnswers { question_id, resp: tx });
        rx
    }
}

impl prono_api::Surveys for SyncPronoAdapter {
    fn empty_survey(&self) -> prono_api::Survey {
        crate::empty_survey()
    }

    fn add_answers(&mut self, user: &str, answers: Vec<(String, prono_api::Answer)>) {
        let converted: Vec<(String, Answer)> = answers.into_iter().map(|(q, a)| (q, a.into())).collect();
        let rx = self.request_submit_answers(user, converted);
        if let Ok(Err(e)) = rx.try_recv() {
            error!("Failed to submit answers: {e}");
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

#[cfg(all(test, debug_assertions))]
mod tests {
    use super::*;
    use crate::adapters::fake_db::FakeRepo;

    #[tokio::test]
    async fn new_with_fake_db_has_no_warning() {
        let adapter = SyncPronoAdapter::new_with_db_config::<FakeRepo>((), "test-device".into())
            .await
            .unwrap();
        assert!(adapter.startup_warning().is_none());
        let survey = prono_api::Surveys::empty_survey(&adapter);
        assert!(!survey.questions.is_empty());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn submit_then_retrieve_via_fake_db() {
        let mut adapter = SyncPronoAdapter::new_with_db_config::<FakeRepo>((), "test-device".into())
            .await
            .unwrap();

        let survey = prono_api::Surveys::empty_survey(&adapter);
        let qid1 = survey.questions[0].id.clone();
        let qid2 = survey.questions[1].id.clone();

        prono_api::Surveys::add_answers(
            &mut adapter,
            "testuser",
            vec![
                (qid1.clone(), prono_api::Answer::Text("hello".to_string())),
                (qid2.clone(), prono_api::Answer::Text("world".to_string())),
            ],
        );

        std::thread::sleep(std::time::Duration::from_millis(50));

        let answers1 = prono_api::Surveys::all_answers(&adapter, qid1);
        assert_eq!(answers1.len(), 1);
        assert_eq!(answers1[0].0, "testuser");

        let answers2 = prono_api::Surveys::all_answers(&adapter, qid2);
        assert_eq!(answers2.len(), 1);
        assert_eq!(answers2[0].0, "testuser");
    }
}
