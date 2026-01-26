use crate::database::DbPool;
use crate::error::AppError;
use crate::updater::env::{get_game_dir, get_hycore_data_dir, get_user_data_dir};
use std::fs;
use tauri_plugin_opener::OpenerExt;

pub mod info;

/// Opens a URL in the system default browser.
///
/// # Arguments
///
/// * `url` - The URL to open
///
/// # Errors
///
/// Returns an error if it fails to open the URL in the browser.
#[tauri::command]
pub fn open_url(app: tauri::AppHandle, url: String) -> Result<(), AppError> {
    log::info!("Opening URL: {}", url);
    app.opener()
        .open_url(&url, None::<&str>)
        .map_err(|e| AppError::Unknown(format!("Failed to open URL {}: {}", url, e)))?;
    Ok(())
}

/// Opens the game directory in the system file manager.
///
/// Creates the directory if it does not exist.
/// Emits a warning if the game is running.
///
/// # Errors
///
/// Returns an error if:
/// - Fails to create the directory
/// - Fails to open the file manager
#[tauri::command]
pub fn open_game_folder(
    db_pool: tauri::State<DbPool>,
    app: tauri::AppHandle,
) -> Result<(), AppError> {
    let path = get_game_dir(&db_pool);
    log::info!("Opening game folder: {:?}", path);

    if crate::platform::is_game_running() {
        log::warn!("User is opening game folder while game is running");
    }

    if !path.exists() {
        log::info!("Creating game folder: {:?}", path);
        fs::create_dir_all(&path)
            .map_err(|e| AppError::DirCreation(format!("Failed to create {:?}: {}", path, e)))?;
    }

    #[cfg(target_os = "linux")]
    {
        let _ = app;
        use std::process::Command;
        log::info!("Using xdg-open to open folder on Linux");
        Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| AppError::Unknown(format!("Failed to execute xdg-open: {}", e)))?;
    }

    #[cfg(not(target_os = "linux"))]
    {
        let path_str = path.to_string_lossy().to_string();
        app.opener()
            .open_path(&path_str, None::<&str>)
            .map_err(|e| AppError::Unknown(format!("Failed to open folder {:?}: {}", path, e)))?;
    }

    Ok(())
}

/// Removes all user game data (saves, settings, etc).
///
/// This operation is destructive and cannot be undone.
/// Does not remove the game files itself, only user data.
///
/// # Errors
///
/// Returns an error if:
/// - The game is running
/// - The path is invalid (security check)
/// - Fails to remove the directory
#[tauri::command]
pub fn wipe_game_data(db_pool: tauri::State<DbPool>) -> Result<(), AppError> {
    if crate::platform::is_game_running() {
        return Err(AppError::GameRunning);
    }

    let path = get_user_data_dir(&db_pool);
    let hycore_dir = get_hycore_data_dir();

    // Validate that the path is within the expected directory (security)
    if !path.starts_with(&hycore_dir) {
        log::error!("Security: Invalid path for wipe: {:?}", path);
        return Err(AppError::InvalidPath(format!("{:?}", path)));
    }

    log::info!("Wiping game data at {:?}", path);

    if path.exists() {
        fs::remove_dir_all(&path)
            .map_err(|e| AppError::Unknown(format!("Failed to wipe data at {:?}: {}", path, e)))?;
        log::info!("Game data wipe complete");
    } else {
        log::info!("No game data to wipe at {:?}", path);
    }

    Ok(())
}

/// Completely uninstalls the game, including:
/// - All game files
/// - Installed JRE
/// - Version files
///
/// This operation is destructive and cannot be undone.
/// Does not remove user data (saves, etc).
///
/// # Errors
///
/// Returns an error if:
/// - The game is running
/// - Any removal operation fails
///
/// Note: Attempts to remove all components even if some fail,
/// returning an aggregated error at the end.
#[tauri::command]
pub fn uninstall_game() -> Result<(), AppError> {
    if crate::platform::is_game_running() {
        return Err(AppError::GameRunning);
    }

    let app_dir = get_hycore_data_dir();
    let game_dir = app_dir.join("game");
    let jre_dir = app_dir.join("jre");

    log::info!("Uninstalling game (all versions)...");

    let mut errors = Vec::new();

    // Try to remove game directory
    if game_dir.exists() {
        log::info!("Removing entire game directory: {:?}", game_dir);
        if let Err(e) = fs::remove_dir_all(&game_dir) {
            log::error!("Failed to remove game directory: {}", e);
            errors.push(format!("game directory ({:?}): {}", game_dir, e));
        } else {
            log::info!("Game directory removed successfully");
        }
    } else {
        log::info!("Game directory does not exist, skipping");
    }

    // Try to remove JRE directory
    if jre_dir.exists() {
        log::info!("Removing JRE directory: {:?}", jre_dir);
        if let Err(e) = fs::remove_dir_all(&jre_dir) {
            log::error!("Failed to remove JRE directory: {}", e);
            errors.push(format!("JRE directory ({:?}): {}", jre_dir, e));
        } else {
            log::info!("JRE directory removed successfully");
        }
    } else {
        log::info!("JRE directory does not exist, skipping");
    }

    // Try to remove version files
    if let Err(e) = crate::updater::cleanup::remove_version_files() {
        log::error!("Failed to remove version files: {}", e);
        errors.push(format!("version files: {}", e));
    } else {
        log::info!("Version files removed successfully");
    }

    // If there was any error, return aggregated error
    if !errors.is_empty() {
        let error_msg = errors.join("; ");
        log::error!("Uninstallation completed with errors: {}", error_msg);
        return Err(AppError::PartialUninstall(error_msg));
    }

    log::info!("Uninstallation complete - all components removed successfully");
    Ok(())
}
