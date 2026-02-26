use crate::Clear;

use super::Question;

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(test, derive(Default))]
pub struct Survey {
    pub id: u64,
    pub description: String,
    pub questions: Vec<Question>,
}

impl Survey {
    pub fn clear(&mut self) {
        self.questions.iter_mut().for_each(Question::clear);
    }

    pub fn update_questions(&mut self, new_questions: Vec<Question>) {
        for (i, new_question) in new_questions.into_iter().enumerate() {
            if let Some(question) = self.questions.get_mut(i) {
                question.update(new_question);
            }
        }
    }
}

impl From<prono_api::Survey> for Survey {
    fn from(survey: prono_api::Survey) -> Self {
        Self {
            id: survey.id,
            description: survey.description,
            questions: survey.questions.into_iter().map(Question::from).collect(),
        }
    }
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

#[cfg(test)]
mod tests {
    use crate::Answer;

    use super::*;

    #[test]
    fn test_clearing_text_answers() {
        let mut survey = Survey {
            questions: vec![
                Question {
                    answer: Answer::Text("Sam".to_string()),
                    ..Default::default()
                },
                Question {
                    answer: Answer::Text("Kevin".to_string()),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        survey.clear();

        assert_eq!(
            survey,
            Survey {
                questions: vec![Question::default(), Question::default(),],
                ..Default::default()
            }
        );
    }

    #[test]
    fn test_update_questions_updates_existing_questions() {
        let mut survey = Survey {
            id: 1,
            description: "Test".to_string(),
            questions: vec![
                Question {
                    id: "q1".to_string(),
                    answer: Answer::Text("old1".to_string()),
                    text: Some("Q1?".to_string()),
                },
                Question {
                    id: "q2".to_string(),
                    answer: Answer::Text("old2".to_string()),
                    text: Some("Q2?".to_string()),
                },
            ],
        };

        let new_questions = vec![
            Question {
                id: "ignored".to_string(),
                answer: Answer::Text("new1".to_string()),
                text: Some("New Q1?".to_string()),
            },
            Question {
                id: "also_ignored".to_string(),
                answer: Answer::Text("new2".to_string()),
                text: Some("New Q2?".to_string()),
            },
        ];

        survey.update_questions(new_questions);

        assert_eq!(survey.questions[0].id, "q1"); // id unchanged
        assert_eq!(survey.questions[0].answer, Answer::Text("new1".to_string()));
        assert_eq!(survey.questions[0].text, Some("New Q1?".to_string()));
        assert_eq!(survey.questions[1].id, "q2"); // id unchanged
        assert_eq!(survey.questions[1].answer, Answer::Text("new2".to_string()));
    }

    #[test]
    fn test_update_questions_with_fewer_new_questions() {
        let mut survey = Survey {
            questions: vec![
                Question {
                    id: "q1".to_string(),
                    answer: Answer::Text("old1".to_string()),
                    text: Some("Q1?".to_string()),
                },
                Question {
                    id: "q2".to_string(),
                    answer: Answer::Text("old2".to_string()),
                    text: Some("Q2?".to_string()),
                },
            ],
            ..Default::default()
        };

        let new_questions = vec![Question {
            id: "ignored".to_string(),
            answer: Answer::Text("new1".to_string()),
            text: Some("New Q1?".to_string()),
        }];

        survey.update_questions(new_questions);

        assert_eq!(survey.questions[0].answer, Answer::Text("new1".to_string()));
        assert_eq!(survey.questions[1].answer, Answer::Text("old2".to_string())); // unchanged
    }

    #[test]
    fn test_update_questions_with_more_new_questions_ignores_extra() {
        let mut survey = Survey {
            questions: vec![Question {
                id: "q1".to_string(),
                answer: Answer::Text("old".to_string()),
                text: Some("Q?".to_string()),
            }],
            ..Default::default()
        };

        let new_questions = vec![
            Question {
                id: "a".to_string(),
                answer: Answer::Text("new1".to_string()),
                text: None,
            },
            Question {
                id: "b".to_string(),
                answer: Answer::Text("new2".to_string()),
                text: None,
            },
        ];

        survey.update_questions(new_questions);

        assert_eq!(survey.questions.len(), 1); // no new questions added
        assert_eq!(survey.questions[0].answer, Answer::Text("new1".to_string()));
    }

    #[test]
    fn test_from_prono_api_survey() {
        let api_survey = prono_api::Survey {
            id: 42,
            description: "API Survey".to_string(),
            questions: vec![prono_api::Question {
                id: "api-q1".to_string(),
                text: Some("API Question?".to_string()),
                answer: prono_api::Answer::Text("api answer".to_string()),
            }],
        };

        let survey: Survey = api_survey.into();

        assert_eq!(survey.id, 42);
        assert_eq!(survey.description, "API Survey");
        assert_eq!(survey.questions.len(), 1);
        assert_eq!(survey.questions[0].id, "api-q1");
    }

    #[test]
    fn test_into_prono_api_survey() {
        let survey = Survey {
            id: 99,
            description: "Test Survey".to_string(),
            questions: vec![Question {
                id: "q1".to_string(),
                text: Some("Question?".to_string()),
                answer: Answer::Text("answer".to_string()),
            }],
        };

        let api_survey: prono_api::Survey = survey.into();

        assert_eq!(api_survey.id, 99);
        assert_eq!(api_survey.description, "Test Survey");
        assert_eq!(api_survey.questions.len(), 1);
        assert_eq!(api_survey.questions[0].id, "q1");
    }

    #[test]
    fn test_survey_roundtrip_conversion() {
        let original = Survey {
            id: 123,
            description: "Roundtrip".to_string(),
            questions: vec![
                Question {
                    id: "q1".to_string(),
                    text: Some("Q1?".to_string()),
                    answer: Answer::Text("a1".to_string()),
                },
                Question {
                    id: "q2".to_string(),
                    text: Some("Q2?".to_string()),
                    answer: Answer::PredictionDate {
                        day: Some(1),
                        month: 1,
                        year: 2025,
                    },
                },
            ],
        };

        let api: prono_api::Survey = original.clone().into();
        let back: Survey = api.into();

        assert_eq!(original, back);
    }
}
