use super::answer::Answer;

#[derive(Debug, PartialEq, Eq)]
pub struct Question {
    pub id: String,
    pub text: String,
    pub answer: Answer,
}

impl From<crate::Question> for Question {
    fn from(question: crate::Question) -> Self {
        Self {
            id: question.id,
            text: question.text,
            answer: question.answer.into(),
        }
    }
}
