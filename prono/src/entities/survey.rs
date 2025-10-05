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
