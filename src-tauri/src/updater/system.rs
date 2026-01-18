use chrono;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::env::get_hycore_data_dir;
use super::types::SystemRequirements;

pub fn log_error(message: &str) {
    let log_path = get_hycore_data_dir().join("updater.log");
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    let log_line = format!("[{}] ERR: {}\n", timestamp, message);

    if let Ok(mut file) = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
    {
        let _ = file.write_all(log_line.as_bytes());
    }
}

pub async fn check_system_requirements() -> SystemRequirements {
    let disks = sysinfo::Disks::new_with_refreshed_list();

    let hycore_dir = get_hycore_data_dir();
    let free_space_bytes = disks
        .iter()
        .find(|disk| hycore_dir.starts_with(disk.mount_point()))
        .map(|disk| disk.available_space())
        .unwrap_or(0);

    let free_space_gb = free_space_bytes / (1024 * 1024 * 1024);

    let has_internet = match reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
    {
        Ok(client) => client
            .head("https://game-patches.hytale.com")
            .send()
            .await
            .is_ok(),
        Err(_) => false,
    };

    SystemRequirements {
        has_internet,
        free_space_gb,
        meets_requirements: has_internet && free_space_gb >= 10,
    }
}

pub fn get_butler_path() -> PathBuf {
    let mut path = get_hycore_data_dir();
    path.push("bin");
    let _ = fs::create_dir_all(&path);

    #[cfg(target_os = "windows")]
    path.push("butler.exe");
    #[cfg(not(target_os = "windows"))]
    path.push("butler");

    path
}

pub async fn ensure_butler(window: &tauri::Window) -> Result<PathBuf, String> {
    use super::types::UpdateStatus;
    use tauri::Emitter;

    let butler_path = get_butler_path();
    if butler_path.exists() {
        return Ok(butler_path);
    }

    let _ = window.emit(
        "update-status",
        UpdateStatus {
            stage: "butler".to_string(),
            progress: 10.0,
            message: "Downloading update tools...".to_string(),
        },
    );

    let os = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "darwin"
    } else {
        "linux"
    };

    let url = format!(
        "https://broth.itch.zone/butler/{}-amd64/LATEST/archive/default",
        os
    );

    let response = reqwest::get(url).await.map_err(|e| e.to_string())?;
    let content = response.bytes().await.map_err(|e| e.to_string())?;

    let mut zip_path = get_hycore_data_dir();
    zip_path.push("butler.zip");
    fs::write(&zip_path, content).map_err(|e| e.to_string())?;

    let _ = window.emit(
        "update-status",
        UpdateStatus {
            stage: "butler".to_string(),
            progress: 80.0,
            message: "Extracting tools...".to_string(),
        },
    );

    extract_zip(&zip_path, &butler_path.parent().unwrap())?;

    let _ = fs::remove_file(zip_path);

    #[cfg(not(target_os = "windows"))]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&butler_path)
            .map_err(|e| e.to_string())?
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&butler_path, perms).map_err(|e| e.to_string())?;
    }

    Ok(butler_path)
}

fn extract_zip(zip_path: &Path, dest: &Path) -> Result<(), String> {
    let file = fs::File::open(zip_path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
        let outpath = dest.join(file.mangled_name());

        if (&*file.name()).ends_with('/') {
            fs::create_dir_all(&outpath).map_err(|e| e.to_string())?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(&p).map_err(|e| e.to_string())?;
                }
            }
            let mut outfile = fs::File::create(&outpath).map_err(|e| e.to_string())?;
            std::io::copy(&mut file, &mut outfile).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[allow(dead_code)]
fn clean_staging_dir(staging: &Path) -> Result<(), String> {
    if staging.exists() {
        fs::remove_dir_all(staging).map_err(|e| e.to_string())?;
    }
    fs::create_dir_all(staging).map_err(|e| e.to_string())?;
    Ok(())
}
