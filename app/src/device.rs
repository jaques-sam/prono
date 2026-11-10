//! A random id that identifies this installation, created on first use. The
//! backend binds each participant's name to it.

#[cfg(not(target_arch = "wasm32"))]
pub fn device_id() -> String {
    let path = dirs::config_dir().unwrap_or_default().join("prono").join("device_id");
    if let Ok(id) = std::fs::read_to_string(&path)
        && !id.trim().is_empty()
    {
        return id.trim().to_string();
    }

    let id = uuid::Uuid::new_v4().to_string();
    let stored = path
        .parent()
        .map_or(Ok(()), std::fs::create_dir_all)
        .and_then(|()| std::fs::write(&path, &id));
    if let Err(err) = stored {
        log::warn!("Cannot store the device id in {}: {err}", path.display());
    }
    id
}

#[cfg(target_arch = "wasm32")]
pub fn device_id() -> String {
    const STORAGE_KEY: &str = "prono_device_id";

    let storage = web_sys::window().and_then(|w| w.local_storage().ok().flatten());
    if let Some(id) = storage.as_ref().and_then(|s| s.get_item(STORAGE_KEY).ok().flatten()) {
        return id;
    }

    let id = uuid::Uuid::new_v4().to_string();
    if storage.is_none_or(|s| s.set_item(STORAGE_KEY, &id).is_err()) {
        log::warn!("Cannot store the device id in local storage");
    }
    id
}
