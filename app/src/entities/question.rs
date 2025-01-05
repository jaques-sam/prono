use serde::{Deserialize, Serialize};

use super::{Answer, Clear};
use prono::api;

#[derive(Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct Question {
    pub id: String,
    pub text: String,
    pub answer: Answer,
}

impl Clear for Question {
    fn clear(&mut self) {
        self.answer.clear();
    }
}

impl From<api::Question> for Question {
    fn from(proto_question: api::Question) -> Self {
        Question {
            id: proto_question.id,
            text: proto_question.text,
            answer: proto_question.answer.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clear() {
        let mut question = Question {
            answer: Answer::Text("John".to_string()),
            ..Default::default()
        };

        question.clear();

        assert_eq!(question, Question::default());
    }
}
