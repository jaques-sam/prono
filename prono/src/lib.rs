#![warn(clippy::all, rust_2018_idioms)]

pub mod generic;
use std::path::Path;

// CLEAN ARCHITECTURE
mod adapters;
mod entities;
mod ports;
mod use_cases;

pub use adapters::*;
pub use entities::*;
pub use ports::*;

static SURVEY_CONFIG: &str = include_str!("./surveys/survey_spacex_starship.json");
static CONFIG_FILENAME: &str = "secure_config.toml";

pub struct PronoLib {
    _survey: FileSurvey,
    _db: Box<dyn crate::prono_db::DB>,
}

impl Default for PronoLib {
    fn default() -> Self {
        let db_config = ConfigRead {}.read(Path::new(CONFIG_FILENAME)).db();

        Self {
            _survey: FileSurvey::create_from_file(SURVEY_CONFIG).into(),
            _db: Box::new(crate::MysqlDb::new(db_config)),
        }
    }
}
