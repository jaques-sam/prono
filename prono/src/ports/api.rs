#[derive(Debug, PartialEq, Eq)]
pub enum Answer {
    Text(String),
    PredictionDate { day: Option<u8>, month: u8, year: u16 },
}

#[derive(Debug, PartialEq, Eq)]
pub struct Question {
    pub id: String,
    pub question: String,
    pub answer: Answer,
}

#[derive(Debug, PartialEq, Eq)]
#[cfg_attr(test, derive(Default))]
pub struct Survey {
    pub id: u64,
    pub description: String,
    pub questions: Vec<Question>,
}

pub trait PronoApi {
    fn survey(&self) -> Survey;
    fn answer(&self, user: u64, id: u16) -> Answer;
}
