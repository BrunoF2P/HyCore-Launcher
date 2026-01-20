use super::api;
use super::manifest::{get_mods_dir, load_manifest, save_manifest};
use super::types::Mod;
use crate::error::AppError;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::Window;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

pub fn get_installed_mods() -> Result<Vec<Mod>, AppError> {
    let manifest = load_manifest()?;
    Ok(manifest.mods)
}

pub async fn install_mod_by_id(
    window: Window,
    mod_id: i32,
    file_id: Option<i32>,
) -> Result<(), AppError> {
    log::info!(
        "Starting mod installation for mod_id: {}, file_id: {:?}",
        mod_id,
        file_id
    );
    let details = api::get_mod_details(mod_id).await?;

    log::info!("Installing mod: {}", details.name);

    let file = if let Some(fid) = file_id {
        log::info!("Fetching specific file version: {}", fid);
        api::get_mod_file_details(mod_id, fid).await?
    } else {
        // Get latest file if not specified
        log::info!("No file_id provided, finding latest compatible file");
        let mut files = api::get_mod_files(mod_id).await?;
        if files.is_empty() {
            log::error!("No files available for this mod on CurseForge");
            return Err(AppError::ModNotFound(
                "No files available for this mod".to_string(),
            ));
        }
        files.sort_by(|a, b| b.file_date.cmp(&a.file_date));
        files.remove(0)
    };

    if file.download_url.is_none() {
        log::warn!(
            "Mod author has disabled direct downloads for file: {}",
            file.display_name
        );
        return Err(AppError::Unknown(
            "Mod author has disabled direct downloads for this file.".to_string(),
        ));
    }
    let download_url = file.download_url.unwrap();

    let mods_dir = get_mods_dir();
    fs::create_dir_all(&mods_dir).map_err(|e| AppError::DirCreation(e.to_string()))?;

    let dest_path = mods_dir.join(&file.file_name);
    log::info!("Downloading mod to: {:?}", dest_path);

    crate::updater::download::download_with_retry(&download_url, &dest_path, &window, 3)
        .await
        .map_err(|e| {
            let err_msg = format!("Mod download failed: {}", e);
            log::error!("{}", err_msg);
            err_msg
        })?;

    let mut manifest = load_manifest()?;

    let mod_uuid = format!("cf-{}", mod_id);
    log::info!("Updating manifest for mod: {} ({})", details.name, mod_uuid);
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
        installed_at: OffsetDateTime::now_utc().format(&Rfc3339).unwrap(),
        updated_at: OffsetDateTime::now_utc().format(&Rfc3339).unwrap(),
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

pub fn remove_mod(mod_id: String) -> Result<(), AppError> {
    log::info!("Removing mod: {}", mod_id);
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
            log::info!("Deleting physical file: {:?}", p);
            let _ = fs::remove_file(p).map_err(|e| {
                log::warn!("Failed to delete physical file {}: {}", path, e);
            });
        }
    } else {
        log::error!(
            "Failed to remove mod: mod_id {} not found in manifest",
            mod_id
        );
        return Err(AppError::ModNotFound(mod_id));
    }

    save_manifest(&manifest)?;
    log::info!("Mod removed from manifest: {}", mod_id);
    Ok(())
}

pub fn toggle_mod(mod_id: String, enabled: bool) -> Result<(), AppError> {
    log::info!("Toggling mod {} (enabled={})", mod_id, enabled);
    let mut manifest = load_manifest()?;
    let mut found = false;

    for m in &mut manifest.mods {
        if m.id == mod_id {
            if m.enabled == enabled {
                log::info!("Mod {} already in target state, skipping", mod_id);
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
                log::info!("Renaming mod file: {:?} -> {:?}", old_path, new_path);
                fs::rename(&old_path, &new_path).map_err(|e| {
                    let err_msg = e.to_string();
                    log::error!("Failed to rename mod file: {}", err_msg);
                    err_msg
                })?;
                m.file_path = new_path.to_string_lossy().to_string();
            }
            break;
        }
    }

    if !found {
        log::error!("Failed to toggle mod: mod_id {} not found", mod_id);
        return Err(AppError::ModNotFound(mod_id));
    }

    save_manifest(&manifest)?;
    log::info!("Mod toggle complete for {}", mod_id);
    Ok(())
}

#[tauri::command]
pub async fn check_mods_updates() -> Result<Vec<String>, AppError> {
    log::info!("Checking for mod updates...");
    let mut manifest = load_manifest()?;
    let mut mod_ids = Vec::with_capacity(manifest.mods.len());
    for m in &manifest.mods {
        if let Some(id) = m.curse_forge_id {
            mod_ids.push(id);
        }
    }

    if mod_ids.is_empty() {
        log::info!("No installed mods with CurseForge IDs, skipping update check");
        return Ok(vec![]);
    }

    log::info!(
        "Querying update details for {} mods from CurseForge",
        mod_ids.len()
    );
    // CurseForge supports up to 200 IDs per request
    let remote_mods = api::get_mods(mod_ids).await?;
    let mut updated_ids = vec![];

    for remote_mod in remote_mods {
        if let Some(m) = manifest
            .mods
            .iter_mut()
            .find(|m| m.curse_forge_id == Some(remote_mod.id))
        {
            // Check if latest file ID is different
            if let Some(latest_file) = remote_mod.latest_files.first() {
                if m.file_id != Some(latest_file.id) {
                    log::info!(
                        "Update found for mod {}: {} -> {}",
                        m.name,
                        m.version,
                        latest_file.display_name
                    );
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
        log::info!(
            "Found updates for {} mods, saving manifest",
            updated_ids.len()
        );
        save_manifest(&manifest)?;
    } else {
        log::info!("All mods are up to date");
    }

    Ok(updated_ids)
}
