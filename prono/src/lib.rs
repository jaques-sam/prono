// CLEAN ARCHITECTURE
mod adapters;
mod entities;
mod ports;
mod use_cases;

pub use adapters::*;
pub(crate) use entities::*;
pub use ports::*;
pub use use_cases::*;

static SURVEY_CONFIG: &str = include_str!("./surveys/survey_spacex_starship.json");

/// Returns an empty survey template parsed from the embedded survey JSON.
#[must_use]
pub fn empty_survey() -> prono_api::Survey {
    let survey: Survey = FileSurvey::create_from_file(SURVEY_CONFIG).into();
    survey.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_survey_is_non_empty() {
        let survey = empty_survey();
        assert!(!survey.description.is_empty());
        assert!(!survey.questions.is_empty());
    }
}
