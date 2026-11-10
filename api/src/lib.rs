//! The Prono model shared by the app, the backend and the CLI: surveys,
//! answers, the rules a submission must follow, and the REST contract.

mod catalog;
mod survey;

pub use catalog::{survey, surveys};
pub use survey::*;

/// Baked-in API key for backend authentication.
///
/// It ships inside the public web app, so it only keeps out casual scripts:
/// it is not a secret.
pub const API_KEY: &str = "ZGC3Nag7VYVNeExWyoTTZJXa0PkbxO9v+0ScITZFEJI=";

/// Header carrying the submitting device's id.
pub const DEVICE_ID_HEADER: &str = "X-Device-Id";

/// REST path of a survey's answers: `POST` submits a [`Submission`], `GET`
/// returns the [`Results`].
#[must_use]
pub fn answers_path(survey_id: u64) -> String {
    format!("/api/surveys/{survey_id}/answers")
}
