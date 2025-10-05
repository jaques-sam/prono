use log::info;
use mysql_async::{prelude::*, Opts};
use std::sync::Arc;

use prono::api::{self, Answer};
use tokio::{runtime, sync::Mutex};

use crate::{AnswerResponse, Config};

struct State {
    connection: mysql_async::Conn,
}

pub struct MysqlDb {
    rt: runtime::Runtime,
    state: Arc<Mutex<Option<State>>>,
}

impl MysqlDb {
    /// Represents a mysql database connection pool and runtime.
    ///
    /// # Panics
    ///
    /// This function will panic if:
    /// - The database URL cannot be constructed from the provided `Config`.
    /// - The database URL is invalid.
    /// - The connection to the database fails.
    ///
    /// # Arguments
    ///
    /// * `secure_config` - A reference to a `Config` object that contains the necessary
    ///   information to construct the database URL.
    ///
    /// # Returns
    ///
    /// A new instance of `MysqlDb`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use secure_string::SecureString;
    /// use prono_db::{Config, MysqlDb};
    ///
    /// let config = Config {
    ///     user: SecureString::from("user"),
    ///     pass: SecureString::from("pass"),
    ///     host: SecureString::from("host"),
    ///     port: SecureString::from("1234"),
    ///     db_name: String::from("database"),
    /// };
    ///
    /// # std::thread::spawn(move || {
    /// let db = MysqlDb::new(&config);
    /// #     std::process::exit(0);
    /// # });
    /// ```
    #[must_use]
    pub fn new(secure_config: &Config) -> Self {
        let database_url = secure_config.construct_url();
        let database_url = database_url.unsecure();
        let pool = mysql_async::Pool::new(Opts::from_url(database_url).expect("catch this error"));

        let rt = runtime::Builder::new_current_thread().enable_all().build().unwrap();
        let state: Arc<Mutex<Option<State>>> = Arc::new(Mutex::new(None));
        let state_clone = state.clone();

        rt.block_on(async move {
            let connection = pool.get_conn().await.expect("catch this error");
            *state_clone.lock().await = Some(State { connection });
        });

        Self { rt, state }
    }
}

impl api::Surveys for MysqlDb {
    fn answer(&self, user: &str, question_id: u64) -> Option<api::Answer> {
        info!("DB: user {user} asks for answer for question id={question_id})");
        let state = self.state.clone();
        let user = user.to_string();
        let question_id = question_id.to_string();
        self.rt.block_on(async move {
            let mut lock = state.lock().await;
            let state = lock.as_mut().expect("catch this error");

            let row: Option<(String, String)> =
                r"SELECT question_id, answer FROM AnswerResponse WHERE user = :user AND question_id = :question_id"
                    .with(params! { "user" => user, "question_id" => question_id })
                    .first(&mut state.connection)
                    .await
                    .expect("catch this error");

            if let Some(row) = row {
                return Some(Answer::from(row.1));
            }

            None
        })
    }

    fn response(&self, user: &str, survey_id: u64) -> Option<api::Survey> {
        info!("DB: user {user} asks for response for survey id={survey_id})");
        let state = self.state.clone();
        let user = user.to_string();

        self.rt.block_on(async move {
            let mut lock = state.lock().await;
            let state = lock.as_mut().expect("catch this error");

            let rows: Vec<(String, String)> =
                r"SELECT question_id, answer FROM AnswerResponse WHERE user = :user AND survey_id = :survey_id"
                    .with(params! { "user" => user, "survey_id" => survey_id })
                    .fetch(&mut state.connection)
                    .await
                    .expect("catch this error");

            let mut questions = Vec::new();
            for row in rows {
                questions.push(api::Question {
                    id: row.0,
                    answer: Answer::from(row.1),
                    text: None,
                });
            }

            Some(api::Survey {
                id: survey_id,
                description: None,
                questions,
            })
        })
    }

    fn add_answer(&mut self, user: &str, question_id: String, answer: api::Answer) {
        info!("DB: user {user} adds answer for question id={question_id})");
        let state = self.state.clone();
        let user = user.to_string();

        self.rt.block_on(async move {
            let mut lock = state.lock().await;
            let state = lock.as_mut().expect("catch this error");
            let answer = answer.to_string();

            r"INSERT INTO AnswerResponse (user, question_id, answer)
              VALUES (:user, :question_id, :answer )"
                .with(params! { "user" => user, "question_id" => question_id, "answer" => answer })
                .ignore(&mut state.connection)
                .await
                .expect("catch this error");
        });
    }
}
