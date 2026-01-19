use super::api;
use super::types::{Mod, ModManifest};
use crate::updater::env::get_user_data_dir;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::Window;

fn get_mods_dir() -> PathBuf {
    get_user_data_dir().join("Mods")
}

fn get_modpacks_dir() -> PathBuf {
    get_mods_dir().join("Profiles")
}

fn get_active_profile_name_path() -> PathBuf {
    get_mods_dir().join("active_profile.txt")
}

#[tauri::command]
pub fn get_active_profile() -> String {
    let path = get_active_profile_name_path();
    if path.exists() {
        fs::read_to_string(path).unwrap_or_else(|_| "Default".to_string())
    } else {
        "Default".to_string()
    }
}

fn set_active_profile_name(name: &str) -> Result<(), String> {
    let path = get_active_profile_name_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(path, name).map_err(|e| e.to_string())
}

fn get_manifest_path() -> PathBuf {
    let active = get_active_profile();
    get_modpacks_dir().join(format!("{}.json", active))
}

pub fn load_manifest() -> Result<ModManifest, String> {
    let path = get_manifest_path();
    if !path.exists() {
        return Ok(ModManifest {
            mods: vec![],
            version: "1.0".to_string(),
        });
    }

    let data = fs::read(&path).map_err(|e| e.to_string())?;
    serde_json::from_slice(&data).map_err(|e| e.to_string())
}

pub fn save_manifest(manifest: &ModManifest) -> Result<(), String> {
    let path = get_manifest_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let data = serde_json::to_string(manifest).map_err(|e| e.to_string())?;
    fs::write(path, data).map_err(|e| e.to_string())
}

pub fn get_installed_mods() -> Result<Vec<Mod>, String> {
    let manifest = load_manifest()?;
    Ok(manifest.mods)
}

pub async fn install_mod_by_id(
    window: Window,
    mod_id: i32,
    file_id: Option<i32>,
) -> Result<(), String> {
    let details = api::get_mod_details(mod_id).await?;

    let file = if let Some(fid) = file_id {
        api::get_mod_file_details(mod_id, fid).await?
    } else {
        // Get latest file if not specified
        let mut files = api::get_mod_files(mod_id).await?;
        if files.is_empty() {
            return Err("No files available for this mod".to_string());
        }
        files.sort_by(|a, b| b.file_date.cmp(&a.file_date));
        files.remove(0)
    };

    if file.download_url.is_none() {
        return Err("Mod author has disabled direct downloads for this file.".to_string());
    }
    let download_url = file.download_url.unwrap();

    let mods_dir = get_mods_dir();
    fs::create_dir_all(&mods_dir).map_err(|e| e.to_string())?;

    let dest_path = mods_dir.join(&file.file_name);

    crate::updater::download::download_with_retry(&download_url, &dest_path, &window, 3).await?;

    let mut manifest = load_manifest()?;

    let mod_uuid = format!("cf-{}", mod_id);
    manifest.mods.retain(|m| m.id != mod_uuid);

    let author = details
        .authors
        .first()
        .map(|a| a.name.clone())
        .unwrap_or("Unknown".to_string());
    let category = details
        .categories
        .first()
        .map(|c| c.name.clone())
        .unwrap_or("General".to_string());
    let logo = details.logo.map(|l| l.thumbnail_url).unwrap_or_default();

    let new_mod = Mod {
        id: mod_uuid,
        name: details.name,
        slug: Some(details.slug),
        version: file.display_name,
        author,
        description: details.summary,
        download_url: Some(download_url),
        curse_forge_id: Some(mod_id),
        file_id: Some(file.id),
        enabled: true,
        installed_at: chrono::Local::now().to_rfc3339(),
        updated_at: chrono::Local::now().to_rfc3339(),
        file_path: dest_path.to_string_lossy().to_string(),
        icon_url: Some(logo),
        downloads: Some(details.download_count),
        category: Some(category),
        latest_version: None,
        latest_file_id: None,
    };

    manifest.mods.push(new_mod);
    save_manifest(&manifest)?;

    Ok(())
}

pub fn remove_mod(mod_id: String) -> Result<(), String> {
    let mut manifest = load_manifest()?;

    let mut mod_path: Option<String> = None;

    manifest.mods.retain(|m| {
        if m.id == mod_id {
            mod_path = Some(m.file_path.clone());
            false // remove
        } else {
            true // keep
        }
    });

    if let Some(path) = mod_path {
        let p = Path::new(&path);
        if p.exists() {
            let _ = fs::remove_file(p);
        }
    } else {
        return Err(format!("Mod not found: {}", mod_id));
    }

    save_manifest(&manifest)
}

pub fn toggle_mod(mod_id: String, enabled: bool) -> Result<(), String> {
    let mut manifest = load_manifest()?;
    let mut found = false;

    for m in &mut manifest.mods {
        if m.id == mod_id {
            if m.enabled == enabled {
                return Ok(()); // No change
            }

            m.enabled = enabled;
            found = true;

            // Rename file .disabled
            let old_path = PathBuf::from(&m.file_path);
            let mut new_path = old_path.clone();

            if enabled {
                if let Some(ext) = old_path.extension() {
                    if ext == "disabled" {
                        new_path.set_extension("");
                    }
                }
            } else {
                let mut file_name = new_path.file_name().unwrap_or_default().to_os_string();
                file_name.push(".disabled");
                new_path.set_file_name(file_name);
            }

            if old_path.exists() && old_path != new_path {
                fs::rename(&old_path, &new_path).map_err(|e| e.to_string())?;
                m.file_path = new_path.to_string_lossy().to_string();
            }
            break;
        }
    }

    if !found {
        return Err(format!("Mod not found: {}", mod_id));
    }

    save_manifest(&manifest)
}


