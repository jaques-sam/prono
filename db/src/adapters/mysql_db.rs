use async_trait::async_trait;
use log::{error, info};
use prono::{Error, PronoResult};
use sqlx::mysql::MySqlPoolOptions;
use sqlx::MySqlPool;
use std::time::Duration;

use crate::DbError;
use prono::repo::{self, Answer};

pub struct MysqlDb {
    pool: MySqlPool,
}

impl MysqlDb {
    /// # Errors
    ///
    /// This function will return an error if:
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
    /// Async constructor that connects using the caller's Tokio runtime.
    ///
    /// Use this from an existing runtime to ensure the DB connection is created
    /// on the same runtime as other async work.
    async fn connect(secure_config: &crate::Config) -> Result<Self, sqlx::Error> {
        let database_url = secure_config.construct_url();
        let database_url = database_url.unsecure();
        let pool = MySqlPoolOptions::new()
            .max_connections(5)
            .idle_timeout(Duration::from_secs(10))
            .connect(database_url)
            .await?;
        info!("MySQL database connected.");

        Ok(Self { pool })
    }

    /// Helper to get or create user_id from user_name
    async fn get_or_create_user_id(&self, user_name: &str) -> PronoResult<i64> {
        // Try to get existing user
        let existing = sqlx::query!(
            "SELECT user_id FROM Users WHERE user_name = ?",
            user_name
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(DbError::from)?;

        if let Some(row) = existing {
            return Ok(row.user_id);
        }

        // User doesn't exist, create with empty device_id
        let result = sqlx::query!(
            "INSERT INTO Users (user_name, device_id) VALUES (?, '')",
            user_name
        )
        .execute(&self.pool)
        .await
        .map_err(DbError::from)?;

        Ok(result.last_insert_id() as i64)
    }

    /// Helper to get user_id from user_name (returns None if not found)
    async fn get_user_id(&self, user_name: &str) -> Option<i64> {
        sqlx::query!("SELECT user_id FROM Users WHERE user_name = ?", user_name)
            .fetch_optional(&self.pool)
            .await
            .ok()?
            .map(|row| row.user_id)
    }
}

#[async_trait]
impl repo::Db for MysqlDb {
    type Config = crate::Config;

    async fn init(config: Self::Config) -> PronoResult<Self> {
        info!("Initializing MySQL database...");

        Ok(Self::connect(&config).await.map_err(DbError::from)?)
    }
}

#[async_trait]
impl repo::Surveys for MysqlDb {
    async fn answer(&self, user: &str, question_id: String) -> Option<repo::Answer> {
        let user_id = self.get_user_id(user).await?;

        let row = sqlx::query!(
            "SELECT answer FROM AnswerResponse WHERE user_id = ? AND question_id = ?",
            user_id,
            question_id
        )
        .fetch_optional(&self.pool)
        .await
        .ok()??;

        Some(Answer::from(row.answer))
    }

    async fn response(&self, user: &str, survey_id: u64) -> Option<repo::Survey> {
        let user_id = self.get_user_id(user).await?;

        let rows = sqlx::query!(
            "SELECT question_id, answer FROM AnswerResponse WHERE user_id = ?",
            user_id
        )
        .fetch_all(&self.pool)
        .await
        .ok()?;

        let questions = rows
            .into_iter()
            .map(|row| repo::Question {
                id: row.question_id,
                answer: Answer::from(row.answer),
            })
            .collect();

        Some(repo::Survey {
            id: survey_id,
            description: None,
            questions,
        })
    }

    async fn add_answer(&self, user: &str, question_id: String, answer: repo::Answer) -> PronoResult<()> {
        let user_id = self.get_or_create_user_id(user).await?;

        let existing = sqlx::query!(
            "SELECT 1 as found FROM AnswerResponse WHERE user_id = ? AND question_id = ?",
            user_id,
            question_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(DbError::from)?;

        if existing.is_some() {
            return Err(Error::AnswerExists);
        }

        let ans = answer.to_string();
        sqlx::query!(
            "INSERT INTO AnswerResponse (user_id, question_id, answer) VALUES (?, ?, ?)",
            user_id,
            question_id,
            ans
        )
        .execute(&self.pool)
        .await
        .map_err(DbError::from)?;
        Ok(())
    }

    async fn all_answers(&self, question_id: String) -> Vec<(String, Answer)> {
        let rows = sqlx::query!(
            "SELECT u.user_name, ar.answer
             FROM AnswerResponse ar
             JOIN Users u ON ar.user_id = u.user_id
             WHERE ar.question_id = ?",
            question_id
        )
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        rows.into_iter()
            .map(|row| (row.user_name, Answer::from(row.answer)))
            .collect()
    }
}

#[async_trait]
impl repo::Users for MysqlDb {
    async fn all_users(&self) -> PronoResult<Vec<String>> {
        let rows = sqlx::query!("SELECT user_name FROM Users")
            .fetch_all(&self.pool)
            .await
            .map_err(DbError::from)?;

        Ok(rows.into_iter().map(|row| row.user_name).collect())
    }

    async fn delete_user(&self, name: &str) -> PronoResult<()> {
        // With CASCADE, this will also delete from AnswerResponse
        sqlx::query!("DELETE FROM Users WHERE user_name = ?", name)
            .execute(&self.pool)
            .await
            .map_err(DbError::from)?;
        Ok(())
    }
}

#[async_trait]
impl repo::DeviceRegistry for MysqlDb {
    async fn register_device(&self, user: &str, device_id: &str) -> PronoResult<()> {
        sqlx::query!(
            "INSERT INTO Users (user_name, device_id) VALUES (?, ?)
             ON DUPLICATE KEY UPDATE device_id = VALUES(device_id)",
            user,
            device_id
        )
        .execute(&self.pool)
        .await
        .map_err(DbError::from)?;
        Ok(())
    }

    async fn verify_device(&self, user: &str, device_id: &str) -> PronoResult<bool> {
        let row = sqlx::query!(
            "SELECT device_id FROM Users WHERE user_name = ?",
            user
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(DbError::from)?;

        match row {
            Some(row) => Ok(row.device_id == device_id),
            None => Ok(true),
        }
    }
}
