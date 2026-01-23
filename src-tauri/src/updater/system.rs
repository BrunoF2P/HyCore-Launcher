use futures_util::StreamExt;
use std::fs;
use std::path::{Path, PathBuf};

use super::env::get_hycore_data_dir;
use super::types::SystemRequirements;

pub async fn check_system_requirements() -> SystemRequirements {
    let disks = sysinfo::Disks::new_with_refreshed_list();

    let hycore_dir = get_hycore_data_dir();
    let free_space_bytes = disks
        .iter()
        .find(|disk| hycore_dir.starts_with(disk.mount_point()))
        .map(|disk| disk.available_space())
        .unwrap_or(0);

    let free_space_gb = free_space_bytes / (1024 * 1024 * 1024);

    let has_internet = crate::http::HTTP_CLIENT
        .head("https://game-patches.hytale.com")
        .send()
        .await
        .is_ok();

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

pub async fn ensure_butler(window: &tauri::Window) -> anyhow::Result<PathBuf> {
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
            message: "Fetching update tools metadata...".to_string(),
        },
    );

    let os = crate::platform::get_butler_os();

    // Fetch latest version string to build accurate URL
    let latest_url = format!("https://broth.itch.zone/butler/{}-amd64/LATEST", os);
    let version = crate::http::HTTP_CLIENT
        .get(latest_url)
        .send()
        .await?
        .text()
        .await?
        .trim()
        .to_string();

    let download_url = format!(
        "https://broth.itch.zone/butler/{}-amd64/{}/archive/default",
        os, version
    );

    log::info!("Downloading Butler {} from: {}", version, download_url);

    let _ = window.emit(
        "update-status",
        UpdateStatus {
            stage: "butler".to_string(),
            progress: 20.0,
            message: format!("Downloading Butler {}...", version),
        },
    );

    let response = crate::http::HTTP_CLIENT.get(download_url).send().await?;
    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "Failed to download Butler: {}",
            response.status()
        ));
    }

    let mut zip_path = get_hycore_data_dir();
    zip_path.push("butler.zip");

    {
        let mut file = fs::File::create(&zip_path)?;
        let mut stream = response.bytes_stream();
        while let Some(item) = stream.next().await {
            let chunk = item.map_err(|e| anyhow::anyhow!("Error while downloading: {}", e))?;
            std::io::copy(&mut &*chunk, &mut file)?;
        }
    }

    let file_size = fs::metadata(&zip_path)?.len();
    if file_size < 1_000_000 {
        let _ = fs::remove_file(&zip_path);
        return Err(anyhow::anyhow!(
            "Butler download too small ({} bytes), likely corrupted",
            file_size
        ));
    }

    let _ = window.emit(
        "update-status",
        UpdateStatus {
            stage: "butler".to_string(),
            progress: 80.0,
            message: "Verifying and extracting tools...".to_string(),
        },
    );

    // Initial ZIP check (will error if invalid)
    let dest_parent = butler_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Invalid butler path parent"))?;
    extract_zip(&zip_path, dest_parent)?;

    let _ = fs::remove_file(zip_path);

    #[cfg(not(target_os = "windows"))]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&butler_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&butler_path, perms)?;
    }

    Ok(butler_path)
}

fn extract_zip(zip_path: &Path, dest: &Path) -> anyhow::Result<()> {
    let file = fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = dest.join(file.mangled_name());

        if (&*file.name()).ends_with('/') {
            fs::create_dir_all(&outpath)?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(&p)?;
                }
            }
            let mut outfile = fs::File::create(&outpath)?;
            std::io::copy(&mut file, &mut outfile)?;
        }
    }
    Ok(())
}
