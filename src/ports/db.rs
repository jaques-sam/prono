use std::path::Path;

mod answer;

pub use answer::*;

pub trait DB {
    fn initialize(&self, config: &Path);
    fn add_answer(&mut self, user: u64, id: u16, answer: DbAnswer);
}
