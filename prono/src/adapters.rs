mod config_read;
#[cfg(not(target_arch = "wasm32"))]
mod mysql_prono_db;

pub use config_read::*;
#[cfg(not(target_arch = "wasm32"))]
pub use mysql_prono_db::*;
