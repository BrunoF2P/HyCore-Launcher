pub mod check;
pub mod cleanup;
pub mod download;
pub mod env;
pub mod java;
pub mod system;
pub mod types;

pub use download::download_with_retry;
pub use system::check_system_requirements;
pub use types::{LocalManifest, LocalVersionInfo, SystemRequirements, UpdateStatus};

use crate::error::AppError;
use once_cell::sync::Lazy;
use std::fs;
use std::process::Command;
use std::sync::Mutex;
use tauri::{Emitter, Window};

static UPDATE_LOCK: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(false));

use crate::platform::{get_hytale_arch, get_hytale_os};

pub fn get_local_manifest() -> LocalManifest {
    let conn = match crate::database::get_conn() {
        Ok(c) => c,
        Err(e) => {
            log::error!("Failed to open DB for local manifest: {}. Using default.", e);
            return LocalManifest::default();
        }
    };
    let mut stmt = match conn.prepare(
        "SELECT version, channel, installed_at, last_modified, size, etag FROM installed_versions",
    ) {
        Ok(s) => s,
        Err(e) => {
            log::error!("Failed to prepare statement: {}", e);
            return LocalManifest::default();
        }
    };

    let version_iter = match stmt.query_map([], |row| {
        Ok(LocalVersionInfo {
            version: row.get(0)?,
            channel: row.get(1)?,
            installed_at: row.get(2)?,
            last_modified: row.get(3)?,
            size: row.get(4)?,
            etag: row.get(5)?,
        })
    }) {
        Ok(iter) => iter,
        Err(e) => {
            log::error!("Failed to query installed versions: {}", e);
            return LocalManifest::default();
        }
    };

    let mut installed = Vec::new();
    for v in version_iter {
        if let Ok(info) = v {
            // Validate existence of folder
            let dir = env::get_version_dir(info.version);
            if dir.exists() {
                installed.push(info);
            } else {
                // Cleanup missing from DB
                let _ = conn.execute(
                    "DELETE FROM installed_versions WHERE version = ?",
                    [info.version],
                );
            }
        }
    }

    // Migration
    if installed.is_empty() {
        let json_path = env::get_versions_manifest_path();
        if json_path.exists() {
            if let Ok(content) = fs::read_to_string(&json_path) {
                if let Ok(manifest) = serde_json::from_str::<LocalManifest>(&content) {
                    for v in &manifest.installed {
                        let _ = conn.execute(
                            "INSERT OR IGNORE INTO installed_versions (version, channel, installed_at, last_modified, size, etag) 
                             VALUES (?, ?, ?, ?, ?, ?)",
                            rusqlite::params![v.version, v.channel, v.installed_at, v.last_modified, v.size, v.etag],
                        );
                    }
                    installed = manifest.installed;
                }
            }
        }
    }

    let settings = crate::settings::load_settings();
    LocalManifest {
        installed,
        active_version: settings.active_version,
    }
}

pub async fn is_update_available() -> Result<(bool, u32), AppError> {
    let settings = crate::settings::load_settings();
    let active_version = settings.active_version;

    // If no version is active or latest isn't found, find latest
    let latest = check::find_latest_version(&settings.channel)
        .await
        .map_err(AppError::from)?;

    let target_version = if active_version == 0 || active_version > latest {
        latest
    } else {
        active_version
    };

    let manifest = get_local_manifest();
    let local_info = manifest
        .installed
        .into_iter()
        .find(|v| v.version == target_version);

    // If target version isn't installed at all, it's "available"
    let is_installed = local_info.is_some();
    if !is_installed {
        return Ok((true, target_version));
    }

    let local = local_info.unwrap();

    // If it's installed, check for hotfixes/metadata changes
    match check::get_remote_metadata(target_version, &settings.channel).await {
        Ok(remote) => {
            let size_changed = local.size.is_some() && remote.size != local.size;
            let modified_changed =
                local.last_modified.is_some() && remote.last_modified != local.last_modified;
            let etag_changed =
                local.etag.is_some() && remote.etag.is_some() && remote.etag != local.etag;

            if size_changed || modified_changed || etag_changed {
                log::info!("Update detected for version {}: size_changed={}, modified_changed={}, etag_changed={}", 
                    target_version, size_changed, modified_changed, etag_changed);
                return Ok((true, target_version));
            }
        }
        Err(e) => {
            log::warn!(
                "Failed to fetch remote metadata for version {}: {}",
                target_version,
                e
            );
        }
    }

    Ok((false, target_version))
}

