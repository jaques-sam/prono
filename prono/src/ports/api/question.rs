use super::answer::Answer;

#[derive(Debug, PartialEq, Eq)]
pub struct Question {
    pub id: String,
    pub question: String,
    pub answer: Answer,
}

impl From<crate::Question> for Question {
    fn from(question: crate::Question) -> Self {
        Self {
            id: question.id,
            question: question.question,
            answer: question.answer.into(),
        }
    }
}
