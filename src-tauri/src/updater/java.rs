use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{Emitter, Window};

use super::download::download_with_retry;
use super::env::get_hycore_data_dir;
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

pub fn get_java_bin_path() -> PathBuf {
    get_hycore_data_dir()
        .join("jre")
        .join("bin")
        .join(if cfg!(windows) { "java.exe" } else { "java" })
}

pub async fn ensure_java(window: &Window) -> anyhow::Result<PathBuf> {
    let java_path = get_java_bin_path();

    if java_path.exists() {
        if let Ok(metadata) = fs::metadata(&java_path) {
            if metadata.len() > 0 {
                log::info!("Java already installed at {:?}", java_path);
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
    fs::create_dir_all(&jre_dir)?;

    let os = crate::platform::get_java_os();

    let arch = crate::platform::get_java_arch();

    let url = format!(
        "https://api.adoptium.net/v3/binary/latest/{}/ga/{}/{}/jre/hotspot/normal/eclipse?project=jdk",
        JRE_VERSION, os, arch
    );

    log::info!("Downloading JRE from: {}", url);

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

    log::info!("Extraction complete for JRE");
    let _ = fs::remove_file(archive_path);

    if !java_path.exists() {
        return Err(anyhow::anyhow!(
            "Java executable not found after extraction"
        ));
    }

    #[cfg(not(target_os = "windows"))]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&java_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&java_path, perms)?;
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

async fn extract_jre(archive_path: &Path, dest: &Path) -> anyhow::Result<()> {
    log::info!("Extracting JRE to {:?}", dest);

    let file = fs::File::open(archive_path)?;

    if archive_path.extension().unwrap_or_default() == "zip" {
        let mut archive = zip::ZipArchive::new(file)?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;

            if file.name().ends_with('/') {
                continue;
            }

            let path = file
                .enclosed_name()
                .ok_or_else(|| anyhow::anyhow!("Invalid file path in zip"))?;
            let mut components = path.components();
            components.next();

            let outpath = dest.join(components.as_path());

            if let Some(p) = outpath.parent() {
                fs::create_dir_all(p)?;
            }

            let mut outfile = fs::File::create(&outpath)?;
            std::io::copy(&mut file, &mut outfile)?;
        }
    } else {
        use flate2::read::GzDecoder;
        use tar::Archive;

        let tar = GzDecoder::new(file);
        let mut archive = Archive::new(tar);

        for entry in archive.entries()? {
            let mut entry = entry?;
            let path = entry.path()?.to_path_buf();

            let mut components = path.components();
            components.next();

            let outpath = dest.join(components.as_path());

            if let Some(p) = outpath.parent() {
                fs::create_dir_all(p)?;
            }

            entry.unpack(&outpath)?;
        }
    }

    let mac_home = dest.join("Contents").join("Home");
    if mac_home.exists() {
        log::info!("macOS JRE structure detected, normalizing...");

        // List all files in mac_home and move them to dest
        let entries = fs::read_dir(&mac_home)?;
        for entry in entries {
            let entry = entry?;
            let target_path = dest.join(entry.file_name());

            // If target exists (rare, but just in case), remove it
            if target_path.exists() {
                if target_path.is_dir() {
                    let _ = fs::remove_dir_all(&target_path);
                } else {
                    let _ = fs::remove_file(&target_path);
                }
            }

            fs::rename(entry.path(), target_path)?;
        }

        let _ = fs::remove_dir_all(dest.join("Contents"));
    }

    Ok(())
}
