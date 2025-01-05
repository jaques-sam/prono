use prono::api;
use serde::{Deserialize, Serialize};

use chrono::{
    Datelike, {DateTime, Utc},
};

fn datetime() -> DateTime<Utc> {
    let now = std::time::SystemTime::now();
    now.into()
}

pub trait Clear {
    fn clear(&mut self);
}

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

impl Clear for Answer {
    fn clear(&mut self) {
        match self {
            Answer::Text(text) => text.clear(),
            Answer::PredictionDate { day, month, year } => {
                let dt = datetime();
                *day = None;
                *month = dt.month() as u8;
                *year = dt.year() as u16;
            }
        }
    }
}

impl From<api::Answer> for Answer {
    fn from(proto_answer: api::Answer) -> Self {
        match proto_answer {
            api::Answer::Text(text) => Answer::Text(text),
            api::Answer::PredictionDate { day, month, year } => Answer::PredictionDate { day, month, year },
        }
    }
}

impl From<&Answer> for api::Answer {
    fn from(proto_answer: &Answer) -> Self {
        match proto_answer {
            Answer::Text(text) => api::Answer::Text(text.clone()),
            Answer::PredictionDate { day, month, year } => api::Answer::PredictionDate {
                day: *day,
                month: *month,
                year: *year,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clear_text_answer() {
        let mut answer = Answer::Text("John".to_string());
        answer.clear();
        assert_eq!(answer, Answer::Text("".to_string()));
    }

    #[test]
    fn test_datetime_on_clearing_prediction_date() {
        let mut prediction = Answer::PredictionDate {
            day: Some(10),
            month: 1,
            year: 2150,
        };

        prediction.clear();

        assert_eq!(
            prediction,
            Answer::PredictionDate {
                day: None,
                month: datetime().month() as u8,
                year: datetime().year() as u16,
            }
        );
    }
}
