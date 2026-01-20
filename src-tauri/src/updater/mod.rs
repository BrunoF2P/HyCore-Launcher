pub mod check;
pub mod cleanup;
pub mod download;
pub mod env;
pub mod java;
pub mod system;
pub mod types;

pub use download::download_with_retry;
pub use system::check_system_requirements;
pub use types::{LocalVersionInfo, SystemRequirements, UpdateStatus};

use crate::error::AppError;
use std::fs;
use std::process::Command;
use tauri::{Emitter, Window};

use crate::platform::{get_hytale_arch, get_hytale_os};

pub fn get_local_version_info() -> LocalVersionInfo {
    let json_path = env::get_version_file_path();

    if json_path.exists() {
        if let Ok(content) = fs::read_to_string(&json_path) {
            if let Ok(info) = serde_json::from_str::<LocalVersionInfo>(&content) {
                return info;
            }
        }
    }

    // Fallback to legacy version.txt
    let txt_path = env::get_legacy_version_file_path();
    let version = fs::read_to_string(txt_path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0);

    LocalVersionInfo {
        version,
        ..Default::default()
    }
}

pub async fn is_update_available() -> Result<(bool, u32), AppError> {
    let local = get_local_version_info();
    let latest = check::find_latest_version().await.map_err(AppError::from)?;

    if latest > local.version {
        return Ok((true, latest));
    }

    if latest == local.version && latest > 0 {
        // Version number is the same, check metadata
        match check::get_remote_metadata(latest).await {
            Ok(remote) => {
                let size_changed = local.size.is_some() && remote.size != local.size;
                let modified_changed =
                    local.last_modified.is_some() && remote.last_modified != local.last_modified;
                let etag_changed =
                    local.etag.is_some() && remote.etag.is_some() && remote.etag != local.etag;

                if size_changed || modified_changed || etag_changed {
                    log::info!("Update detected for same version {}: size_changed={}, modified_changed={}, etag_changed={}", 
                        latest, size_changed, modified_changed, etag_changed);
                    return Ok((true, latest));
                }
            }
            Err(e) => {
                log::warn!(
                    "Failed to fetch remote metadata for version {}: {}",
                    latest,
                    e
                );
            }
        }
    }

    Ok((false, latest))
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
    let latest = check::find_latest_version().await.map_err(AppError::from)?;
    let remote_metadata = check::get_remote_metadata(latest).await.ok();

    let os = get_hytale_os();
    let arch = get_hytale_arch();

    let patch_url = format!(
        "https://game-patches.hytale.com/patches/{}/{}/release/0/{}.pwr",
        os, arch, latest
    );

    let pwr_path = env::get_hycore_data_dir().join(format!("{}.pwr", latest));

    download_with_retry(&patch_url, &pwr_path, &window, 5)
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

    let game_dir = env::get_game_dir();
    let staging_dir = game_dir.join("staging");

    cleanup::clean_staging_dir(&staging_dir).map_err(AppError::from)?;

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

    let version_info = if let Some(info) = remote_metadata {
        info
    } else {
        LocalVersionInfo {
            version: latest,
            ..Default::default()
        }
    };

    if let Ok(json) = serde_json::to_string(&version_info) {
        let _ = fs::write(env::get_version_file_path(), json);
    }

    fs::write(env::get_legacy_version_file_path(), latest.to_string()).map_err(AppError::Io)?;

    log::info!("Update complete! Version bumped to {}", latest);

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
