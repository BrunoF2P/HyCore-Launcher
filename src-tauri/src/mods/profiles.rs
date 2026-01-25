use super::manifest::{
    get_active_profile, get_modpacks_dir, load_manifest, set_active_profile_name,
};
use super::operations::{install_mod_by_id, toggle_mod};
use super::types::{ModManifest, Modpack};
use crate::error::AppError;
use once_cell::sync::Lazy;
use std::fs;
use std::path::Path;
use tauri::Window;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use tokio::sync::Mutex;

static PROFILE_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

fn validate_profile_name(name: &str) -> Result<(), AppError> {
    if name.trim().is_empty() {
        return Err(AppError::Unknown("Profile name cannot be empty".to_string()));
    }
    if name.chars().any(|c| !c.is_alphanumeric() && c != '_' && c != '-' && c != ' ') {
        return Err(AppError::Unknown("Profile name contains invalid characters. Only letters, numbers, spaces, -, and _ are allowed.".to_string()));
    }
    Ok(())
}

#[tauri::command]
pub async fn create_profile(name: String, empty: bool) -> Result<(), AppError> {
    let _guard = PROFILE_LOCK.lock().await;
    validate_profile_name(&name)?;
    log::info!("Creating new profile: {} (empty={})", name, empty);
    let profiles_dir = get_modpacks_dir();
    fs::create_dir_all(&profiles_dir).map_err(|e| {
        let err_msg = e.to_string();
        log::error!("Failed to create profiles directory: {}", err_msg);
        err_msg
    })?;

    let pack_path = profiles_dir.join(format!("{}.json", name));
    if pack_path.exists() {
        log::error!("Failed to create profile: {} already exists", name);
        return Err(AppError::Unknown("Profile already exists".to_string()));
    }

    let manifest = if empty {
        ModManifest {
            mods: vec![],
            version: "1.0".to_string(),
        }
    } else {
        log::info!("Cloning current manifest for new profile");
        load_manifest()?
    };

    let data = serde_json::to_string_pretty(&manifest).map_err(|e| {
        let err_msg = e.to_string();
        log::error!("Failed to serialize new profile manifest: {}", err_msg);
        err_msg
    })?;
    fs::write(&pack_path, data).map_err(|e| {
        let err_msg = e.to_string();
        log::error!("Failed to write profile file: {}", err_msg);
        err_msg
    })?;
    log::info!("Profile created successfully at {:?}", pack_path);
    Ok(())
}

#[tauri::command]
pub async fn list_profiles() -> Result<Vec<Modpack>, AppError> {
    let _guard = PROFILE_LOCK.lock().await;
    log::info!("Listing available profiles...");
    let profiles_dir = get_modpacks_dir();
    if !profiles_dir.exists() {
        // Create default if nothing exists
        log::info!("Profiles directory not found, creating Default profile");
        let _ = create_profile("Default".to_string(), true).await;
        return Ok(vec![Modpack {
            name: "Default".to_string(),
            mod_count: 0,
            created_at: OffsetDateTime::now_utc()
                .format(&Rfc3339)
                .unwrap_or_default(),
        }]);
    }

    let mut packs = vec![];
    let entries = fs::read_dir(&profiles_dir).map_err(|e| {
        let err_msg = e.to_string();
        log::error!("Failed to read profiles directory: {}", err_msg);
        err_msg
    })?;

    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            let data = fs::read(&path).map_err(|e| e.to_string())?;
            let manifest: ModManifest = serde_json::from_slice(&data).map_err(|e| {
                log::warn!("Failed to parse profile at {:?}: {}", path, e);
                e.to_string()
            })?;
            let name = path.file_stem().unwrap().to_string_lossy().to_string();
            let metadata = fs::metadata(&path).map_err(|e| e.to_string())?;
            let created_at = metadata
                .created()
                .or_else(|_| metadata.modified())
                .unwrap_or_else(|_| std::time::SystemTime::now());

            let dt = OffsetDateTime::from(created_at);

            packs.push(Modpack {
                name,
                mod_count: manifest.mods.len(),
                created_at: dt.format(&Rfc3339).unwrap_or_default(),
            });
        }
    }

    if packs.is_empty() {
        log::info!("No profile files found, creating Default profile");
        let _ = create_profile("Default".to_string(), true).await;
        let _ = create_profile("Default".to_string(), true).await;
        return Ok(vec![Modpack {
            name: "Default".to_string(),
            mod_count: 0,
            created_at: OffsetDateTime::now_utc()
                .format(&Rfc3339)
                .unwrap_or_default(),
        }]);
    }

    log::info!("Found {} profiles", packs.len());
    Ok(packs)
}

