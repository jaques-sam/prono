use log::{error, info};
use sqlx::mysql::MySqlPoolOptions;
use sqlx::{MySqlPool, Row};
use std::sync::Arc;
use std::time::Duration;

use crate::{AnswerResponse, Config, User};
use prono::repo::{self, Answer};
use tokio::{runtime, sync::Mutex};

struct State {
    pool: MySqlPool,
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

        let rt = runtime::Builder::new_current_thread().enable_all().build().unwrap();
        let state: Arc<Mutex<Option<State>>> = Arc::new(Mutex::new(None));
        let state_clone = state.clone();

        rt.block_on(async move {
            let pool = MySqlPoolOptions::new()
                .max_connections(5)
                .connect(database_url)
                .await
                .expect("failed to connect to db");

            *state_clone.lock().await = Some(State { pool });
        });
        Self { rt, state }
    }
}

impl repo::Surveys for MysqlDb {
    fn answer(&self, user: &str, question_id: u64) -> Option<repo::Answer> {
        info!("DB: user {user} asks for answer for question id={question_id}");
        let state = self.state.clone();
        let user = user.to_string();
        let qid = question_id.to_string();

        self.rt.block_on(async move {
            let mut lock = state.lock().await;
            let state = lock.as_mut().expect("catch this error");
            let pool = &state.pool;
            let row = sqlx::query("SELECT answer FROM AnswerResponse WHERE user = ? AND question_id = ?")
                .bind(user)
                .bind(qid)
                .fetch_optional(pool)
                .await
                .ok()?;

            let row = row?;
            let answer: String = row.get("answer");
            Some(Answer::from(answer))
        })
    }

    fn response(&self, user: &str, survey_id: u64) -> Option<repo::Survey> {
        info!("DB: user {user} asks for response for survey id={survey_id}");
        let state = self.state.clone();
        let user = user.to_string();

        self.rt.block_on(async move {
            let mut lock = state.lock().await;
            let state = lock.as_mut().expect("catch this error");
            let pool = &state.pool;
            let rows = sqlx::query("SELECT question_id, answer FROM AnswerResponse WHERE user = ? AND survey_id = ?")
                .bind(user)
                .bind(survey_id)
                .fetch_all(pool)
                .await
                .ok()?;

            let mut questions = Vec::new();
            for row in rows {
                let qid: String = row.get("question_id");
                let ans: String = row.get("answer");
                questions.push(repo::Question {
                    id: qid,
                    answer: Answer::from(ans),
                    text: None,
                });
            }

            Some(repo::Survey {
                id: survey_id,
                description: None,
                questions,
            })
        })
    }

    fn add_answer(&mut self, user: &str, question_id: String, answer: repo::Answer) {
        info!("DB: user {user} adds answer for question id={question_id}");
        let state = self.state.clone();
        let user = user.to_string();
        let ans = answer.to_string();

        self.rt.block_on(async move {
            let mut lock = state.lock().await;
            let state = lock.as_mut().expect("catch this error");
            let pool = &state.pool;
            let _ = sqlx::query("INSERT INTO AnswerResponse (user, question_id, answer) VALUES (?, ?, ?)")
                .bind(user)
                .bind(question_id)
                .bind(ans)
                .execute(pool)
                .await;
        });
    }
}
