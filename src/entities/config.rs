use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Survey {
    description: String,
    questions: Vec<Question>,
}

impl Survey {
    pub fn create_from_file(json_body: &str) -> Self {
        serde_json::from_str(json_body)
            .unwrap_or_else(|err| panic!("Unable to parse survey configuration (err: {err})"))
    }
}

impl From<Survey> for super::Survey {
    fn from(survey: Survey) -> Self {
        Self {
            description: survey.description,
            questions: survey
                .questions
                .into_iter()
                .enumerate()
                .map(|(index, mut q)| {
                    q.id = index as u16;
                    q.into()
                })
                .collect(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct Question {
    #[serde(skip)]
    id: u16,
    question: String,
    answer_type: AnswerType,
}

impl From<Question> for super::Question {
    fn from(question: Question) -> Self {
        Self {
            id: question.id,
            question: question.question,
            answer: match question.answer_type {
                AnswerType::Text => super::Answer::new_text(),
                AnswerType::PredictionDate => super::Answer::new_prediction_date(),
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

    #[test]
    fn test_configuration_from_file() {
        let json_data = include_str!("../configurations/survey_spacex_starship.json");

        let survey: Survey = serde_json::from_str(json_data).unwrap();
        println!("{:?}", survey);
    }
}
