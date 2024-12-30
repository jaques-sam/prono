#![warn(clippy::all, rust_2018_idioms)]

// CLEAN ARCHITECTURE
mod entities;
mod ports;
mod use_cases;

pub(crate) use entities::*;
pub use ports::*;

static SURVEY_CONFIG: &str = include_str!("./surveys/survey_spacex_starship.json");

pub struct PronoLib {
    _survey: FileSurvey,
}

impl Default for PronoLib {
    fn default() -> Self {
        Self {
            _survey: FileSurvey::create_from_file(SURVEY_CONFIG),
        }
    }
}
