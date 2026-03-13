use actix_web::HttpResponse;

#[derive(Debug, thiserror::Error)]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub enum Error {
    #[error("Repository error: {0}")]
    Repository(String),

    #[error("Answer already exists")]
    AnswerExists,

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Device mismatch: username is registered to a different device")]
    DeviceMismatch,
}

pub type BackendResult<T> = std::result::Result<T, Error>;

impl From<prono::Error> for Error {
    fn from(err: prono::Error) -> Self {
        match err {
            prono::Error::Repository(msg) => Error::Repository(msg),
            prono::Error::AnswerExists => Error::AnswerExists,
            prono::Error::DeviceMismatch => Error::DeviceMismatch,
        }
    }
}

impl actix_web::ResponseError for Error {
    fn error_response(&self) -> HttpResponse {
        match self {
            Error::AnswerExists => HttpResponse::Conflict().json(self.to_string()),
            Error::DeviceMismatch => HttpResponse::Forbidden().json(self.to_string()),
            Error::Repository(msg) | Error::Config(msg) => HttpResponse::InternalServerError().json(msg.clone()),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::http::header;
    use actix_web::{body::to_bytes, http::StatusCode};
    use rstest::rstest;

    #[test]
    fn from_prono_error_maps_repository() {
        let err: Error = prono::Error::Repository("db down".to_string()).into();
        assert_eq!(err, Error::Repository("db down".to_string()));
    }

    #[test]
    fn from_prono_error_maps_answer_exists() {
        let err: Error = prono::Error::AnswerExists.into();
        assert_eq!(err, Error::AnswerExists);
    }

    #[test]
    fn from_prono_error_maps_device_mismatch() {
        let err: Error = prono::Error::DeviceMismatch.into();
        assert_eq!(err, Error::DeviceMismatch);
    }

    #[test]
    fn display_messages_are_expected() {
        assert_eq!(
            Error::Repository("oops".to_string()).to_string(),
            "Repository error: oops"
        );
        assert_eq!(
            Error::Config("bad env".to_string()).to_string(),
            "Configuration error: bad env"
        );
        assert_eq!(Error::AnswerExists.to_string(), "Answer already exists");
        assert_eq!(
            Error::DeviceMismatch.to_string(),
            "Device mismatch: username is registered to a different device"
        );
    }

    async fn assert_response(err: Error, expected_status: StatusCode, expected_json_string: &str) {
        let resp = actix_web::ResponseError::error_response(&err);
        assert_eq!(resp.status(), expected_status);

        let body = to_bytes(resp.into_body())
            .await
            .expect("response body should be readable");
        let body = std::str::from_utf8(&body).expect("response body should be valid UTF-8");

        assert_eq!(body, format!("\"{expected_json_string}\""));
    }

    #[actix_web::test]
    async fn error_response_answer_exists_is_conflict() {
        assert_response(Error::AnswerExists, StatusCode::CONFLICT, "Answer already exists").await;
    }

    #[actix_web::test]
    async fn error_response_device_mismatch_is_forbidden() {
        assert_response(
            Error::DeviceMismatch,
            StatusCode::FORBIDDEN,
            "Device mismatch: username is registered to a different device",
        )
        .await;
    }

    #[actix_web::test]
    async fn error_response_repository_is_internal_server_error_and_uses_raw_message() {
        assert_response(
            Error::Repository("storage unavailable".to_string()),
            StatusCode::INTERNAL_SERVER_ERROR,
            "storage unavailable",
        )
        .await;
    }

    #[actix_web::test]
    async fn error_response_config_is_internal_server_error_and_uses_raw_message() {
        assert_response(
            Error::Config("missing SECRET_KEY".to_string()),
            StatusCode::INTERNAL_SERVER_ERROR,
            "missing SECRET_KEY",
        )
        .await;
    }
    #[rstest]
    #[case(Error::AnswerExists, StatusCode::CONFLICT)]
    #[case(Error::DeviceMismatch, StatusCode::FORBIDDEN)]
    #[case(
    Error::Repository("storage unavailable".to_string()),
    StatusCode::INTERNAL_SERVER_ERROR
)]
    #[case(
    Error::Config("missing SECRET_KEY".to_string()),
    StatusCode::INTERNAL_SERVER_ERROR
)]
    fn error_response_status_matches_variant(#[case] err: Error, #[case] expected: StatusCode) {
        let resp = actix_web::ResponseError::error_response(&err);
        assert_eq!(resp.status(), expected);
    }

    #[rstest]
    #[case(Error::AnswerExists)]
    #[case(Error::DeviceMismatch)]
    #[case(Error::Repository("db".to_string()))]
    #[case(Error::Config("env".to_string()))]
    fn error_response_sets_json_content_type(#[case] err: Error) {
        let resp = actix_web::ResponseError::error_response(&err);
        let content_type = resp
            .headers()
            .get(header::CONTENT_TYPE)
            .expect("content-type header should be present")
            .to_str()
            .expect("content-type header should be valid UTF-8");

        assert!(content_type.starts_with("application/json"));
    }

    #[test]
    fn from_prono_error_preserves_repository_message_exactly() {
        let msg = " db down \n retry ".to_string();
        let err: Error = prono::Error::Repository(msg.clone()).into();
        assert_eq!(err, Error::Repository(msg));
    }

    #[actix_web::test]
    async fn repository_error_response_does_not_use_display_prefix() {
        let err = Error::Repository("storage unavailable".to_string());
        let resp = actix_web::ResponseError::error_response(&err);

        let body = to_bytes(resp.into_body())
            .await
            .expect("response body should be readable");
        let body = std::str::from_utf8(&body).expect("response body should be valid UTF-8");

        assert_eq!(body, "\"storage unavailable\"");
        assert_ne!(body, "\"Repository error: storage unavailable\"");
    }

    #[actix_web::test]
    async fn repository_error_response_escapes_special_characters_as_json() {
        let err = Error::Repository("line1\n\"quoted\"".to_string());
        let resp = actix_web::ResponseError::error_response(&err);

        let body = to_bytes(resp.into_body())
            .await
            .expect("response body should be readable");
        let body = std::str::from_utf8(&body).expect("response body should be valid UTF-8");

        assert_eq!(body, "\"line1\\n\\\"quoted\\\"\"");
    }
}
