use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{Emitter, Window};

use super::download::download_with_retry;
use super::system::log_error;
use super::types::UpdateStatus;

const JRE_VERSION: &str = "25";

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct JrePlatform {
    url: String,
    sha256: String,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct JreConfig {
    #[allow(dead_code)]
    version: String,
    #[allow(dead_code)]
    download_url: std::collections::HashMap<String, std::collections::HashMap<String, JrePlatform>>,
}

#[tauri::command]
pub fn get_java_bin_path() -> PathBuf {
    super::env::get_java_binary()
}

pub async fn ensure_java(window: &Window) -> Result<PathBuf, String> {
    let java_path = get_java_bin_path();

    if java_path.exists() {
        if let Ok(metadata) = fs::metadata(&java_path) {
            if metadata.len() > 0 {
                return Ok(java_path);
            }
        }
    }

    // Java not found, download it
    let _ = window.emit(
        "update-status",
        UpdateStatus {
            stage: "jre".to_string(),
            progress: 0.0,
            message: "Initializing Java Runtime download...".to_string(),
        },
    );

    let jre_dir = super::env::get_jre_dir();
    if jre_dir.exists() {
        let _ = fs::remove_dir_all(&jre_dir);
    }
    fs::create_dir_all(&jre_dir).map_err(|e| e.to_string())?;

    let os = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "mac"
    } else {
        "linux"
    };

    let arch = if cfg!(target_arch = "x86_64") {
        "x64"
    } else if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else {
        return Err("Unsupported architecture".to_string());
    };

    let url = format!(
        "https://api.adoptium.net/v3/binary/latest/{}/ga/{}/{}/jre/hotspot/normal/eclipse?project=jdk",
        JRE_VERSION, os, arch
    );

    log_error(&format!("Downloading JRE from: {}", url));

    let archive_ext = if cfg!(target_os = "windows") {
        "zip"
    } else {
        "tar.gz"
    };
    let archive_path = super::env::get_hycore_data_dir().join(format!("jre.{}", archive_ext));

    download_with_retry(&url, &archive_path, window, 3).await?;

    let _ = window.emit(
        "update-status",
        UpdateStatus {
            stage: "jre".to_string(),
            progress: 90.0,
            message: "Extracting Java Runtime...".to_string(),
        },
    );

    extract_jre(&archive_path, &jre_dir).await?;

    let _ = fs::remove_file(archive_path);

    if !java_path.exists() {
        return Err("Java executable not found after extraction".to_string());
    }

    #[cfg(not(target_os = "windows"))]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&java_path)
            .map_err(|e| e.to_string())?
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&java_path, perms).map_err(|e| e.to_string())?;
    }

    let _ = window.emit(
        "update-status",
        UpdateStatus {
            stage: "jre".to_string(),
            progress: 100.0,
            message: "Java Runtime Ready".to_string(),
        },
    );

    Ok(java_path)
}

async fn extract_jre(archive_path: &Path, dest: &Path) -> Result<(), String> {
    log_error(&format!("Extracting JRE to {:?}", dest));

    let file = fs::File::open(archive_path).map_err(|e| e.to_string())?;

    if archive_path.extension().unwrap_or_default() == "zip" {
        let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i).map_err(|e| e.to_string())?;

            if file.name().ends_with('/') {
                continue;
            }

            let path = file.enclosed_name().ok_or("Invalid file path in zip")?;
            let mut components = path.components();
            components.next();

            let outpath = dest.join(components.as_path());

            if let Some(p) = outpath.parent() {
                fs::create_dir_all(p).map_err(|e| e.to_string())?;
            }

            let mut outfile = fs::File::create(&outpath).map_err(|e| e.to_string())?;
            std::io::copy(&mut file, &mut outfile).map_err(|e| e.to_string())?;
        }
    } else {
        use flate2::read::GzDecoder;
        use tar::Archive;

        let tar = GzDecoder::new(file);
        let mut archive = Archive::new(tar);

        for entry in archive.entries().map_err(|e| e.to_string())? {
            let mut entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path().map_err(|e| e.to_string())?;

            let mut components = path.components();
            components.next();

            let outpath = dest.join(components.as_path());

            if let Some(p) = outpath.parent() {
                fs::create_dir_all(p).map_err(|e| e.to_string())?;
            }

            entry.unpack(&outpath).map_err(|e| e.to_string())?;
        }
    }

    let mac_home = dest.join("Contents").join("Home");
    if mac_home.exists() {
        log_error("macOS JRE structure detected, normalizing...");

        // List all files in mac_home and move them to dest
        let entries = fs::read_dir(&mac_home).map_err(|e| e.to_string())?;
        for entry in entries {
            let entry = entry.map_err(|e| e.to_string())?;
            let target_path = dest.join(entry.file_name());

            // If target exists (rare, but just in case), remove it
            if target_path.exists() {
                if target_path.is_dir() {
                    let _ = fs::remove_dir_all(&target_path);
                } else {
                    let _ = fs::remove_file(&target_path);
                }
            }

            fs::rename(entry.path(), target_path).map_err(|e| e.to_string())?;
        }

        let _ = fs::remove_dir_all(dest.join("Contents"));
    }

    Ok(())
}
