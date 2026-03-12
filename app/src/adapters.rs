mod app;
#[cfg(not(target_arch = "wasm32"))]
pub mod identity_native;
#[cfg(target_arch = "wasm32")]
pub mod identity_wasm;
mod timeline;

pub use app::*;
