use super::Answer;

#[derive(Debug, PartialEq, Eq)]
#[cfg_attr(feature = "test-utils", derive(Default))]
pub struct Question {
    pub id: String,
    pub answer: Answer,
    pub text: Option<String>,
}
