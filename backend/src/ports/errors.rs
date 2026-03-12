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
}

pub type BackendResult<T> = std::result::Result<T, Error>;

impl From<prono::Error> for Error {
    fn from(err: prono::Error) -> Self {
        match err {
            prono::Error::Repository(msg) => Error::Repository(msg),
            prono::Error::AnswerExists => Error::AnswerExists,
        }
    }
}

impl actix_web::ResponseError for Error {
    fn error_response(&self) -> HttpResponse {
        match self {
            Error::AnswerExists => HttpResponse::Conflict().json(self.to_string()),
            Error::Repository(msg) | Error::Config(msg) => HttpResponse::InternalServerError().json(msg.clone()),
        }
    }
}