pub async fn run_update(window: Window) -> Result<(), AppError> {
    let reqs = check_system_requirements().await;
    if !reqs.meets_requirements {
        return Err(AppError::Unknown(format!(
            "System requirements not met. Space: {}GB, Internet: {}",
            reqs.free_space_gb, reqs.has_internet
        )));
    }

    let butler = system::ensure_butler(&window)
        .await
        .map_err(AppError::from)?;

    let settings = crate::settings::load_settings();
    let latest = check::find_latest_version(&settings.channel)
        .await
        .map_err(AppError::from)?;
    let remote_metadata = check::get_remote_metadata(latest, &settings.channel)
        .await
        .ok();

    let os = get_hytale_os();
    let arch = get_hytale_arch();

    let patch_url = format!(
        "https://game-patches.hytale.com/patches/{}/{}/{}/0/{}.pwr",
        os, arch, &settings.channel, latest
    );

    let pwr_path = env::get_hycore_data_dir().join(format!("{}.pwr", latest));

    download_with_retry(&patch_url, &pwr_path, &window, 5, None)
        .await
        .map_err(AppError::from)?;

    if pwr_path.exists() {
        let file_size = fs::metadata(&pwr_path).map_err(AppError::Io)?.len();

        if file_size < 100_000 {
            let _ = fs::remove_file(&pwr_path);
            return Err(AppError::Unknown(
                "Downloaded file too small or corrupted. Please try again.".to_string(),
            ));
        }
    }

    let _ = window.emit(
        "update-status",
        UpdateStatus {
            stage: "install".to_string(),
            progress: 50.0,
            message: "Applying patch with Butler...".to_string(),
        },
    );

    let game_dir = env::get_version_dir(latest);
    let _ = fs::create_dir_all(&game_dir);
    let staging_dir = game_dir.join("staging");

    // Robust cleanup: ensure staging is completely empty before Butler starts
    if staging_dir.exists() {
        let _ = fs::remove_dir_all(&staging_dir);
    }
    let _ = fs::create_dir_all(&staging_dir);

    log::info!("Applying patch with Butler to {:?}", game_dir);
    let output = Command::new(butler)
        .arg("apply")
        .arg("--staging-dir")
        .arg(&staging_dir)
        .arg(&pwr_path)
        .arg(&game_dir)
        .output()
        .map_err(AppError::Io)?;

    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let err_msg = format!(
            "Butler failed!\nSTDOUT: {}\nSTDERR: {}",
            stdout.trim(),
            stderr.trim()
        );
        log::error!("{}", err_msg);

        if pwr_path.exists() {
            let _ = fs::remove_file(&pwr_path);
        }

        return Err(AppError::Unknown(
            "Installation tool error. The update may be corrupted. Check logs.".to_string(),
        ));
    }

    let _ = fs::remove_file(pwr_path);
    let _ = fs::remove_dir_all(staging_dir);

    // Update local manifest in DB
    let conn = crate::database::get_conn().map_err(AppError::from)?;
    let version_info = if let Some(mut info) = remote_metadata {
        info.installed_at = Some(time::OffsetDateTime::now_utc().to_string());
        info
    } else {
        LocalVersionInfo {
            version: latest,
            channel: settings.channel.clone(),
            installed_at: Some(time::OffsetDateTime::now_utc().to_string()),
            ..Default::default()
        }
    };

    let _ = conn.execute(
        "INSERT OR REPLACE INTO installed_versions (version, channel, installed_at, last_modified, size, etag) 
         VALUES (?, ?, ?, ?, ?, ?)",
        rusqlite::params![
            version_info.version,
            version_info.channel,
            version_info.installed_at,
            version_info.last_modified,
            version_info.size,
            version_info.etag
        ],
    );

    // Update active version in settings
    let mut settings = crate::settings::load_settings();
    settings.active_version = latest;
    let _ = crate::settings::set_game_settings(settings);

    log::info!("Update complete! Version installed to {:?}", game_dir);

    let _ = window.emit(
        "update-status",
        UpdateStatus {
            stage: "done".to_string(),
            progress: 100.0,
            message: "Update complete!".to_string(),
        },
    );

    Ok(())
}

#[tauri::command]
pub async fn check_update_requirements() -> Result<SystemRequirements, AppError> {
    log::info!("Checking update requirements...");
    let reqs = check_system_requirements().await;
    log::info!(
        "Requirements checked: meets_requirements={}",
        reqs.meets_requirements
    );
    Ok(reqs)
}

#[tauri::command]
pub async fn check_for_game_update() -> Result<(bool, u32), AppError> {
    log::info!("Checking for game update...");
    match is_update_available().await {
        Ok(res) => {
            log::info!("Game update check: available={}, version={}", res.0, res.1);
            Ok(res)
        }
        Err(e) => {
            log::error!("Failed to check for game update: {}", e);
            Err(e)
        }
    }
}

#[tauri::command]
pub async fn start_game_update(window: tauri::Window) -> Result<(), AppError> {
    {
        let mut lock = UPDATE_LOCK.lock().unwrap();
        if *lock {
            log::warn!("Update already in progress, ignoring request");
            return Err(AppError::Unknown(
                "Uma atualização já está em andamento.".to_string(),
            ));
        }
        *lock = true;
    }

    log::info!("Starting game update process...");
    let result = run_update(window).await;

    {
        let mut lock = UPDATE_LOCK.lock().unwrap();
        *lock = false;
    }

    match result {
        Ok(_) => {
            log::info!("Game update process finished successfully");
            Ok(())
        }
        Err(e) => {
            log::error!("Game update process failed: {}", e);
            Err(e)
        }
    }
}

#[tauri::command]
pub async fn java_bin_path_command() -> std::path::PathBuf {
    log::info!("Frontend requested Java binary path");
    java::get_java_bin_path()
}

#[tauri::command]
pub fn get_local_manifest_command() -> LocalManifest {
    get_local_manifest()
}

#[tauri::command]
pub async fn get_available_versions_command() -> Result<Vec<u32>, AppError> {
    let settings = crate::settings::load_settings();
    check::find_all_versions(&settings.channel)
        .await
        .map_err(AppError::from)
}

#[tauri::command]
pub fn switch_version_command(version: u32) -> Result<(), AppError> {
    let mut settings = crate::settings::load_settings();
    settings.active_version = version;
    crate::settings::set_game_settings(settings)?;
    log::info!("Switched active version to {}", version);
    Ok(())
}
