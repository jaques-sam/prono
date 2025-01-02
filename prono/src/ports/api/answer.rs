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
