//! Server-side core of Prono: the use cases (uploading surveys, submitting
//! answers and results), the repository they run against, and the
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
pub use service::{results, submit, survey, surveys, upload_survey, user_status};
