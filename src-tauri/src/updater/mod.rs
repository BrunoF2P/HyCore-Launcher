pub mod cleanup;
pub mod download;
pub mod env;
pub mod java;
pub mod system;
pub mod types;

pub use download::download_with_retry;
pub use system::check_system_requirements;
pub use types::{LocalVersionInfo, SystemRequirements, UpdateStatus};

use std::fs;
use std::process::Command;
use std::time::Duration;

use tauri::{Emitter, Window};

pub fn get_local_version_info() -> LocalVersionInfo {
    let data_dir = env::get_hycore_data_dir();
    let json_path = data_dir.join("version.json");

    if json_path.exists() {
        if let Ok(content) = fs::read_to_string(&json_path) {
            if let Ok(info) = serde_json::from_str::<LocalVersionInfo>(&content) {
                return info;
            }
        }
    }

    // Fallback to version.txt
    let txt_path = data_dir.join("version.txt");
    let version = fs::read_to_string(txt_path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0);

    LocalVersionInfo {
        version,
        ..Default::default()
    }
}

pub async fn is_update_available() -> Result<(bool, u32), String> {
    let local = get_local_version_info();
    let latest = find_latest_version().await?;

    if latest > local.version {
        return Ok((true, latest));
    }

    if latest == local.version && latest > 0 {
        // Version number is the same, check metadata
        match get_remote_metadata(latest).await {
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

async fn get_remote_metadata(version: u32) -> Result<LocalVersionInfo, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| e.to_string())?;

    let os = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "darwin"
    } else {
        "linux"
    };

    let arch = if cfg!(target_arch = "x86_64") {
        "amd64"
    } else {
        "arm64"
    };

    let url = format!(
        "https://game-patches.hytale.com/patches/{}/{}/release/0/{}.pwr",
        os, arch, version
    );

    let resp = client.head(&url).send().await.map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Err(format!("Server returned status {}", resp.status()));
    }

    let headers = resp.headers();

    Ok(LocalVersionInfo {
        version,
        size: headers
            .get(reqwest::header::CONTENT_LENGTH)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse().ok()),
        last_modified: headers
            .get(reqwest::header::LAST_MODIFIED)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string()),
        etag: headers
            .get(reqwest::header::ETAG)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string()),
    })
}

use futures_util::{stream, StreamExt};

async fn find_latest_version() -> Result<u32, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| e.to_string())?;

    let os = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "darwin"
    } else {
        "linux"
    };

    let arch = if cfg!(target_arch = "x86_64") {
        "amd64"
    } else {
        "arm64"
    };

    let tasks = stream::iter(1..=20).map(|v| {
        let client = client.clone();
        let url = format!(
            "https://game-patches.hytale.com/patches/{}/{}/release/0/{}.pwr",
            os, arch, v
        );
        async move {
            let resp = client.head(&url).send().await;
            (v, resp.map(|r| r.status().is_success()).unwrap_or(false))
        }
    });

    let _ = cleanup::cleanup_incomplete_downloads();

    let mut results = tasks.buffer_unordered(10);
    let mut latest = 0;

    while let Some((version, exists)) = results.next().await {
        if exists && version > latest {
            latest = version;
        }
    }

    if latest == 0 {
        return Err("Could not find any game version on patch server".to_string());
    }

    log::info!("Latest available game version: {}", latest);
    Ok(latest)
}

fn clean_staging_dir(staging: &std::path::Path) -> Result<(), String> {
    if staging.exists() {
        if let Err(_) = fs::remove_dir_all(staging) {
            #[cfg(target_os = "windows")]
            {
                if let Ok(entries) = fs::read_dir(staging) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_file() {
                            let _ = fs::remove_file(path);
                        } else if path.is_dir() {
                            let _ = fs::remove_dir_all(path);
                        }
                    }
                }
                // Try removing the dir again
                let _ = fs::remove_dir_all(staging);
            }
        }
    }

    if let Some(parent) = staging.parent() {
        if let Ok(entries) = fs::read_dir(parent) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.ends_with(".tmp") || name.starts_with("sf-") {
                    let _ = fs::remove_file(entry.path());
                }
            }
        }
    }

    fs::create_dir_all(staging).map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn run_update(window: Window) -> Result<(), String> {
    let reqs = check_system_requirements().await;
    if !reqs.meets_requirements {
        return Err(format!(
            "System requirements not met. Space: {}GB, Internet: {}",
            reqs.free_space_gb, reqs.has_internet
        ));
    }

    let butler = system::ensure_butler(&window).await?;
    let latest = find_latest_version().await?;
    let remote_metadata = get_remote_metadata(latest).await.ok();

    let os = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "darwin"
    } else {
        "linux"
    };
    let arch = if cfg!(target_arch = "x86_64") {
        "amd64"
    } else {
        "arm64"
    };

    let patch_url = format!(
        "https://game-patches.hytale.com/patches/{}/{}/release/0/{}.pwr",
        os, arch, latest
    );

    let pwr_path = env::get_hycore_data_dir().join(format!("{}.pwr", latest));

    download_with_retry(&patch_url, &pwr_path, &window, 5).await?;

    if pwr_path.exists() {
        let file_size = fs::metadata(&pwr_path).map_err(|e| e.to_string())?.len();

        if file_size < 100_000 {
            let _ = fs::remove_file(&pwr_path);
            return Err("Downloaded file too small or corrupted. Please try again.".to_string());
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

    clean_staging_dir(&staging_dir)?;

    log::info!("Applying patch with Butler to {:?}", game_dir);
    let output = Command::new(butler)
        .arg("apply")
        .arg("--staging-dir")
        .arg(&staging_dir)
        .arg(&pwr_path)
        .arg(&game_dir)
        .output()
        .map_err(|e| e.to_string())?;

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

        return Err(
            "Installation tool error. The update may be corrupted. Check logs.".to_string(),
        );
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
        let _ = fs::write(env::get_hycore_data_dir().join("version.json"), json);
    }

    fs::write(
        env::get_hycore_data_dir().join("version.txt"),
        latest.to_string(),
    )
    .map_err(|e| e.to_string())?;

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
