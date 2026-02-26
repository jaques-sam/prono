use serde::{Deserialize, Serialize};

use super::Question;

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(test, derive(Default))]
pub struct Survey {
    pub id: u64,
    pub description: String,
    pub questions: Vec<Question>,
}

impl From<Survey> for prono_api::Survey {
    fn from(survey: Survey) -> Self {
        Self {
            id: survey.id,
            description: survey.description,
            questions: survey.questions.into_iter().map(prono_api::Question::from).collect(),
        }
    }
}

impl From<prono_api::Survey> for Survey {
    fn from(proto_survey: prono_api::Survey) -> Self {
        Self {
            id: proto_survey.id,
            description: proto_survey.description,
            questions: proto_survey
                .questions
                .into_iter()
                .map(|q| Question {
                    id: q.id,
                    answer: q.answer.into(),
                    text: q.text.unwrap_or_default(),
                })
                .collect(),
        }
    }
}

impl Survey {
    pub(crate) fn empty(&mut self) {
        for question in &mut self.questions {
            question.answer.empty();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Answer;

    #[test]
    fn test_empty_clears_all_answers() {
        let mut survey = Survey {
            id: 1,
            description: "Test".to_string(),
            questions: vec![
                Question {
                    id: "q1".to_string(),
                    answer: Answer::Text("answer1".to_string()),
                    text: "Q1?".to_string(),
                },
                Question {
                    id: "q2".to_string(),
                    answer: Answer::Text("answer2".to_string()),
                    text: "Q2?".to_string(),
                },
            ],
        };

        survey.empty();

        assert_eq!(survey.questions[0].answer, Answer::Text(String::new()));
        assert_eq!(survey.questions[1].answer, Answer::Text(String::new()));
    }

    #[test]
    fn test_empty_preserves_question_metadata() {
        let mut survey = Survey {
            id: 42,
            description: "Survey description".to_string(),
            questions: vec![Question {
                id: "q1".to_string(),
                answer: Answer::Text("to clear".to_string()),
                text: "Question text".to_string(),
            }],
        };

        survey.empty();

        assert_eq!(survey.id, 42);
        assert_eq!(survey.description, "Survey description");
        assert_eq!(survey.questions[0].id, "q1");
        assert_eq!(survey.questions[0].text, "Question text");
    }

    #[test]
    fn test_from_prono_api_survey() {
        let api_survey = prono_api::Survey {
            id: 99,
            description: "API Survey".to_string(),
            questions: vec![prono_api::Question {
                id: "api-q".to_string(),
                text: Some("API Question?".to_string()),
                answer: prono_api::Answer::Text("api answer".to_string()),
            }],
        };

        let survey: Survey = api_survey.into();

        assert_eq!(survey.id, 99);
        assert_eq!(survey.description, "API Survey");
        assert_eq!(survey.questions.len(), 1);
        assert_eq!(survey.questions[0].id, "api-q");
        assert_eq!(survey.questions[0].text, "API Question?");
    }

    #[test]
    fn test_from_prono_api_survey_with_no_text() {
        let api_survey = prono_api::Survey {
            id: 1,
            description: "Test".to_string(),
            questions: vec![prono_api::Question {
                id: "q".to_string(),
                text: None,
                answer: prono_api::Answer::Text(String::new()),
            }],
        };

        let survey: Survey = api_survey.into();

        assert_eq!(survey.questions[0].text, "");
    }

    #[test]
    fn test_into_prono_api_survey() {
        let survey = Survey {
            id: 123,
            description: "Test Survey".to_string(),
            questions: vec![Question {
                id: "q1".to_string(),
                answer: Answer::PredictionDate {
                    day: Some(1),
                    month: 1,
                    year: 2025,
                },
                text: "When?".to_string(),
            }],
        };

        let api_survey: prono_api::Survey = survey.into();

        assert_eq!(api_survey.id, 123);
        assert_eq!(api_survey.description, "Test Survey");
        assert_eq!(api_survey.questions.len(), 1);
        assert_eq!(api_survey.questions[0].text, Some("When?".to_string()));
    }

    #[test]
    fn test_survey_default() {
        let survey = Survey::default();
        assert_eq!(survey.id, 0);
        assert_eq!(survey.description, "");
        assert!(survey.questions.is_empty());
    }
}
