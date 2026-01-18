use super::api;
use super::types::{Mod, ModManifest};
use crate::updater::env::get_user_data_dir;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::Window;

fn get_mods_dir() -> PathBuf {
    get_user_data_dir().join("Mods")
}

fn get_manifest_path() -> PathBuf {
    get_mods_dir().join("manifest.json")
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

    let data = serde_json::to_string_pretty(manifest).map_err(|e| e.to_string())?;
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
