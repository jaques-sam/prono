//! Server-side core of Prono: the use cases (uploading surveys, submitting
//! answers, results and scores), the repository they run against, and the
//! database configuration.

mod config;
mod error;
mod memory;
mod mysql;
mod repo;
mod service;

pub use config::DbConfig;
pub use error::{Error, Result};
pub use memory::MemoryRepo;
pub use mysql::MysqlRepo;
pub use repo::Repo;
pub use service::{check_pass_phrase, results, scores, submit, survey, surveys, upload_survey, user_status};
