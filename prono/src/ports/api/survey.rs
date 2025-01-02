use super::Question;

#[derive(Debug, PartialEq, Eq)]
#[cfg_attr(test, derive(Default))]
pub struct Survey {
    pub id: u64,
    pub description: String,
    pub questions: Vec<Question>,
}

impl From<&crate::Survey> for Survey {
    fn from(survey: &crate::Survey) -> Self {
        Self {
            id: survey.id,
            description: survey.description.clone(),
            questions: survey.questions.clone().into_iter().map(Into::into).collect(),
        }
    }
}
