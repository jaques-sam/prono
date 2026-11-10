use std::time::Duration;

use async_trait::async_trait;
use log::info;
use prono_api::{Answer, UserAnswer};
use sqlx::Row;
use sqlx::mysql::{MySqlConnectOptions, MySqlPool, MySqlPoolOptions};

use crate::{DbConfig, Error, Repo, Result};

const DB_NAME: &str = "db_prono";

/// The production [`Repo`]: a MySQL/MariaDB database with the tables
/// described in `backend/README.md`.
pub struct MysqlRepo {
    pool: MySqlPool,
}

impl MysqlRepo {
    /// # Errors
    ///
    /// Fails if the database cannot be reached.
    pub async fn connect(config: &DbConfig) -> Result<Self> {
        let options = MySqlConnectOptions::new()
            .host(&config.host)
            .port(config.port)
            .username(&config.user)
            .password(&config.pass)
            .database(DB_NAME);
        let pool = MySqlPoolOptions::new()
            .max_connections(5)
            .idle_timeout(Duration::from_secs(10))
            .connect_with(options)
            .await
            .map_err(|err| connect_error(config, &err))?;
        info!("Connected to database at {}:{}", config.host, config.port);
        Ok(Self { pool })
    }
}

fn connect_error(config: &DbConfig, err: &sqlx::Error) -> Error {
    Error::Repository(format!(
        "cannot connect to the database at {}:{} (check its settings in `secure_config.toml` or `PRONO_DB_*`): {err}",
        config.host, config.port
    ))
}

#[async_trait]
impl Repo for MysqlRepo {
    async fn device_of(&self, user: &str) -> Result<Option<String>> {
        Ok(
            sqlx::query_scalar("SELECT device_id FROM Users WHERE user_name = ? LIMIT 1")
                .bind(user)
                .fetch_optional(&self.pool)
                .await?,
        )
    }

    async fn user_of(&self, device_id: &str) -> Result<Option<String>> {
        Ok(
            sqlx::query_scalar("SELECT user_name FROM Users WHERE device_id = ? LIMIT 1")
                .bind(device_id)
                .fetch_optional(&self.pool)
                .await?,
        )
    }

    async fn save_answers(&self, user: &str, device_id: &str, answers: &[(String, Answer)]) -> Result<()> {
        // Dropping the transaction on an early return rolls it back.
        let mut tx = self.pool.begin().await?;

        let existing: Option<i64> = sqlx::query_scalar("SELECT user_id FROM Users WHERE user_name = ? LIMIT 1")
            .bind(user)
            .fetch_optional(&mut *tx)
            .await?;
        let user_id = if let Some(id) = existing {
            id
        } else {
            let inserted = sqlx::query("INSERT INTO Users (user_name, device_id) VALUES (?, ?)")
                .bind(user)
                .bind(device_id)
                .execute(&mut *tx)
                .await?;
            i64::try_from(inserted.last_insert_id()).map_err(|e| Error::Repository(e.to_string()))?
        };

        for (question_id, _) in answers {
            let answered: Option<i64> =
                sqlx::query_scalar("SELECT 1 FROM AnswerResponse WHERE user_id = ? AND question_id = ? LIMIT 1")
                    .bind(user_id)
                    .bind(question_id)
                    .fetch_optional(&mut *tx)
                    .await?;
            if answered.is_some() {
                return Err(Error::AnswerExists);
            }
        }

        for (question_id, answer) in answers {
            sqlx::query("INSERT INTO AnswerResponse (user_id, question_id, answer) VALUES (?, ?, ?)")
                .bind(user_id)
                .bind(question_id)
                .bind(to_db(answer))
                .execute(&mut *tx)
                .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn answers(&self, question_id: &str) -> Result<Vec<UserAnswer>> {
        let rows = sqlx::query(
            "SELECT u.user_name, ar.answer
             FROM AnswerResponse ar
             JOIN Users u ON ar.user_id = u.user_id
             WHERE ar.question_id = ?",
        )
        .bind(question_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                Ok(UserAnswer {
                    user: row.try_get("user_name")?,
                    answer: from_db(row.try_get("answer")?),
                })
            })
            .collect()
    }

    async fn users(&self) -> Result<Vec<String>> {
        Ok(sqlx::query_scalar("SELECT user_name FROM Users")
            .fetch_all(&self.pool)
            .await?)
    }

    async fn delete_user(&self, user: &str) -> Result<()> {
        // `ON DELETE CASCADE` removes the user's answers too.
        sqlx::query("DELETE FROM Users WHERE user_name = ?")
            .bind(user)
            .execute(&self.pool)
            .await?;
        info!("Deleted user '{user}' and their answers");
        Ok(())
    }
}

/// Storage format of an answer: `DD/MM/YYYY` for dates (`00` when there is
/// no day), the text itself otherwise.
fn to_db(answer: &Answer) -> String {
    match answer {
        Answer::Text(text) => text.clone(),
        Answer::PredictionDate { day, month, year } => format!("{:02}/{month:02}/{year:04}", day.unwrap_or(0)),
    }
}

/// Inverse of [`to_db`]; also accepts `MM/YYYY`.
fn from_db(text: String) -> Answer {
    let parts: Vec<&str> = text.split('/').collect();
    let (day, month, year) = match parts[..] {
        [day, month, year] => (day.parse::<u8>().ok(), month, year),
        [month, year] => (Some(0), month, year),
        _ => (None, "", ""),
    };
    let date = day.zip(month.parse::<u8>().ok()).zip(year.parse::<u16>().ok());
    match date {
        Some(((day, month), year)) if (1..=12).contains(&month) => Answer::PredictionDate {
            day: (day != 0).then_some(day),
            month,
            year,
        },
        _ => Answer::Text(text),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(day: Option<u8>, month: u8, year: u16) -> Answer {
        Answer::PredictionDate { day, month, year }
    }

    #[test]
    fn dates_round_trip() {
        for answer in [date(None, 5, 2025), date(Some(15), 12, 2030)] {
            assert_eq!(from_db(to_db(&answer)), answer);
        }
        assert_eq!(to_db(&date(None, 5, 2025)), "00/05/2025");
        assert_eq!(to_db(&date(Some(15), 5, 2025)), "15/05/2025");
    }

    #[test]
    fn month_year_parses_as_date() {
        assert_eq!(from_db("05/2025".into()), date(None, 5, 2025));
    }

    #[test]
    fn everything_else_is_text() {
        for text in ["Some random text", "13/2025", "00/2025", "1/2/3/4", "aa/bb/cccc", ""] {
            assert_eq!(from_db(text.into()), Answer::Text(text.into()), "{text}");
        }
        assert_eq!(to_db(&Answer::Text("hello".into())), "hello");
    }

    #[test]
    fn connect_error_names_the_database() {
        let config = DbConfig {
            host: "db.local".into(),
            port: 3306,
            user: "nobody".into(),
            pass: String::new(),
        };
        let message = connect_error(&config, &sqlx::Error::PoolTimedOut).to_string();
        assert!(
            message.contains("cannot connect to the database at db.local:3306"),
            "{message}"
        );
        assert!(message.contains("pool timed out"), "{message}");
    }
}
