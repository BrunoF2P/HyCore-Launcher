use crate::error::AppError;
use crate::updater::env::{get_game_dir, get_jre_dir, get_user_data_dir};
use std::fs;
use tauri_plugin_opener::OpenerExt;

pub mod info;

#[tauri::command]
pub async fn open_url(app: tauri::AppHandle, url: String) -> Result<(), AppError> {
    log::info!("Opening URL: {}", url);
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| AppError::Unknown(e.to_string()))?;
    Ok(())
}

#[tauri::command]
pub async fn open_game_folder(app: tauri::AppHandle) -> Result<(), AppError> {
    let path = get_game_dir();
    log::info!("Opening game folder: {:?}", path);

    if !path.exists() {
        log::info!("Creating game folder: {:?}", path);
        fs::create_dir_all(&path).map_err(|e| AppError::DirCreation(e.to_string()))?;
    }

    let path_str = path.to_string_lossy().to_string();
    app.opener()
        .open_path(path_str, None::<&str>)
        .map_err(|e| AppError::Unknown(e.to_string()))?;
    Ok(())
}

#[tauri::command]
pub async fn wipe_game_data() -> Result<(), AppError> {
    let path = get_user_data_dir();
    log::info!("Wiping game data at {:?}", path);
    if path.exists() {
        fs::remove_dir_all(&path).map_err(AppError::Io)?;
    }
    log::info!("Game data wipe complete");
    Ok(())
}

#[tauri::command]
pub async fn uninstall_game() -> Result<(), AppError> {
    let game_dir = get_game_dir();
    let jre_dir = get_jre_dir();

    log::info!("Uninstalling game...");

    if game_dir.exists() {
        log::info!("Removing game directory: {:?}", game_dir);
        fs::remove_dir_all(&game_dir).map_err(AppError::Io)?;
    }

    if jre_dir.exists() {
        log::info!("Removing JRE directory: {:?}", jre_dir);
        fs::remove_dir_all(&jre_dir).map_err(AppError::Io)?;
    }

    crate::updater::cleanup::remove_version_files().map_err(AppError::from)?;

    log::info!("Uninstallation complete");

    Ok(())
}
