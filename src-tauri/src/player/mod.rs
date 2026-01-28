use crate::database::DbPool;
use crate::settings::{load_settings, save_settings};

pub mod auth;

pub fn get_player_name(pool: &DbPool) -> String {
    load_settings(pool).player_name
}

pub fn set_player_name(pool: &DbPool, name: &str) -> Result<(), String> {
    let trimmed = name.trim();

    if trimmed.is_empty() {
        return Err("Player name cannot be empty".to_string());
    }

    if trimmed.len() > 32 {
        return Err("Player name too long (max 32 characters)".to_string());
    }

    let mut settings = load_settings(pool);
    settings.player_name = trimmed.to_string();

    save_settings(pool, &settings).map_err(|e| format!("Failed to save settings: {}", e))?;

    Ok(())
}

/// Returns the persistent player UUID used for online identity and cosmetics.
pub fn get_player_uuid(pool: &DbPool) -> String {
    load_settings(pool).player_id
}

/// Resets the persistent player UUID. The next settings load will generate a new one.
pub fn reset_player_uuid(pool: &DbPool) -> anyhow::Result<()> {
    let mut settings = load_settings(pool);
    settings.player_id.clear();
    save_settings(pool, &settings)?;
    Ok(())
}

#[tauri::command]
pub fn get_player_name_command(db_pool: tauri::State<DbPool>) -> String {
    let name = get_player_name(&db_pool);
    log::info!("Player name requested (from DB): {}", name);
    name
}

#[tauri::command]
pub fn set_player_name_command(db_pool: tauri::State<DbPool>, name: String) -> Result<(), String> {
    log::info!("Setting player name to: {} (in DB)", name);
    match set_player_name(&db_pool, &name) {
        Ok(_) => {
            log::info!("Player name updated successfully in DB");
            Ok(())
        }
        Err(e) => {
            log::error!("Failed to update player name: {}", e);
            Err(e)
        }
    }
}

#[tauri::command]
pub fn get_player_uuid_command(db_pool: tauri::State<DbPool>) -> Result<String, String> {
    let uuid = get_player_uuid(&db_pool);
    if uuid.is_empty() {
        Err("Player UUID is not initialized yet".to_string())
    } else {
        log::info!("Player UUID requested (from DB): {}", uuid);
        Ok(uuid)
    }
}

#[tauri::command]
pub fn reset_player_uuid_command(db_pool: tauri::State<DbPool>) -> Result<(), String> {
    log::warn!("Player UUID reset requested");
    reset_player_uuid(&db_pool).map_err(|e| {
        log::error!("Failed to reset player UUID: {}", e);
        e.to_string()
    })
}
