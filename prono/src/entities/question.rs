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

impl From<prono_api::Question> for Question {
    fn from(question: prono_api::Question) -> Self {
        Self {
            id: question.id,
            text: question.text,
            answer: question.answer.into(),
        }
    }
}

impl From<Question> for prono_api::Question {
    fn from(question: Question) -> Self {
        Self {
            id: question.id,
            text: question.text,
            answer: question.answer.into(),
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

    #[test]
    fn test_update_changes_answer_and_text() {
        let mut question = Question {
            id: "q1".to_string(),
            answer: Answer::Text("old".to_string()),
            text: Some("old text".to_string()),
        };

        let proto = Question {
            id: "ignored".to_string(),
            answer: Answer::Text("new".to_string()),
            text: Some("new text".to_string()),
        };

        question.update(proto);

        assert_eq!(question.id, "q1"); // id unchanged
        assert_eq!(question.answer, Answer::Text("new".to_string()));
        assert_eq!(question.text, Some("new text".to_string()));
    }

    #[test]
    fn test_update_returns_mutable_reference() {
        let mut question = Question::default();
        let proto = Question {
            answer: Answer::Text("chained".to_string()),
            ..Default::default()
        };

        let result = question.update(proto);
        assert_eq!(result.answer, Answer::Text("chained".to_string()));
    }

    #[test]
    fn test_from_prono_api_question() {
        let api_question = prono_api::Question {
            id: "api-q1".to_string(),
            text: Some("API question?".to_string()),
            answer: prono_api::Answer::Text("api answer".to_string()),
        };

        let question: Question = api_question.into();

        assert_eq!(question.id, "api-q1");
        assert_eq!(question.text, Some("API question?".to_string()));
        assert_eq!(question.answer, Answer::Text("api answer".to_string()));
    }

    #[test]
    fn test_into_prono_api_question() {
        let question = Question {
            id: "q1".to_string(),
            text: Some("Question?".to_string()),
            answer: Answer::PredictionDate {
                day: Some(10),
                month: 5,
                year: 2025,
            },
        };

        let api_question: prono_api::Question = question.into();

        assert_eq!(api_question.id, "q1");
        assert_eq!(api_question.text, Some("Question?".to_string()));
        assert_eq!(
            api_question.answer,
            prono_api::Answer::PredictionDate {
                day: Some(10),
                month: 5,
                year: 2025
            }
        );
    }

    #[test]
    fn test_question_roundtrip_conversion() {
        let original = Question {
            id: "roundtrip".to_string(),
            text: Some("Test?".to_string()),
            answer: Answer::Text("test".to_string()),
        };

        let api: prono_api::Question = original.clone().into();
        let back: Question = api.into();

        assert_eq!(original, back);
    }
}
