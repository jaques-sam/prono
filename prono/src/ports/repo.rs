mod answer;
mod question;
mod survey;

pub use answer::*;
pub use question::*;
pub use survey::*;

use async_trait::async_trait;

use crate::PronoResult;

#[async_trait]
pub trait Surveys: Send + Sync {
    async fn answer(&self, user: &str, question_id: String) -> Option<Answer>;
    async fn response(&self, user: &str, survey_id: u64) -> Option<Survey>;
    async fn add_answer(&self, user: &str, question_id: String, answer: Answer) -> PronoResult<()>;
    async fn all_answers(&self, question_id: String) -> Vec<(String, Answer)>;
}

#[async_trait]
pub trait Db: Surveys + Sized + Send + Sync {
    /// Associated config type required to initialize this DB implementation.
    type Config: Send + 'static;

    /// Initialize the DB instance from the provided config. Runs on an
    /// async runtime and returns the constructed DB instance.
    async fn init(config: Self::Config) -> PronoResult<Self>;
}
