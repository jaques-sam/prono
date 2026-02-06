use serde::{Deserialize, Serialize};

use crate::Survey;

#[derive(Default, Deserialize, Serialize)]
pub enum SurveyState {
    #[default]
    NotStarted,
    InProgress(Survey),
    Completed,
}
