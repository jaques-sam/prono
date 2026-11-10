//! Server-side core of Prono: the submit and results use cases, the
//! repository they run against, and the database configuration.

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
pub use service::{results, submit, user_status};
