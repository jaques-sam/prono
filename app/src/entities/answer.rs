use chrono::Datelike;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
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

impl From<crate::Answer> for prono_api::Answer {
    fn from(a: crate::Answer) -> Self {
        match a {
            crate::Answer::Text(s) => prono_api::Answer::Text(s), // only inner String cloned
            crate::Answer::PredictionDate { day, month, year } => {
                prono_api::Answer::PredictionDate { day, month, year }
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::match_wildcard_for_single_variants)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_text_answer() {
        let mut answer = Answer::Text("some text".to_string());
        answer.empty();
        assert_eq!(answer, Answer::Text(String::new()));
    }

    #[test]
    fn test_empty_prediction_date_resets_to_current() {
        let mut answer = Answer::PredictionDate {
            day: Some(15),
            month: 1,
            year: 2000,
        };
        answer.empty();
        match answer {
            Answer::PredictionDate { day, month, year } => {
                assert_eq!(day, None);
                // month and year should be current date
                let now = chrono::Utc::now();
                assert_eq!(month, u8::try_from(now.month()).unwrap_or_default());
                assert_eq!(year, u16::try_from(now.year()).unwrap_or_default());
            }
            _ => panic!("Expected PredictionDate"),
        }
    }

    #[test]
    fn test_from_prono_api_text_answer() {
        let api_answer = prono_api::Answer::Text("test".to_string());
        let answer: Answer = api_answer.into();
        assert_eq!(answer, Answer::Text("test".to_string()));
    }

    #[test]
    fn test_from_prono_api_prediction_date() {
        let api_answer = prono_api::Answer::PredictionDate {
            day: Some(10),
            month: 5,
            year: 2025,
        };
        let answer: Answer = api_answer.into();
        assert_eq!(
            answer,
            Answer::PredictionDate {
                day: Some(10),
                month: 5,
                year: 2025
            }
        );
    }

    #[test]
    fn test_into_prono_api_text_answer() {
        let answer = Answer::Text("hello".to_string());
        let api_answer: prono_api::Answer = answer.into();
        assert_eq!(api_answer, prono_api::Answer::Text("hello".to_string()));
    }

    #[test]
    fn test_into_prono_api_prediction_date() {
        let answer = Answer::PredictionDate {
            day: None,
            month: 12,
            year: 2030,
        };
        let api_answer: prono_api::Answer = answer.into();
        assert_eq!(
            api_answer,
            prono_api::Answer::PredictionDate {
                day: None,
                month: 12,
                year: 2030
            }
        );
    }

    #[test]
    fn test_default_answer() {
        let answer = Answer::default();
        assert_eq!(answer, Answer::Text(String::new()));
    }

    #[test]
    fn test_answer_clone() {
        let original = Answer::PredictionDate {
            day: Some(5),
            month: 6,
            year: 2025,
        };
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    #[test]
    fn test_answer_equality() {
        let a1 = Answer::Text("same".to_string());
        let a2 = Answer::Text("same".to_string());
        let a3 = Answer::Text("different".to_string());
        assert_eq!(a1, a2);
        assert_ne!(a1, a3);
    }
}
