use serde::{Deserialize, Serialize};

use super::{Answer, Clear};
use prono::api;

#[derive(Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct Question {
    pub id: String,
    pub answer: Answer,
    pub text: String,
}

impl Clear for Question {
    fn clear(&mut self) {
        self.answer.clear();
    }
}

impl Question {
    pub fn update(&mut self, proto_question: api::Question) -> &mut Self {
        self.answer = proto_question.answer.into();
        self
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
