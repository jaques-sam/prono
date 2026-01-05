// CLEAN ARCHITECTURE
mod adapters;
mod entities;
pub(crate) use adapters::*;
pub(crate) use entities::*;

#[cfg(not(target_arch = "wasm32"))]
mod main_native;

#[cfg(target_arch = "wasm32")]
mod main_wasm;

#[cfg(not(target_arch = "wasm32"))]
pub use main_native::main;

#[cfg(target_arch = "wasm32")]
pub use main_wasm::main;
