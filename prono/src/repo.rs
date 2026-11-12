use async_trait::async_trait;
use prono_api::{Answer, Question, UserAnswer};
use uuid::Uuid;

use crate::Result;

/// Where surveys, participants and their answers are stored.
///
/// A participant is identified by name and bound to the device that first
/// submitted for that name.
// `async_trait` marks its boxed futures `#[must_use]` twice.
#[allow(clippy::double_must_use)]
#[async_trait]
pub trait Repo: Send + Sync {
    /// The device registered for `user`, if the user exists.
    async fn device_of(&self, user: &str) -> Result<Option<String>>;

    /// The user registered on `device_id`, if any.
    async fn user_of(&self, device_id: &str) -> Result<Option<String>>;

    /// Registers `user` on `device_id` (if new) and stores `answers`, all or
    /// nothing.
    ///
    /// # Errors
    ///
    /// [`crate::Error::AnswerExists`] if `user` already answered one of the
    /// questions.
    async fn save_answers(&self, user: &str, device_id: &str, answers: &[(Uuid, Answer)]) -> Result<()>;

    /// Every participant's answer to `question`. Answers are stored as text,
    /// so the question's kind tells how to decode them.
    async fn answers(&self, question: &Question) -> Result<Vec<UserAnswer>>;

    /// The ids of the surveys whose name list holds `user`, given as the
    /// participant's [`prono_api::full_name`] (compared like user names).
    async fn listed_surveys(&self, user: &str) -> Result<Vec<u64>>;

    /// Puts a participant on the name list of survey `survey_id`; a no-op if
    /// already there.
    async fn allow_user(&self, survey_id: u64, first_name: &str, last_name: &str) -> Result<()>;

    /// Takes a participant off the name list of survey `survey_id`.
    async fn disallow_user(&self, survey_id: u64, first_name: &str, last_name: &str) -> Result<()>;

    /// The `(first name, last name)` pairs on the name list of survey
    /// `survey_id`.
    async fn allowed_users(&self, survey_id: u64) -> Result<Vec<(String, String)>>;

    /// The pass phrase of every survey that has one, as `(survey id, pass
    /// phrase)` pairs. Compare them exactly: the database's collation would
    /// ignore case.
    async fn pass_phrases(&self) -> Result<Vec<(u64, String)>>;

    /// Sets, or replaces, the pass phrase of survey `survey_id`.
    async fn set_pass_phrase(&self, survey_id: u64, pass_phrase: &str) -> Result<()>;

    /// The stored definition (the JSON text, see `prono_api::Survey::from_json`)
    /// of survey `survey_id`, if it was uploaded.
    async fn survey_definition(&self, survey_id: u64) -> Result<Option<String>>;

    /// The stored definitions of all surveys.
    async fn survey_definitions(&self) -> Result<Vec<String>>;

    /// Stores the definition of survey `survey_id`, replacing an earlier one.
    async fn save_survey_definition(&self, survey_id: u64, definition: &str) -> Result<()>;

    /// The names of all participants.
    async fn users(&self) -> Result<Vec<String>>;

    /// Deletes `user` and all their answers.
    async fn delete_user(&self, user: &str) -> Result<()>;
}
