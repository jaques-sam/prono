#[derive(Debug, PartialEq, Eq)]
pub enum Answer {
    Text(String),
    PredictionDate { day: Option<u8>, month: u8, year: u16 },
}

impl From<crate::Answer> for Answer {
    fn from(answer: crate::Answer) -> Self {
        match answer {
            crate::Answer::Text(text) => Answer::Text(text),
            crate::Answer::PredictionDate { day, month, year } => Answer::PredictionDate { day, month, year },
        }
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
