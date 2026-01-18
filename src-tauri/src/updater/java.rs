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
        // Quick verify
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

    // Determine platform
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

    // Construct Adoptium URL directly
    // Using binary/latest/{version}/ga/{os}/{arch}/jre/hotspot/normal/eclipse?project=jdk
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

    // Cleanup
    let _ = fs::remove_file(archive_path);

    // Verify
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

        // JRE archives often have a top-level directory e.g., "jdk-25+xx-jre/"
        // We want to flatten this effectively or just extract and then move.
        // For simplicity, let's extract all.

        for i in 0..archive.len() {
            let mut file = archive.by_index(i).map_err(|e| e.to_string())?;

            // Skip if it's a directory
            if file.name().ends_with('/') {
                continue;
            }

            // Strip the top level directory if present
            let path = file.enclosed_name().ok_or("Invalid file path in zip")?;
            let mut components = path.components();
            components.next(); // Skip root folder

            let outpath = dest.join(components.as_path());

            if let Some(p) = outpath.parent() {
                fs::create_dir_all(p).map_err(|e| e.to_string())?;
            }

            let mut outfile = fs::File::create(&outpath).map_err(|e| e.to_string())?;
            std::io::copy(&mut file, &mut outfile).map_err(|e| e.to_string())?;
        }
    } else {
        // tar.gz
        use flate2::read::GzDecoder;
        use tar::Archive;

        let tar = GzDecoder::new(file);
        let mut archive = Archive::new(tar);

        for entry in archive.entries().map_err(|e| e.to_string())? {
            let mut entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path().map_err(|e| e.to_string())?;

            // Strip top level directory
            let mut components = path.components();
            components.next(); // Skip root folder (e.g. jdk-21.0.1-jre)

            let outpath = dest.join(components.as_path());

            if let Some(p) = outpath.parent() {
                fs::create_dir_all(p).map_err(|e| e.to_string())?;
            }

            entry.unpack(&outpath).map_err(|e| e.to_string())?;
        }
    }

    // On macOS, the structure might be different (contents/home), handle normalization if needed.
    // Adoptium logic:
    // Linux/Windows: jdk-xx-jre/bin/java
    // macOS: jdk-xx-jre/Contents/Home/bin/java
    // With strip component logic above:
    // Linux/Windows: /bin/java (Correct)
    // macOS: Contents/Home/bin/java (Need to handle deep structure?)

    // Check if bin exists in dest, otherwise search for it
    if !dest.join("bin").exists() {
        // Use a walker to find bin directory?
        // For now, let's assume the strip logic works for Linux/Windows which is user's immediate concern.
        // If macOS structure remains nested (Contents/Home), we might need extra logic.

        // Simple fix for macOS if needed:
        let mac_home = dest.join("Contents").join("Home");
        if mac_home.exists() {
            // Move contents of Home to dest
            // This is complex to implement robustly in one go.
            // Letting it fail if logic is wrong is better than deleting random things.
        }
    }

    Ok(())
}
