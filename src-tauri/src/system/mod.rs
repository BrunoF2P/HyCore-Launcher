use crate::error::AppError;
use crate::updater::env::{get_game_dir, get_hycore_data_dir, get_user_data_dir};
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

    if crate::platform::is_game_running() {
        log::warn!("User is opening game folder while game is running");
    }

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
    if crate::platform::is_game_running() {
        return Err(AppError::GameRunning);
    }

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
    if crate::platform::is_game_running() {
        return Err(AppError::GameRunning);
    }

    let app_dir = get_hycore_data_dir();
    let game_dir = app_dir.join("game");
    let jre_dir = app_dir.join("jre");

    log::info!("Uninstalling game (all versions)...");

    if game_dir.exists() {
        log::info!("Removing entire game directory: {:?}", game_dir);
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
