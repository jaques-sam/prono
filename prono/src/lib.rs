#![warn(clippy::all, rust_2018_idioms)]

// CLEAN ARCHITECTURE
mod entities;
mod ports;
mod use_cases;

pub(crate) use entities::*;
pub use ports::*;

static SURVEY_CONFIG: &str = include_str!("./surveys/survey_spacex_starship.json");

pub struct PronoLib {
    api: Option<Box<dyn api::PronoApi>>,
    survey: Survey,
}

impl Default for PronoLib {
    fn default() -> Self {
        Self {
            api: None,
            survey: FileSurvey::create_from_file(SURVEY_CONFIG).into(),
        }
    }
}

impl PronoLib {
    pub fn new(api: Option<Box<dyn api::PronoApi>>) -> Self {
        Self {
            api,
            ..Default::default()
        }
    }
}

impl api::PronoApi for PronoLib {
    fn answer(&self, user: &str, id: u64) -> api::Answer {
        self.api.as_ref().expect("prono api adapter not set").answer(user, id)
    }

    fn response(&self, user: &str, id: u64) -> Option<api::Survey> {
        self.api.as_ref().expect("prono api adapter not set").response(user, id)
    }
}

impl Prono for PronoLib {
    fn survey(&self) -> api::Survey {
        (&self.survey).into()
    }
}
