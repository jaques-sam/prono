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
    async fn answer(&self, user: &str, question_id: u64) -> Option<Answer>;
    async fn response(&self, user: &str, survey_id: u64) -> Option<Survey>;
    async fn add_answer(&self, user: &str, question_id: String, answer: Answer) -> PronoResult<()>;
}
