mod answer;
mod config;

pub use answer::*;
use async_trait::async_trait;
pub use config::*;

#[async_trait]
pub trait DB {
    fn initialize(&self, config: Config);
    async fn add_answer(&mut self, user: u64, id: u16, answer: DbAnswer);
}
