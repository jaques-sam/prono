use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Serde error: {0}")]
    SerdeError(#[from] serde_json::Error),
    #[error("Mysql error: {0}")]
    MysqlError(#[from] mysql_async::Error),
    #[error("Configuration error")]
    ConfigError(String),
    #[error("Internal error: {0}")]
    InternalError(String),
}
