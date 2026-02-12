#[derive(Debug, thiserror::Error)]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub enum Error {
    #[error("Repository error: {0}")]
    Repository(String),

    #[error("Answer already exists")]
    AnswerExists,
}

pub type PronoResult<T> = std::result::Result<T, Error>;
