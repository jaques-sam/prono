mod answer;
mod question;
mod survey;

pub use answer::*;
pub use question::*;
pub use survey::*;

pub trait PronoApi {
    fn answer(&self, user: u64, id: u16) -> Answer;
}
