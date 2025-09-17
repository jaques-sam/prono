use crate::{Answer, Survey};

pub trait Prono {
    fn empty_survey(&self) -> Survey;
    fn filled_survey(&self, user: &str, survey_id: u64) -> Option<Survey>;
    fn add_answer(&mut self, user: &str, question_id: String, answer: Answer);
    fn response(&self, user: &str, id: u64) -> Option<Survey>;
}
