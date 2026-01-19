pub mod cleanup;
pub mod download;
pub mod env;
pub mod java;
pub mod system;
pub mod types;

pub use download::download_with_retry;
pub use system::check_system_requirements;
pub use types::{SystemRequirements, UpdateStatus};

use std::fs;
use std::process::Command;
use std::time::Duration;

use tauri::{Emitter, Window};

pub fn get_local_version() -> u32 {
    let path = env::get_hycore_data_dir().join("version.txt");
    fs::read_to_string(path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0)
}

pub async fn is_update_available() -> Result<(bool, u32), String> {
    let local = get_local_version();
    let latest = find_latest_version().await?;
    Ok((latest > local, latest))
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
