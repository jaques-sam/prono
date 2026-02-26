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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Answer {
    Text(String),
    PredictionDate { day: Option<u8>, month: u8, year: u16 },
}

impl Answer {
    #[must_use]
    pub fn new_text() -> Answer {
        Answer::Text(String::default())
    }

    /// # Panics
    ///
    /// Panics if the current month or year cannot be converted to u8 or u16 respectively.
    #[must_use]
    pub fn new_prediction_date() -> Answer {
        // TODO: use time provider
        let dt = datetime();
        Answer::PredictionDate {
            day: None,
            month: u8::try_from(dt.month()).expect("invalid month"),
            year: u16::try_from(dt.year()).expect("invalid year"),
        }
    }
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
                *month = u8::try_from(dt.month()).expect("invalid month");
                *year = u16::try_from(dt.year()).expect("invalid year");
            }
        }
    }
}

impl From<prono_api::Answer> for Answer {
    fn from(answer: prono_api::Answer) -> Self {
        match answer {
            prono_api::Answer::Text(text) => Answer::Text(text),
            prono_api::Answer::PredictionDate { day, month, year } => Answer::PredictionDate { day, month, year },
        }
    }
}

impl From<Answer> for prono_api::Answer {
    fn from(answer: Answer) -> Self {
        match answer {
            Answer::Text(text) => prono_api::Answer::Text(text),
            Answer::PredictionDate { day, month, year } => prono_api::Answer::PredictionDate { day, month, year },
        }
    }
}

#[cfg(test)]
#[allow(clippy::match_wildcard_for_single_variants)]
mod tests {
    use super::*;

    #[test]
    fn test_clear_text_answer() {
        let mut answer = Answer::Text("John".to_string());
        answer.clear();
        assert_eq!(answer, Answer::Text(String::new()));
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
                month: u8::try_from(datetime().month()).expect("invalid month"),
                year: u16::try_from(datetime().year()).expect("invalid year"),
            }
        );
    }

    #[test]
    fn test_new_text_creates_empty_string() {
        let answer = Answer::new_text();
        assert_eq!(answer, Answer::Text(String::new()));
    }

    #[test]
    fn test_new_prediction_date_uses_current_date() {
        let answer = Answer::new_prediction_date();
        let dt = datetime();
        match answer {
            Answer::PredictionDate { day, month, year } => {
                assert_eq!(day, None);
                assert_eq!(month, u8::try_from(dt.month()).unwrap());
                assert_eq!(year, u16::try_from(dt.year()).unwrap());
            }
            _ => panic!("Expected PredictionDate variant"),
        }
    }

    #[test]
    fn test_default_is_empty_text() {
        let answer = Answer::default();
        assert_eq!(answer, Answer::Text(String::new()));
    }

    #[test]
    fn test_from_prono_api_text_answer() {
        let api_answer = prono_api::Answer::Text("test".to_string());
        let answer: Answer = api_answer.into();
        assert_eq!(answer, Answer::Text("test".to_string()));
    }

    #[test]
    fn test_from_prono_api_prediction_date_answer() {
        let api_answer = prono_api::Answer::PredictionDate {
            day: Some(15),
            month: 6,
            year: 2025,
        };
        let answer: Answer = api_answer.into();
        assert_eq!(
            answer,
            Answer::PredictionDate {
                day: Some(15),
                month: 6,
                year: 2025
            }
        );
    }

    #[test]
    fn test_into_prono_api_text_answer() {
        let answer = Answer::Text("test".to_string());
        let api_answer: prono_api::Answer = answer.into();
        assert_eq!(api_answer, prono_api::Answer::Text("test".to_string()));
    }

    #[test]
    fn test_into_prono_api_prediction_date_answer() {
        let answer = Answer::PredictionDate {
            day: Some(15),
            month: 6,
            year: 2025,
        };
        let api_answer: prono_api::Answer = answer.into();
        assert_eq!(
            api_answer,
            prono_api::Answer::PredictionDate {
                day: Some(15),
                month: 6,
                year: 2025
            }
        );
    }

    #[test]
    fn test_answer_roundtrip_conversion() {
        let original = Answer::PredictionDate {
            day: None,
            month: 12,
            year: 2030,
        };
        let api: prono_api::Answer = original.clone().into();
        let back: Answer = api.into();
        assert_eq!(original, back);
    }
}