#[tauri::command]
pub fn create_profile(name: String, empty: bool) -> Result<(), String> {
    let profiles_dir = get_modpacks_dir();
    fs::create_dir_all(&profiles_dir).map_err(|e| e.to_string())?;

    let pack_path = profiles_dir.join(format!("{}.json", name));
    if pack_path.exists() {
        return Err("Profile already exists".to_string());
    }

    let manifest = if empty {
        ModManifest {
            mods: vec![],
            version: "1.0".to_string(),
        }
    } else {
        load_manifest()?
    };

    let data = serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?;
    fs::write(pack_path, data).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_profiles() -> Result<Vec<super::types::Modpack>, String> {
    let profiles_dir = get_modpacks_dir();
    if !profiles_dir.exists() {
        // Create default if nothing exists
        let _ = create_profile("Default".to_string(), true);
        return Ok(vec![super::types::Modpack {
            name: "Default".to_string(),
            mod_count: 0,
            created_at: chrono::Local::now().to_rfc3339(),
        }]);
    }

    let mut packs = vec![];
    let entries = fs::read_dir(profiles_dir).map_err(|e| e.to_string())?;

    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            let data = fs::read(&path).map_err(|e| e.to_string())?;
            let manifest: ModManifest = serde_json::from_slice(&data).map_err(|e| e.to_string())?;
            let name = path.file_stem().unwrap().to_string_lossy().to_string();
            let metadata = fs::metadata(&path).map_err(|e| e.to_string())?;
            let created_at = metadata
                .created()
                .or_else(|_| metadata.modified())
                .unwrap_or_else(|_| std::time::SystemTime::now());
            
            let dt: chrono::DateTime<chrono::Local> = created_at.into();

            packs.push(super::types::Modpack {
                name,
                mod_count: manifest.mods.len(),
                created_at: dt.to_rfc3339(),
            });
        }
    }

    if packs.is_empty() {
         let _ = create_profile("Default".to_string(), true);
         return list_profiles();
    }

    Ok(packs)
}

#[tauri::command]
pub async fn set_active_profile(window: tauri::Window, name: String) -> Result<(), String> {
    let pack_path = get_modpacks_dir().join(format!("{}.json", name));
    if !pack_path.exists() {
        return Err("Profile not found".to_string());
    }

    // 1. Disable all currently active mods from the current profile
    // This ensures no mod files are left enabled when switching context
    let current_manifest = load_manifest()?;
    for m in current_manifest.mods {
        if m.enabled {
             let _ = toggle_mod(m.id.clone(), false);
        }
    }

    // 2. Update active profile state
    set_active_profile_name(&name)?;

    // 3. Synchronize physical files with the new profile's requirements
    sync_profile(window, name).await
}

async fn sync_profile(window: tauri::Window, name: String) -> Result<(), String> {
    let pack_path = get_modpacks_dir().join(format!("{}.json", name));
    let data = fs::read(pack_path).map_err(|e| e.to_string())?;
    let target_manifest: ModManifest = serde_json::from_slice(&data).map_err(|e| e.to_string())?;

    // Iterate through the target profile's mods and ensure local files match its state
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
                let _ = install_mod_by_id(window.clone(), cf_id, pack_mod.file_id).await;
            }
        } else {
            // Apply the enabled/disabled state defined in the profile
            let _ = toggle_mod(pack_mod.id, pack_mod.enabled);
        }
    }

    Ok(())
}

#[tauri::command]
pub fn delete_profile(name: String) -> Result<(), String> {
    if name == "Default" {
        return Err("Cannot delete Default profile".to_string());
    }

    if get_active_profile() == name {
        let _ = set_active_profile_name("Default");
    }

    let pack_path = get_modpacks_dir().join(format!("{}.json", name));
    if pack_path.exists() {
        fs::remove_file(pack_path).map_err(|e| e.to_string())?;
    }
    Ok(())
}
#[tauri::command]
pub async fn check_mods_updates() -> Result<Vec<String>, String> {
    let mut manifest = load_manifest()?;
    let mut mod_ids = Vec::with_capacity(manifest.mods.len());
    for m in &manifest.mods {
        if let Some(id) = m.curse_forge_id {
            mod_ids.push(id);
        }
    }

    if mod_ids.is_empty() {
        return Ok(vec![]);
    }

    // CurseForge supports up to 200 IDs per request
    let remote_mods = api::get_mods(mod_ids).await?;
    let mut updated_ids = vec![];

    for remote_mod in remote_mods {
        if let Some(m) = manifest.mods.iter_mut().find(|m| m.curse_forge_id == Some(remote_mod.id)) {
            // Check if latest file ID is different
            if let Some(latest_file) = remote_mod.latest_files.first() {
                if m.file_id != Some(latest_file.id) {
                    m.latest_version = Some(latest_file.display_name.clone());
                    m.latest_file_id = Some(latest_file.id);
                    updated_ids.push(m.id.clone());
                } else {
                    // Reset if already updated/up to date
                    m.latest_version = None;
                    m.latest_file_id = None;
                }
            }
        }
    }

    if !updated_ids.is_empty() {
        save_manifest(&manifest)?;
    }

    Ok(updated_ids)
}
