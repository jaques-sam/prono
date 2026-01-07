use async_trait::async_trait;
use log::{error, info};
use prono::PronoResult;
use sqlx::mysql::MySqlPoolOptions;
use sqlx::{MySqlPool, Row};
use std::time::Duration;

use crate::{Config, DbError};
use prono::repo::{self, Answer};

pub struct MysqlDb {
    pool: MySqlPool,
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
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("create rt");
        rt.block_on(async { Self::connect(secure_config).await.expect("connect") })
    }

    /// # Errors
    ///
    /// This function will return an error if the database connection fails.
    pub async fn connect(secure_config: &Config) -> Result<Self, sqlx::Error> {
        let database_url = secure_config.construct_url();
        let database_url = database_url.unsecure();
        let pool = MySqlPoolOptions::new()
            .max_connections(5)
            .idle_timeout(Duration::from_secs(5))
            .connect(database_url)
            .await?;
        Ok(Self { pool })
    }
}

#[async_trait]
impl repo::Surveys for MysqlDb {
    async fn answer(&self, user: &str, question_id: u64) -> Option<repo::Answer> {
        let qid = question_id.to_string();
        let row = sqlx::query("SELECT answer FROM AnswerResponse WHERE user = ? AND question_id = ?")
            .bind(user)
            .bind(qid)
            .fetch_optional(&self.pool)
            .await
            .ok()?;
        let row = row?;
        let answer: String = row.get("answer");
        Some(Answer::from(answer))
    }

    async fn response(&self, user: &str, survey_id: u64) -> Option<repo::Survey> {
        let rows = sqlx::query("SELECT question_id, answer FROM AnswerResponse WHERE user = ? AND survey_id = ?")
            .bind(user)
            .bind(survey_id)
            .fetch_all(&self.pool)
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
    }

    async fn add_answer(&self, user: &str, question_id: String, answer: repo::Answer) -> PronoResult<()> {
        let existing = sqlx::query("SELECT 1 FROM AnswerResponse WHERE user = ? AND question_id = ?")
            .bind(user)
            .bind(&question_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(DbError::from)?;

        if existing.is_some() {
            return Err(Error::AnswerExists);
        }
        let ans = answer.to_string();
        sqlx::query("INSERT INTO AnswerResponse (user, question_id, answer) VALUES (?, ?, ?)")
            .bind(user)
            .bind(question_id)
            .bind(ans)
            .execute(&self.pool)
            .await
            .map_err(DbError::from)?;
        Ok(())
    }
}
