mod answer;
mod question;
mod survey;

pub use answer::*;
pub use question::*;
pub use survey::*;

pub trait Surveys {
    fn answer(&self, user: &str, question_id: u64) -> Answer;
    fn response(&self, user: &str, survey_id: u64) -> Option<Survey>;
    fn add_answer(&mut self, user: &str, question_id: String, answer: Answer);
}
