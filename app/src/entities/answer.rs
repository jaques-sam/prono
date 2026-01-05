use chrono::Datelike;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
pub enum Answer {
    Text(String),
    PredictionDate { day: Option<u8>, month: u8, year: u16 },
}

impl Answer {
    pub fn empty(&mut self) {
        match self {
            Answer::Text(text) => text.clear(),
            Answer::PredictionDate { day, month, year } => {
                *day = None;
                *month = u8::try_from(chrono::Utc::now().month()).unwrap_or_default();
                *year = u16::try_from(chrono::Utc::now().year()).unwrap_or_default();
            }
        }
    }
}

#[cfg(test)]
impl Default for Answer {
    fn default() -> Self {
        Answer::Text(String::new())
    }
}

impl From<prono_api::Answer> for Answer {
    fn from(proto_answer: prono_api::Answer) -> Self {
        match proto_answer {
            prono_api::Answer::Text(text) => Answer::Text(text),
            prono_api::Answer::PredictionDate { day, month, year } => Answer::PredictionDate { day, month, year },
        }
    }
}

impl From<Answer> for prono_api::Answer {
    fn from(proto_answer: Answer) -> Self {
        match proto_answer {
            Answer::Text(text) => prono_api::Answer::Text(text),
            Answer::PredictionDate { day, month, year } => prono_api::Answer::PredictionDate { day, month, year },
        }
    }
}
