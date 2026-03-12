use log::info;

static STORAGE_KEY: &str = "prono_device_id";

pub struct WasmIdentity {
    id: String,
}

impl WasmIdentity {
    pub fn load_or_create() -> Self {
        let storage = web_sys::window().and_then(|w| w.local_storage().ok().flatten());

        let id = storage
            .as_ref()
            .and_then(|s| s.get_item(STORAGE_KEY).ok().flatten())
            .unwrap_or_else(|| {
                let id = uuid::Uuid::new_v4().to_string();
                if let Some(s) = &storage {
                    let _ = s.set_item(STORAGE_KEY, &id);
                }
                id
            });

        info!("Device ID: {id}");
        Self { id }
    }
}

impl prono_api::Identity for WasmIdentity {
    fn device_id(&self) -> &str {
        &self.id
    }
}
