use serde::{Deserialize, Serialize};

use super::{Clear, Question};

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(test, derive(Default))]
pub struct Survey {
    pub description: String,
    pub questions: Vec<Question>,
}

impl Survey {
    pub fn clear(&mut self) {
        self.questions.iter_mut().for_each(Question::clear);
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
