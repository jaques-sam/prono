use super::Question;

#[derive(Debug, PartialEq, Eq)]
#[cfg_attr(test, derive(Default))]
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
