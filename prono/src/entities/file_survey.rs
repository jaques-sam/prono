use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A survey retrieved from file, which can be seen as a template and does not contain any answers
#[derive(Serialize, Deserialize, Debug)]
pub struct FileSurvey {
    #[serde(rename = "survey_id")]
    id: u64,
    description: String,
    questions: Vec<Question>,
}

impl FileSurvey {
    /// # Panics
    ///
    /// Panics if the provided JSON string cannot be parsed into a `FileSurvey`.
    /// TODO: have proper error handling instead of panicking.
    #[must_use]
    pub fn create_from_file(json_body: &str) -> Self {
        serde_json::from_str(json_body)
            .unwrap_or_else(|err| panic!("Unable to parse survey configuration (err: {err})"))
    }
}

impl From<FileSurvey> for crate::Survey {
    fn from(survey: FileSurvey) -> Self {
        Self {
            id: survey.id,
            description: survey.description,
            questions: survey.questions.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct Question {
    question: String,
    answer_type: AnswerType,
}

impl From<Question> for crate::Question {
    fn from(question: Question) -> Self {
        let data_to_hash = format!("{}{:?}", question.question, question.answer_type);
        let id = Uuid::new_v5(&Uuid::NAMESPACE_DNS, data_to_hash.as_bytes()).to_string();

        Self {
            id,
            text: Some(question.question),
            answer: match question.answer_type {
                AnswerType::Text => crate::Answer::new_text(),
                AnswerType::PredictionDate => crate::Answer::new_prediction_date(),
            },
        }
    }
}

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AnswerType {
    Text,
    PredictionDate,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use serde_json::json;

    #[test]
    fn test_configuration_from_file() {
        let json_data = include_str!("../surveys/survey_spacex_starship.json");

        let survey: FileSurvey = serde_json::from_str(json_data).unwrap();
        println!("{survey:?}");
    }

    #[test]
    fn test_survey_creation_from_valid_json() {
        let json_data = json!(
        {
            "survey_id": 1,
            "description": "Test Survey",
            "questions": [
                {
                    "question": "What is your name?",
                    "answer_type": "text"
                },
                {
                    "question": "When is the event?",
                    "answer_type": "prediction_date"
                }
            ]
        });

        let survey: FileSurvey = serde_json::from_value(json_data).unwrap();
        assert_eq!(survey.id, 1);
        assert_eq!(survey.description, "Test Survey");
        assert_eq!(survey.questions.len(), 2);
        assert_eq!(survey.questions[0].question, "What is your name?");
        assert_eq!(survey.questions[0].answer_type, AnswerType::Text);
        assert_eq!(survey.questions[1].question, "When is the event?");
        assert_eq!(survey.questions[1].answer_type, AnswerType::PredictionDate);
    }

    #[test]
    #[should_panic(expected = "Unable to parse survey configuration")]
    fn test_survey_creation_from_invalid_json() {
        let invalid_json_data = r#"
        {
            "survey_id": "invalid_id",
            "description": "Test Survey",
            "questions": []
        }
        "#;

        let _ = FileSurvey::create_from_file(invalid_json_data);
    }

    #[test]
    fn test_question_conversion() {
        let config_question = Question {
            question: String::from("What is your name?"),
            answer_type: AnswerType::Text,
        };

        let question: crate::Question = config_question.into();
        assert_eq!(question.text.unwrap(), "What is your name?");
        assert_eq!(question.answer, crate::Answer::new_text());
    }

    #[test]
    fn test_survey_conversion() {
        let config_survey = FileSurvey {
            id: 1,
            description: String::from("Test Survey"),
            questions: vec![
                Question {
                    question: String::from("What is your name?"),
                    answer_type: AnswerType::Text,
                },
                Question {
                    question: String::from("When is the event?"),
                    answer_type: AnswerType::PredictionDate,
                },
            ],
        };

        let survey: crate::Survey = config_survey.into();
        assert_eq!(survey.id, 1);
        assert_eq!(survey.description, "Test Survey");
        assert_eq!(survey.questions.len(), 2);
    }

    #[test]
    fn test_config_question_id_conversion_is_identical() {
        let config_question_json = json!({
            "question": "What is your name?",
            "answer_type": "text"
        });
        let config_question_json: Question = serde_json::from_value(config_question_json).unwrap();

        let config_question_obj = Question {
            question: String::from("What is your name?"),
            answer_type: AnswerType::Text,
        };

        let question_from_obj: crate::Question = config_question_obj.into();
        let question_from_json: crate::Question = config_question_json.into();
        assert_eq!(question_from_obj.id, question_from_json.id);
        assert_eq!(question_from_obj, question_from_json); // sanity check
    }

    #[rstest]
    #[case("What is your name", AnswerType::Text)]
    #[case("What is your name.", AnswerType::Text)]
    #[case("What is your Name?", AnswerType::Text)]
    #[case("What is your name?", AnswerType::PredictionDate)]
    #[case("", AnswerType::Text)]
    fn test_config_question_id_conversion_is_different(#[case] question: &str, #[case] answer_type: AnswerType) {
        let config_question_json = json!({
            "question": "What is your name?",
            "answer_type": "text"
        });
        let config_question_json: Question = serde_json::from_value(config_question_json).unwrap();

        let config_question_obj = Question {
            question: question.to_string(),
            answer_type,
        };

        let question_from_obj: crate::Question = config_question_obj.into();
        let question_from_json: crate::Question = config_question_json.into();
        assert_ne!(question_from_obj.id, question_from_json.id);
        assert_ne!(question_from_obj, question_from_json); // sanity check
    }
}