#[tauri::command]
pub async fn set_active_profile(window: Window, name: String) -> Result<(), AppError> {
    let _guard = PROFILE_LOCK.lock().await;
    validate_profile_name(&name)?;
    log::info!("Setting active profile to: {}", name);
    let pack_path = get_modpacks_dir().join(format!("{}.json", name));
    if !pack_path.exists() {
        log::error!(
            "Failed to set active profile: {} not found at {:?}",
            name,
            pack_path
        );
        return Err(AppError::ProfileNotFound(name));
    }

    // 1. Disable all currently active mods from the current profile
    // This ensures no mod files are left enabled when switching context
    log::info!("Disabling mods from current profile before switch...");
    let current_manifest = load_manifest()?;
    for m in current_manifest.mods {
        if m.enabled {
            let _ = toggle_mod(m.id.clone(), false);
        }
    }

    // 2. Update active profile state
    set_active_profile_name(&name)?;

    // 3. Synchronize physical files with the new profile's requirements
    log::info!("Synchronizing files for new profile: {}", name);
    sync_profile(window, name).await
}

pub async fn sync_profile(window: Window, name: String) -> Result<(), AppError> {
    log::info!("Syncing profile: {}", name);
    let pack_path = get_modpacks_dir().join(format!("{}.json", name));
    let data = fs::read(&pack_path).map_err(|e| {
        let err_msg = format!("Failed to read target profile {:?}: {}", pack_path, e);
        log::error!("{}", err_msg);
        AppError::from(err_msg)
    })?;
    let target_manifest: ModManifest = serde_json::from_slice(&data).map_err(|e| {
        let err_msg = format!("Failed to parse target profile {:?}: {}", pack_path, e);
        log::error!("{}", err_msg);
        AppError::from(err_msg)
    })?;

    // Iterate through the target profile's mods and ensure local files match its state
    log::info!(
        "Syncing {} mods for profile {}",
        target_manifest.mods.len(),
        name
    );
    for pack_mod in target_manifest.mods {
        let p = Path::new(&pack_mod.file_path);
        let mut disabled_path = p.to_path_buf();
        let mut file_name = disabled_path.file_name().unwrap_or_default().to_os_string();
        file_name.push(".disabled");
        disabled_path.set_file_name(file_name);

        let exists = p.exists() || disabled_path.exists();

        // If mod is missing, trigger an installation from CurseForge
        if !exists {
            if let Some(cf_id) = pack_mod.curse_forge_id {
                log::info!(
                    "Mod missing locally, re-installing: {} (cf_id={})",
                    pack_mod.name,
                    cf_id
                );
                let _ = install_mod_by_id(window.clone(), cf_id, pack_mod.file_id).await;
            } else {
                log::warn!(
                    "Mod {} missing locally and has no CurseForge ID to re-install",
                    pack_mod.name
                );
            }
        } else {
            // Apply the enabled/disabled state defined in the profile
            log::info!(
                "Ensuring mod state: {} (enabled={})",
                pack_mod.name,
                pack_mod.enabled
            );
            let _ = toggle_mod(pack_mod.id, pack_mod.enabled);
        }
    }

    log::info!("Profile sync complete for: {}", name);
    Ok(())
}

#[tauri::command]
pub async fn delete_profile(name: String) -> Result<(), AppError> {
    let _guard = PROFILE_LOCK.lock().await;
    validate_profile_name(&name)?;
    log::info!("Deleting profile: {}", name);
    if name == "Default" {
        log::error!("Attempted to delete protected 'Default' profile");
        return Err(AppError::Unknown(
            "Cannot delete Default profile".to_string(),
        ));
    }

    if get_active_profile() == name {
        log::info!("Deleting currently active profile, switching to Default");
        let _ = set_active_profile_name("Default");
    }

    let pack_path = get_modpacks_dir().join(format!("{}.json", name));
    if pack_path.exists() {
        fs::remove_file(&pack_path).map_err(|e| {
            let err_msg = format!("Failed to delete profile file at {:?}: {}", pack_path, e);
            log::error!("{}", err_msg);
            AppError::from(err_msg)
        })?;
        log::info!("Profile file deleted: {:?}", pack_path);
    } else {
        log::warn!("Profile file not found for deletion: {:?}", pack_path);
    }
    Ok(())
}
