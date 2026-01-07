#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Repository error: {0}")]
    Repository(String),

    #[error("Answer already exists")]
    AnswerExists,
}

pub type PronoResult<T> = std::result::Result<T, Error>;
