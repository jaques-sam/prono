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
