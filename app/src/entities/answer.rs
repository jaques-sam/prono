use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
pub enum Answer {
    Text(String),
    PredictionDate { day: Option<u8>, month: u8, year: u16 },
}

impl Default for Answer {
    fn default() -> Self {
        Answer::Text(String::new())
    }
}

impl From<prono::Answer> for Answer {
    fn from(proto_answer: prono::Answer) -> Self {
        match proto_answer {
            prono::Answer::Text(text) => Answer::Text(text),
            prono::Answer::PredictionDate { day, month, year } => Answer::PredictionDate { day, month, year },
        }
    }
}

impl From<Answer> for prono::Answer {
    fn from(proto_answer: Answer) -> Self {
        match proto_answer {
            Answer::Text(text) => prono::Answer::Text(text),
            Answer::PredictionDate { day, month, year } => prono::Answer::PredictionDate { day, month, year },
        }
    }
}
