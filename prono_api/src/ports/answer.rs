#[derive(Debug, PartialEq, Eq)]
pub enum Answer {
    Text(String),
    PredictionDate { day: Option<u8>, month: u8, year: u16 },
}

#[cfg(feature = "test-utils")]
impl Default for Answer {
    fn default() -> Self {
        Answer::Text(String::new())
    }
}
