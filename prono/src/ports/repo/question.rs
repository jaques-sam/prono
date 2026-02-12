use super::answer::Answer;

#[derive(Debug, PartialEq, Eq)]
pub struct Question {
    pub id: String,
    pub answer: Answer,
}

impl From<crate::Question> for Question {
    fn from(question: crate::Question) -> Self {
        Self {
            id: question.id,
            answer: question.answer.into(),
        }
    }
}

impl From<Question> for crate::Question {
    fn from(question: Question) -> Self {
        Self {
            id: question.id,
            answer: question.answer.into(),
            text: None,
        }
    }
}
