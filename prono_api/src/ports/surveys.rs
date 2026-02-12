use crate::{Answer, Survey};

pub trait Surveys {
    fn empty_survey(&self) -> Survey;
    fn add_answer(&mut self, user: &str, question_id: String, answer: Answer);
    #[must_use]
    fn response(&self, user: &str, survey_id: u64) -> Option<Survey>;
    fn all_answers(&self, question_id: String) -> Vec<(String, Answer)>;
}
