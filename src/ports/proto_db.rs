mod answer;
mod config;

pub use answer::*;
pub use config::*;

use super::SecureConfig;

pub trait DB {
    fn initialize(&self, config: SecureConfig);
    fn add_answer(&mut self, user: u64, id: u16, answer: DbAnswer);
}
