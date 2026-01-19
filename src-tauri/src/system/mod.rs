use crate::updater::env::{get_game_dir, get_jre_dir, get_user_data_dir};
use std::fs;
use std::process::Command;

#[tauri::command]
pub async fn open_url(url: String) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(&url)
            .spawn()
            .map_err(|e| format!("Failed to open URL: {}", e))?;
    }

    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .arg("/C")
            .arg("start")
            .arg(&url)
            .spawn()
            .map_err(|e| format!("Failed to open URL: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(&url)
            .spawn()
            .map_err(|e| format!("Failed to open URL: {}", e))?;
    }

    Ok(())
}

#[tauri::command]
pub async fn open_game_folder() -> Result<(), String> {
    let path = get_game_dir();

    if !path.exists() {
        fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }

    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }

    Ok(())
}

#[tauri::command]
pub async fn wipe_game_data() -> Result<(), String> {
    let path = get_user_data_dir();
    if path.exists() {
        fs::remove_dir_all(&path).map_err(|e| format!("Failed to wipe data: {}", e))?;
    }
    Ok(())
}

#[tauri::command]
pub async fn uninstall_game() -> Result<(), String> {
    let game_dir = get_game_dir();
    let jre_dir = get_jre_dir();
    let version_file = crate::updater::env::get_hycore_data_dir().join("version.txt");

    if game_dir.exists() {
        fs::remove_dir_all(&game_dir).map_err(|e| format!("Failed to remove game dir: {}", e))?;
    }

    if jre_dir.exists() {
        fs::remove_dir_all(&jre_dir).map_err(|e| format!("Failed to remove JRE dir: {}", e))?;
    }

    if version_file.exists() {
        fs::remove_file(&version_file)
            .map_err(|e| format!("Failed to remove version file: {}", e))?;
    }

    Ok(())
}
