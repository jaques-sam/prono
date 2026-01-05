#![warn(clippy::all, rust_2018_idioms)]

// CLEAN ARCHITECTURE
mod entities;
mod ports;
mod use_cases;

pub use entities::*;
pub use ports::*;

static SURVEY_CONFIG: &str = include_str!("./surveys/survey_spacex_starship.json");

#[derive(Default)]
pub struct PronoLib {
    api: Option<Box<dyn repo::Surveys>>,
}

impl PronoLib {
    #[must_use]
    pub fn new(api: Option<Box<dyn repo::Surveys>>) -> Self {
        Self { api }
    }
}

impl prono_api::Surveys for PronoLib {
    fn empty_survey(&self) -> prono_api::Survey {
        let survey: Survey = FileSurvey::create_from_file(SURVEY_CONFIG).into();

        survey.into()
    }

    fn add_answer(&mut self, user: &str, question_id: String, answer: prono_api::Answer) {
        let answer: Answer = answer.into();
        self.api
            .as_mut()
            .expect("prono api adapter not set")
            .add_answer(user, question_id, answer.into());
    }

    fn response(&self, user: &str, survey_id: u64) -> Option<prono_api::Survey> {
        self.api
            .as_ref()
            .expect("prono api adapter not set")
            .response(user, survey_id)
            .map(|survey| {
                let survey: Survey = survey.into();
                survey.into()
            })
    }
}
