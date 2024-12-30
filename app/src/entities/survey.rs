use serde::{Deserialize, Serialize};

use super::{Clear, Question};
use prono::api;

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
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
}

impl From<api::Survey> for Survey {
    fn from(proto_survey: api::Survey) -> Self {
        Survey {
            id: proto_survey.id,
            description: proto_survey.description,
            questions: proto_survey.questions.into_iter().map(Into::into).collect(),
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
}
