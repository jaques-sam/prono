use serde::{Deserialize, Serialize};

use super::{Answer, Clear};

#[derive(Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct Question {
    pub id: u16,
    pub question: String,
    pub answer: Answer,
}

impl Clear for Question {
    fn clear(&mut self) {
        self.answer.clear();
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

        assert_eq!(question, Default::default());
    }
}
