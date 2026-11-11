use std::time::Duration;

use async_trait::async_trait;
use log::info;
use prono_api::{Answer, AnswerKind, Question, UserAnswer};
use sqlx::Row;
use sqlx::mysql::{MySqlConnectOptions, MySqlPool, MySqlPoolOptions};
use uuid::Uuid;

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

    async fn save_answers(&self, user: &str, device_id: &str, answers: &[(Uuid, Answer)]) -> Result<()> {
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
                .await
                .map_err(insert_error)?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn answers(&self, question: &Question) -> Result<Vec<UserAnswer>> {
        let rows = sqlx::query(
            "SELECT u.user_name, ar.answer
             FROM AnswerResponse ar
             JOIN Users u ON ar.user_id = u.user_id
             WHERE ar.question_id = ?",
        )
        .bind(question.id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                Ok(UserAnswer {
                    user: row.try_get("user_name")?,
                    answer: from_db(row.try_get("answer")?, question.kind),
                })
            })
            .collect()
    }

    async fn listed_surveys(&self, user: &str) -> Result<Vec<u64>> {
        // The collation makes the comparison ignore case, like for user names.
        Ok(
            sqlx::query_scalar("SELECT survey_id FROM SurveyAllowedUser WHERE CONCAT(first_name, ' ', last_name) = ?")
                .bind(user)
                .fetch_all(&self.pool)
                .await?,
        )
    }

    async fn pass_phrase(&self, survey_id: u64) -> Result<Option<String>> {
        Ok(
            sqlx::query_scalar("SELECT pass_phrase FROM SurveyPassPhrase WHERE survey_id = ?")
                .bind(survey_id)
                .fetch_optional(&self.pool)
                .await?,
        )
    }

    async fn allow_user(&self, survey_id: u64, first_name: &str, last_name: &str) -> Result<()> {
        sqlx::query("INSERT IGNORE INTO SurveyAllowedUser (survey_id, first_name, last_name) VALUES (?, ?, ?)")
            .bind(survey_id)
            .bind(first_name)
            .bind(last_name)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn disallow_user(&self, survey_id: u64, first_name: &str, last_name: &str) -> Result<()> {
        sqlx::query("DELETE FROM SurveyAllowedUser WHERE survey_id = ? AND first_name = ? AND last_name = ?")
            .bind(survey_id)
            .bind(first_name)
            .bind(last_name)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn allowed_users(&self, survey_id: u64) -> Result<Vec<(String, String)>> {
        Ok(sqlx::query_as(
            "SELECT first_name, last_name FROM SurveyAllowedUser WHERE survey_id = ? ORDER BY last_name, first_name",
        )
        .bind(survey_id)
        .fetch_all(&self.pool)
        .await?)
    }

    async fn set_pass_phrase(&self, survey_id: u64, pass_phrase: &str) -> Result<()> {
        sqlx::query(
            "INSERT INTO SurveyPassPhrase (survey_id, pass_phrase) VALUES (?, ?)
             ON DUPLICATE KEY UPDATE pass_phrase = VALUES(pass_phrase)",
        )
        .bind(survey_id)
        .bind(pass_phrase)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn survey_definition(&self, survey_id: u64) -> Result<Option<String>> {
        Ok(sqlx::query_scalar("SELECT definition FROM Survey WHERE survey_id = ?")
            .bind(survey_id)
            .fetch_optional(&self.pool)
            .await?)
    }

    async fn survey_definitions(&self) -> Result<Vec<String>> {
        Ok(sqlx::query_scalar("SELECT definition FROM Survey ORDER BY survey_id")
            .fetch_all(&self.pool)
            .await?)
    }

    async fn save_survey_definition(&self, survey_id: u64, definition: &str) -> Result<()> {
        sqlx::query(
            "INSERT INTO Survey (survey_id, definition) VALUES (?, ?)
             ON DUPLICATE KEY UPDATE definition = VALUES(definition)",
        )
        .bind(survey_id)
        .bind(definition)
        .execute(&self.pool)
        .await?;
        info!("Stored the definition of survey {survey_id}");
        Ok(())
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

/// The error of inserting an answer. A duplicate is caught by the `UNIQUE`
/// key on `(user_id, question_id)` when two submissions of the same user
/// race past the check in [`MysqlRepo::save_answers`].
fn insert_error(err: sqlx::Error) -> Error {
    match &err {
        sqlx::Error::Database(db_err) if db_err.is_unique_violation() => Error::AnswerExists,
        _ => err.into(),
    }
}

/// Storage format of an answer: `DD/MM/YYYY` for dates (`00` when there is
/// no day), the text itself otherwise.
fn to_db(answer: &Answer) -> String {
    match answer {
        Answer::Text(text) | Answer::Poll(text) => text.clone(),
        Answer::PredictionDate { day, month, year } => format!("{:02}/{month:02}/{year:04}", day.unwrap_or(0)),
    }
}

/// Inverse of [`to_db`] for an answer to a question of kind `kind`; dates
/// also accept `MM/YYYY`, and fall back to text when they do not parse.
fn from_db(text: String, kind: AnswerKind) -> Answer {
    match kind {
        AnswerKind::Text => Answer::Text(text),
        AnswerKind::Poll => Answer::Poll(text),
        AnswerKind::PredictionDate => date_from_db(text),
    }
}

fn date_from_db(text: String) -> Answer {
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
            assert_eq!(from_db(to_db(&answer), AnswerKind::PredictionDate), answer);
        }
        assert_eq!(to_db(&date(None, 5, 2025)), "00/05/2025");
        assert_eq!(to_db(&date(Some(15), 5, 2025)), "15/05/2025");
    }

    #[test]
    fn month_year_parses_as_date() {
        assert_eq!(
            from_db("05/2025".into(), AnswerKind::PredictionDate),
            date(None, 5, 2025)
        );
    }

    #[test]
    fn unparsable_dates_fall_back_to_text() {
        for text in ["Some random text", "13/2025", "00/2025", "1/2/3/4", "aa/bb/cccc", ""] {
            assert_eq!(
                from_db(text.into(), AnswerKind::PredictionDate),
                Answer::Text(text.into()),
                "{text}"
            );
        }
    }

    #[test]
    fn text_is_never_guessed_to_be_a_date() {
        for text in ["hello", "05/2025", "15/05/2025", ""] {
            let answer = Answer::Text(text.into());
            assert_eq!(to_db(&answer), text);
            assert_eq!(from_db(to_db(&answer), AnswerKind::Text), answer, "{text}");
        }
    }

    #[test]
    fn poll_answers_round_trip_even_when_they_look_like_dates() {
        for text in [
            "C++",
            "I can't make it",
            "05/2025",
            "15/05/2025",
            "13/2025",
            "other, 1/2/3/4",
        ] {
            let answer = Answer::Poll(text.into());
            assert_eq!(to_db(&answer), text);
            assert_eq!(from_db(to_db(&answer), AnswerKind::Poll), answer, "{text}");
        }
    }

    /// A database error of a given kind, as the driver would report it.
    #[derive(Debug)]
    struct FakeDbError {
        unique_violation: bool,
    }

    impl std::fmt::Display for FakeDbError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "fake database error (unique violation: {})", self.unique_violation)
        }
    }

    impl std::error::Error for FakeDbError {}

    impl sqlx::error::DatabaseError for FakeDbError {
        fn message(&self) -> &'static str {
            "fake database error"
        }

        fn as_error(&self) -> &(dyn std::error::Error + Send + Sync + 'static) {
            self
        }

        fn as_error_mut(&mut self) -> &mut (dyn std::error::Error + Send + Sync + 'static) {
            self
        }

        fn into_error(self: Box<Self>) -> Box<dyn std::error::Error + Send + Sync + 'static> {
            self
        }

        fn kind(&self) -> sqlx::error::ErrorKind {
            if self.unique_violation {
                sqlx::error::ErrorKind::UniqueViolation
            } else {
                sqlx::error::ErrorKind::Other
            }
        }
    }

    #[test]
    fn duplicate_insert_means_answer_exists() {
        let duplicate = sqlx::Error::Database(Box::new(FakeDbError { unique_violation: true }));
        assert_eq!(insert_error(duplicate), Error::AnswerExists);
    }

    #[test]
    fn other_insert_failures_stay_repository_errors() {
        let other_db = sqlx::Error::Database(Box::new(FakeDbError {
            unique_violation: false,
        }));
        assert!(matches!(insert_error(other_db), Error::Repository(_)));
        assert!(matches!(insert_error(sqlx::Error::PoolTimedOut), Error::Repository(_)));
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
