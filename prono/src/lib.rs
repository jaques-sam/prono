#![warn(clippy::all, rust_2018_idioms)]

// CLEAN ARCHITECTURE
mod entities;
mod ports;
mod use_cases;

#[allow(clippy::wildcard_imports)]
pub(crate) use entities::*;
pub use ports::*;

static SURVEY_CONFIG: &str = include_str!("./surveys/survey_spacex_starship.json");

pub struct PronoLib {
    api: Option<Box<dyn api::Surveys>>,
}

impl Default for PronoLib {
    fn default() -> Self {
        Self {
            api: None,
        }
    }
}

impl PronoLib {
    #[must_use]
    pub fn new(api: Option<Box<dyn api::Surveys>>) -> Self {
        Self {
            api,
            ..Default::default()
        }
    }
}

impl Prono for PronoLib {
    fn empty_survey(&self) -> Survey {
            FileSurvey::create_from_file(SURVEY_CONFIG).into()
    }

    fn filled_survey(&self, user: &str, survey_id: u64) -> Option<Survey> {
        self.api.as_ref().expect("prono api adapter not set").response(user, survey_id).map(|s| s.into())
    }

    fn add_answer(&mut self, user: &str, question_id: String, answer: Answer) {
        self.api
            .as_mut()
            .expect("prono api adapter not set")
            .add_answer(user, question_id, answer.into());
    }

    fn response(&self, user: &str, id: u64) -> Option<Survey> {
        self.api.as_ref().expect("prono api adapter not set").response(user, id).map(|s| s.into())
    }

}
