use async_trait::async_trait;
use prono_api::{Answer, UserAnswer};
use uuid::Uuid;

use crate::Result;

/// Where participants and their answers are stored.
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

    /// Every participant's answer to the question `question_id`.
    async fn answers(&self, question_id: Uuid) -> Result<Vec<UserAnswer>>;

    /// The names of all participants.
    async fn users(&self) -> Result<Vec<String>>;

    /// Deletes `user` and all their answers.
    async fn delete_user(&self, user: &str) -> Result<()>;
}
