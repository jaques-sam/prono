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

impl Prono for PronoLib {
    fn empty_survey(&self) -> Survey {
        FileSurvey::create_from_file(SURVEY_CONFIG).into()
    }

    fn filled_survey(&self, user: &str, survey_id: u64) -> Option<Survey> {
        self.api
            .as_ref()
            .expect("prono api adapter not set")
            .response(user, survey_id)
            .map(Into::into)
    }

    fn add_answer(&mut self, user: &str, question_id: String, answer: Answer) {
        self.api
            .as_mut()
            .expect("prono api adapter not set")
            .add_answer(user, question_id, answer.into());
    }

    fn response(&self, user: &str, id: u64) -> Option<Survey> {
        self.api
            .as_ref()
            .expect("prono api adapter not set")
            .response(user, id)
            .map(Into::into)
    }
}
