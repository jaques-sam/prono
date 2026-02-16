#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Answer {
    Text(String),
    PredictionDate { day: Option<u8>, month: u8, year: u16 },
}

#[cfg(test)]
impl Default for Answer {
    fn default() -> Self {
        Answer::PredictionDate {
            day: None,
            month: 8,
            year: 1986,
        }
    }
}

impl From<crate::Answer> for Answer {
    fn from(answer: crate::Answer) -> Self {
        match answer {
            crate::Answer::Text(text) => Answer::Text(text),
            crate::Answer::PredictionDate { day, month, year } => Answer::PredictionDate { day, month, year },
        }
    }
}

impl From<Answer> for crate::Answer {
    fn from(answer: Answer) -> Self {
        match answer {
            Answer::Text(text) => crate::Answer::Text(text),
            Answer::PredictionDate { day, month, year } => crate::Answer::PredictionDate { day, month, year },
        }
    }
}

impl From<String> for Answer {
    fn from(text: String) -> Self {
        if let Some((first_str, rest)) = text.split_once('/') {
            // Try DD/MM/YYYY format
            if let Some((month_str, year_str)) = rest.split_once('/')
                && let (Ok(day), Ok(month), Ok(year)) = (
                    first_str.parse::<u8>(),
                    month_str.parse::<u8>(),
                    year_str.parse::<u16>(),
                )
            {
                return Answer::PredictionDate {
                    day: if day == 0 { None } else { Some(day) },
                    month,
                    year,
                };
            }
            // Try MM/YYYY format
            else if let (Ok(month), Ok(year)) = (first_str.parse::<u8>(), rest.parse::<u16>())
                && (1..=12).contains(&month)
            {
                return Answer::PredictionDate { day: None, month, year };
            }
        }
        Answer::Text(text)
    }
}

impl std::fmt::Display for Answer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Answer::Text(text) => write!(f, "{text}"),
            Answer::PredictionDate { day, month, year } => {
                write!(f, "{:02}/{:02}/{:04}", day.unwrap_or(0), *month, *year)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_repo_answer_with_day_zero() {
        let answer = Answer::from("00/05/2025".to_string());
        assert_eq!(
            answer,
            Answer::PredictionDate {
                day: None,
                month: 5,
                year: 2025
            }
        );
    }

    #[test]
    fn test_parse_normal_date() {
        let answer = Answer::from("15/05/2025".to_string());
        assert_eq!(
            answer,
            Answer::PredictionDate {
                day: Some(15),
                month: 5,
                year: 2025
            }
        );
    }

    #[test]
    fn test_parse_month_year_only() {
        let answer = Answer::from("05/2025".to_string());
        assert_eq!(
            answer,
            Answer::PredictionDate {
                day: None,
                month: 5,
                year: 2025
            }
        );
    }

    #[test]
    fn test_display_format_with_no_day() {
        let answer = Answer::PredictionDate {
            day: None,
            month: 5,
            year: 2025,
        };
        assert_eq!(format!("{answer}"), "00/05/2025");
    }

    #[test]
    fn test_display_format_with_day() {
        let answer = Answer::PredictionDate {
            day: Some(15),
            month: 5,
            year: 2025,
        };
        assert_eq!(format!("{answer}"), "15/05/2025");
    }

    #[test]
    fn test_parse_text_answer() {
        let answer = Answer::from("Some random text".to_string());
        assert_eq!(answer, Answer::Text("Some random text".to_string()));
    }
}
