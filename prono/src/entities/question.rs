use super::{Answer, Clear};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
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
