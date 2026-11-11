use prono_api::Invalid;

/// Why a use case failed. Except for [`Error::Repository`], the message is
/// meant for the participant.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Invalid(#[from] Invalid),

    #[error("Missing or invalid device id")]
    InvalidDeviceId,

    #[error("You already answered this survey")]
    AnswerExists,

    #[error("This name is registered on another device")]
    DeviceMismatch,

    #[error("This device was already used by another participant")]
    DeviceInUse,

    #[error("Wrong pass phrase")]
    WrongPassPhrase,

    #[error("Invalid survey definition: {0}")]
    InvalidSurvey(String),

    #[error("Repository error: {0}")]
    Repository(String),
}

impl From<sqlx::Error> for Error {
    fn from(err: sqlx::Error) -> Self {
        Error::Repository(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;
