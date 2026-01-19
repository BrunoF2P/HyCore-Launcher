use super::types::ModManifest;
use crate::updater::env::get_user_data_dir;
use std::fs;
use std::path::PathBuf;

pub fn get_mods_dir() -> PathBuf {
    get_user_data_dir().join("Mods")
}

pub fn get_modpacks_dir() -> PathBuf {
    get_mods_dir().join("Profiles")
}

pub fn get_active_profile_name_path() -> PathBuf {
    get_mods_dir().join("active_profile.txt")
}

#[tauri::command]
pub fn get_active_profile() -> String {
    let path = get_active_profile_name_path();
    if path.exists() {
        fs::read_to_string(&path)
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|e| {
                log::warn!(
                    "Failed to read active profile file ({}), defaulting to 'Default'",
                    e
                );
                "Default".to_string()
            })
    } else {
        "Default".to_string()
    }
}

pub fn set_active_profile_name(name: &str) -> Result<(), String> {
    log::info!("Switching active profile to: {}", name);
    let path = get_active_profile_name_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            let err_msg = e.to_string();
            log::error!("Failed to create directory for profile state: {}", err_msg);
            err_msg
        })?;
    }
    fs::write(&path, name).map_err(|e| {
        let err_msg = e.to_string();
        log::error!("Failed to write active profile state: {}", err_msg);
        err_msg
    })
}

pub fn get_manifest_path() -> PathBuf {
    let active = get_active_profile();
    get_modpacks_dir().join(format!("{}.json", active))
}

pub fn load_manifest() -> Result<ModManifest, String> {
    let path = get_manifest_path();
    log::info!("Loading manifest from {:?}", path);
    if !path.exists() {
        log::info!("Manifest does not exist, providing empty structure");
        return Ok(ModManifest {
            mods: vec![],
            version: "1.0".to_string(),
        });
    }

    let data = fs::read(&path).map_err(|e| {
        let err_msg = e.to_string();
        log::error!("Failed to read manifest file: {}", err_msg);
        err_msg
    })?;
    serde_json::from_slice(&data).map_err(|e| {
        let err_msg = e.to_string();
        log::error!("Failed to parse manifest JSON: {}", err_msg);
        err_msg
    })
}

pub fn save_manifest(manifest: &ModManifest) -> Result<(), String> {
    let path = get_manifest_path();
    log::info!("Saving manifest to {:?}", path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            let err_msg = e.to_string();
            log::error!("Failed to create directory for manifest: {}", err_msg);
            err_msg
        })?;
    }

    let data = serde_json::to_string(manifest).map_err(|e| {
        let err_msg = e.to_string();
        log::error!("Failed to serialize manifest: {}", err_msg);
        err_msg
    })?;
    fs::write(&path, data).map_err(|e| {
        let err_msg = e.to_string();
        log::error!("Failed to write manifest file: {}", err_msg);
        err_msg
    })
}
