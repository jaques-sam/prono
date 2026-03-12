use std::fs;
use std::path::PathBuf;

use log::info;

pub struct NativeIdentity {
    id: String,
}

impl NativeIdentity {
    pub fn load_or_create() -> Self {
        let path = Self::device_id_path();
        let id = if path.exists() {
            fs::read_to_string(&path).unwrap_or_else(|_| Self::create_new(&path))
        } else {
            Self::create_new(&path)
        };
        info!("Device ID: {id}");
        Self { id }
    }

    fn device_id_path() -> PathBuf {
        let config_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")).join("prono");
        config_dir.join("device_id")
    }

    fn create_new(path: &PathBuf) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(path, &id);
        id
    }
}

impl prono_api::Identity for NativeIdentity {
    fn device_id(&self) -> &str {
        &self.id
    }
}
