//! The Prono model shared by the app, the backend and the CLI: surveys and
//! how they are defined, answers, the rules a submission must follow, and the
//! REST contract.

mod definition;
#[cfg(any(test, feature = "samples"))]
pub mod samples;
mod survey;

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

/// REST path of the surveys a pass phrase opens: `POST` a JSON
/// `Option<String>`, made with [`clean_pass_phrase`], and get the JSON list of
/// the [`Survey`]s with that pass phrase, by id. It answers `403` with the
/// error message when none has it. It sends the same API key and device id
/// headers as a submission. A survey is only ever sent to someone who knows
/// its pass phrase.
#[must_use]
pub fn surveys_path() -> &'static str {
    "/api/surveys"
}

/// REST path of a scored survey's scores: `GET` returns a JSON list of
/// [`Score`]s, best first.
#[must_use]
pub fn scores_path(survey_id: u64) -> String {
    format!("/api/surveys/{survey_id}/scores")
}

/// REST path asking whether `user` already filled in a survey: `GET` returns
/// a JSON `bool`. It sends the same API key and device id headers as a
/// submission, and reveals nothing else about the user.
#[must_use]
pub fn user_path(user: &str) -> String {
    format!(
        "/api/users/{}",
        percent_encoding::utf8_percent_encode(user, percent_encoding::NON_ALPHANUMERIC)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_path_escapes_the_name() {
        assert_eq!(user_path("Sam"), "/api/users/Sam");
        assert_eq!(user_path("a/b c?é"), "/api/users/a%2Fb%20c%3F%C3%A9");
    }
}
