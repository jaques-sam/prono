//! The surveys shipped in `api/surveys/`, as fixtures for tests. They are
//! uploaded to a backend with `prono-cli`, never compiled into a binary.

use crate::Survey;

/// The definition of [`starship`]: a survey with a pass phrase.
pub const STARSHIP_JSON: &str = include_str!("../surveys/spacex_starship.json");
/// The definition of [`course`]: a survey with a name list.
pub const COURSE_JSON: &str = include_str!("../surveys/rust_course.json");

/// The Starship predictions survey.
///
/// # Panics
///
/// Never: the test `the_shipped_surveys_parse` guarantees the file is valid.
#[must_use]
pub fn starship() -> Survey {
    Survey::from_json(STARSHIP_JSON).expect("the Starship survey is valid")
}

/// The Rust course survey.
///
/// # Panics
///
/// Never: the test `the_shipped_surveys_parse` guarantees the file is valid.
#[must_use]
pub fn course() -> Survey {
    Survey::from_json(COURSE_JSON).expect("the course survey is valid")
}
