use std::fs;
use std::path::Path;

use super::env::{get_hycore_data_dir, get_legacy_version_file_path, get_version_file_path};

pub fn cleanup_incomplete_downloads() -> anyhow::Result<()> {
    log::info!("Starting environment cleanup...");

    let app_dir = get_hycore_data_dir();
    let cache_dir = app_dir.join("cache");
    let jre_dir = app_dir.join("jre");
    let butler_dir = app_dir.join("butler");

    // Extensions to clean
    let extensions = vec!["tmp", "partial", "downloading"];

    clean_directory(&cache_dir, &extensions)?;
    clean_directory(&jre_dir, &extensions)?;
    clean_directory(&butler_dir, &extensions)?;
    clean_directory(&app_dir, &extensions)?;

    // Clean incomplete game installation
    // In our structure: hycore/game/Client
    let game_dir = app_dir.join("game");
    clean_incomplete_game(&game_dir)?;

    Ok(())
}

fn clean_directory(dir: &Path, extensions: &[&str]) -> anyhow::Result<()> {
    if !dir.exists() {
        return Ok(());
    }

    let entries = fs::read_dir(dir)?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if extensions.contains(&ext) {
                    log::info!("Removing incomplete file: {:?}", path);
                    let _ = fs::remove_file(path.clone());
                }
            }
            // Also check for butler temp files starting with "sf-" or ending with ".tmp"
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with("sf-") || name.ends_with(".tmp") {
                    log::info!("Removing temp file: {:?}", path);
                    let _ = fs::remove_file(path.clone());
                }
            }
        }
    }
    Ok(())
}

fn clean_incomplete_game(game_dir: &Path) -> anyhow::Result<()> {
    // Check if Client folder exists but is empty or game executable is missing?
    // The Go code uses a specific marker file ".installing". We don't have that yet.
    // However, we can check for the staging directory leftovers.

    let staging_dir = game_dir.join("staging");
    if staging_dir.exists() {
        log::info!("Found leftover staging directory, cleaning up...");
        let _ = fs::remove_dir_all(staging_dir);
    }

    Ok(())
}

pub fn remove_version_files() -> anyhow::Result<()> {
    let json_path = get_version_file_path();
    let txt_path = get_legacy_version_file_path();

    if json_path.exists() {
        log::info!("Removing version file: {:?}", json_path);
        let _ = fs::remove_file(json_path);
    }

    if txt_path.exists() {
        log::info!("Removing legacy version file: {:?}", txt_path);
        let _ = fs::remove_file(txt_path);
    }

    Ok(())
}
