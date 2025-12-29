use serde::{Deserialize, Serialize};

use super::Answer;

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(test, derive(Default))]
pub struct Question {
    pub id: String,
    pub answer: Answer,
    pub text: String,
}

impl Question {
    pub fn update(&mut self, proto_question: prono::Question) -> &mut Self {
        self.answer = proto_question.answer.into();
        self
    }
}

impl From<Question> for prono::Question {
    fn from(question: Question) -> Self {
        Self {
            id: question.id,
            answer: question.answer.into(),
            text: Some(question.text),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update() {
        // Arrange
        let mut question = Question {
            id: "1".to_string(),
            answer: Answer::default(),
            text: "What is your favorite color?".to_string(),
        };

        let proto_question = prono::Question {
            answer: prono::Answer::default(),
            ..Default::default()
        };

        // Act
        question.update(proto_question);

        // Assert
        assert_eq!(question.answer, Answer::default());
    }
}
