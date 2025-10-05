use super::{Answer, Clear};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Question {
    pub id: String,
    pub answer: Answer,
    pub text: Option<String>,
}

impl Clear for Question {
    fn clear(&mut self) {
        self.answer.clear();
    }
}

impl Question {
    pub fn update(&mut self, proto_question: Question) -> &mut Self {
        self.answer = proto_question.answer;
        self.text = proto_question.text;
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
