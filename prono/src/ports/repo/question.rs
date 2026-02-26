use super::answer::Answer;

#[derive(Debug, PartialEq, Eq)]
#[cfg_attr(any(debug_assertions, test), derive(Clone))]
#[cfg_attr(test, derive(Default))]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_domain_question_to_repo_question() {
        let domain_question = crate::Question {
            id: String::from("q1"),
            text: Some(String::from("Test question?")),
            answer: crate::Answer::Text(String::from("Test answer")),
        };

        let repo_question: Question = domain_question.into();
        assert_eq!(repo_question.id, "q1");
        assert_eq!(repo_question.answer, Answer::Text(String::from("Test answer")));
    }

    #[test]
    fn test_from_repo_question_to_domain_question() {
        let repo_question = Question {
            id: String::from("q1"),
            answer: Answer::PredictionDate {
                day: Some(15),
                month: 5,
                year: 2025,
            },
        };

        let domain_question: crate::Question = repo_question.into();
        assert_eq!(domain_question.id, "q1");
        assert_eq!(domain_question.text, None);
        assert_eq!(
            domain_question.answer,
            crate::Answer::PredictionDate {
                day: Some(15),
                month: 5,
                year: 2025
            }
        );
    }

    #[test]
    fn test_question_roundtrip_conversion() {
        let original = Question {
            id: String::from("test-id"),
            answer: Answer::Text(String::from("test")),
        };

        let domain: crate::Question = original.clone().into();
        let back: Question = domain.into();

        assert_eq!(original.id, back.id);
        assert_eq!(original.answer, back.answer);
    }
}
