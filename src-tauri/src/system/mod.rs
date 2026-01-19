use crate::updater::env::{get_game_dir, get_jre_dir, get_user_data_dir};
use std::fs;
use std::process::Command;

pub mod info;

#[tauri::command]
pub async fn open_url(url: String) -> Result<(), String> {
    log::info!("Opening URL: {}", url);
    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open").arg(&url).spawn().map_err(|e| {
            let err_msg = format!("Failed to open URL: {}", e);
            log::error!("{}", err_msg);
            err_msg
        })?;
    }

    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .arg("/C")
            .arg("start")
            .arg(&url)
            .spawn()
            .map_err(|e| {
                let err_msg = format!("Failed to open URL: {}", e);
                log::error!("{}", err_msg);
                err_msg
            })?;
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open").arg(&url).spawn().map_err(|e| {
            let err_msg = format!("Failed to open URL: {}", e);
            log::error!("{}", err_msg);
            err_msg
        })?;
    }

    Ok(())
}

#[tauri::command]
pub async fn open_game_folder() -> Result<(), String> {
    let path = get_game_dir();
    log::info!("Opening game folder: {:?}", path);

    if !path.exists() {
        log::info!("Creating game folder: {:?}", path);
        fs::create_dir_all(&path).map_err(|e| {
            let err_msg = e.to_string();
            log::error!("Failed to create game folder: {}", err_msg);
            err_msg
        })?;
    }

    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open").arg(&path).spawn().map_err(|e| {
            let err_msg = format!("Failed to open folder: {}", e);
            log::error!("{}", err_msg);
            err_msg
        })?;
    }

    #[cfg(target_os = "windows")]
    {
        Command::new("explorer").arg(&path).spawn().map_err(|e| {
            let err_msg = format!("Failed to open folder: {}", e);
            log::error!("{}", err_msg);
            err_msg
        })?;
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open").arg(&path).spawn().map_err(|e| {
            let err_msg = format!("Failed to open folder: {}", e);
            log::error!("{}", err_msg);
            err_msg
        })?;
    }

    Ok(())
}

#[tauri::command]
pub async fn wipe_game_data() -> Result<(), String> {
    let path = get_user_data_dir();
    log::info!("Wiping game data at {:?}", path);
    if path.exists() {
        fs::remove_dir_all(&path).map_err(|e| {
            let err_msg = format!("Failed to wipe data: {}", e);
            log::error!("{}", err_msg);
            err_msg
        })?;
    }
    log::info!("Game data wipe complete");
    Ok(())
}

#[tauri::command]
pub async fn uninstall_game() -> Result<(), String> {
    let game_dir = get_game_dir();
    let jre_dir = get_jre_dir();
    let version_file = crate::updater::env::get_hycore_data_dir().join("version.txt");

    log::info!("Uninstalling game...");

    if game_dir.exists() {
        log::info!("Removing game directory: {:?}", game_dir);
        fs::remove_dir_all(&game_dir).map_err(|e| {
            let err_msg = format!("Failed to remove game dir: {}", e);
            log::error!("{}", err_msg);
            err_msg
        })?;
    }

    if jre_dir.exists() {
        log::info!("Removing JRE directory: {:?}", jre_dir);
        fs::remove_dir_all(&jre_dir).map_err(|e| {
            let err_msg = format!("Failed to remove JRE dir: {}", e);
            log::error!("{}", err_msg);
            err_msg
        })?;
    }

    if version_file.exists() {
        log::info!("Removing version file: {:?}", version_file);
        fs::remove_file(&version_file).map_err(|e| {
            let err_msg = format!("Failed to remove version file: {}", e);
            log::error!("{}", err_msg);
            err_msg
        })?;
    }

    log::info!("Uninstallation complete");

    Ok(())
}
