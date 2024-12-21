use crate::Answer;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum DbAnswer {
    Text(String),
    PredictionDate { day: Option<u8>, month: u8, year: u16 },
}

impl From<Answer> for DbAnswer {
    fn from(answer: Answer) -> Self {
        match answer {
            Answer::Text(text) => DbAnswer::Text(text),
            Answer::PredictionDate { day, month, year } => DbAnswer::PredictionDate { day, month, year },
        }
    }
}
