use super::Answer;

#[derive(Debug, PartialEq, Eq)]
#[cfg_attr(feature = "test-utils", derive(Default))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Question {
    pub id: String,
    pub answer: Answer,
    pub text: Option<String>,
}
