use super::Question;

#[derive(Debug, PartialEq, Eq)]
#[cfg_attr(test, derive(Default))]
#[cfg_attr(any(debug_assertions, test), derive(Clone))]
pub struct Survey {
    pub id: u64,
    pub questions: Vec<Question>,
    pub description: Option<String>,
}

impl From<crate::Survey> for Survey {
    fn from(survey: crate::Survey) -> Self {
        Self {
            id: survey.id,
            questions: survey.questions.into_iter().map(Into::into).collect(),
            description: Some(survey.description),
        }
    }
}

impl From<Survey> for crate::Survey {
    fn from(survey: Survey) -> Self {
        Self {
            id: survey.id,
            description: survey.description.unwrap_or_default(),
            questions: survey.questions.into_iter().map(Into::into).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repo::Answer;

    #[test]
    fn test_from_domain_survey_to_repo_survey() {
        let domain_survey = crate::Survey {
            id: 1,
            description: "Test Survey".to_string(),
            questions: vec![crate::Question {
                id: "q1".to_string(),
                text: Some("Question?".to_string()),
                answer: crate::Answer::Text("answer".to_string()),
            }],
        };

        let repo_survey: Survey = domain_survey.into();

        assert_eq!(repo_survey.id, 1);
        assert_eq!(repo_survey.description, Some("Test Survey".to_string()));
        assert_eq!(repo_survey.questions.len(), 1);
        assert_eq!(repo_survey.questions[0].id, "q1");
    }

    #[test]
    fn test_from_repo_survey_to_domain_survey() {
        let repo_survey = Survey {
            id: 42,
            description: Some("Repo Survey".to_string()),
            questions: vec![Question {
                id: "rq1".to_string(),
                answer: Answer::Text("repo answer".to_string()),
            }],
        };

        let domain_survey: crate::Survey = repo_survey.into();

        assert_eq!(domain_survey.id, 42);
        assert_eq!(domain_survey.description, "Repo Survey");
        assert_eq!(domain_survey.questions.len(), 1);
        assert_eq!(domain_survey.questions[0].id, "rq1");
    }

    #[test]
    fn test_from_repo_survey_with_no_description() {
        let repo_survey = Survey {
            id: 1,
            description: None,
            questions: vec![],
        };

        let domain_survey: crate::Survey = repo_survey.into();

        assert_eq!(domain_survey.description, "");
    }

    #[test]
    fn test_survey_roundtrip_conversion() {
        let original = Survey {
            id: 99,
            description: Some("Roundtrip".to_string()),
            questions: vec![
                Question {
                    id: "q1".to_string(),
                    answer: Answer::Text("a1".to_string()),
                },
                Question {
                    id: "q2".to_string(),
                    answer: Answer::PredictionDate {
                        day: Some(15),
                        month: 6,
                        year: 2025,
                    },
                },
            ],
        };

        let domain: crate::Survey = original.clone().into();
        let back: Survey = domain.into();

        assert_eq!(original.id, back.id);
        assert_eq!(original.questions.len(), back.questions.len());
    }
}
