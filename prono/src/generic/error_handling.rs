#![allow(dead_code)]

use log::error;

pub fn add_panic_hook() {
    std::panic::set_hook(Box::new(|panic_info| {
        let payload = panic_info.payload();
        let msg = if let Some(info) = payload.downcast_ref::<String>() {
            info
        } else if let Some(&info) = payload.downcast_ref::<&str>() {
            info
        } else {
            "Panic occurred"
        };

        error!("{}", msg);
    }));
}
