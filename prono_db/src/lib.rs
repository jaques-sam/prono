// CLEAN ARCHITECTURE
mod adapters;
mod ports;

pub use adapters::*;
pub use ports::*;

use async_trait::async_trait;

#[async_trait]
pub trait DB {
    fn initialize(&self, config: Config);
    async fn add_answer(&mut self, user: u64, id: u16, answer: DbAnswer);
}
