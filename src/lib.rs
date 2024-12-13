#![warn(clippy::all, rust_2018_idioms)]

mod app;
pub mod generic;
pub use app::App;

// CLEAN ARCHITECTURE
mod adapters;
mod entities;
mod ports;
mod use_cases;

#[allow(unused_imports)]
pub use adapters::*;
pub(crate) use entities::*;
pub use ports::*;
