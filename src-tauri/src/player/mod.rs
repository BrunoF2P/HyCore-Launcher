use crate::settings::{load_settings, save_settings};

pub mod auth;

pub fn get_player_name() -> String {
    load_settings().player_name
}

pub fn set_player_name(name: &str) -> Result<(), String> {
    let trimmed = name.trim();

    if trimmed.is_empty() {
        return Err("Player name cannot be empty".to_string());
    }

    if trimmed.len() > 32 {
        return Err("Player name too long (max 32 characters)".to_string());
    }

    let mut settings = load_settings();
    settings.player_name = trimmed.to_string();

    save_settings(&settings).map_err(|e| format!("Failed to save settings: {}", e))?;

    Ok(())
}

#[tauri::command]
pub fn get_player_name_command() -> String {
    let name = get_player_name();
    log::info!("Player name requested (from DB): {}", name);
    name
}

#[tauri::command]
pub fn set_player_name_command(name: String) -> Result<(), String> {
    log::info!("Setting player name to: {} (in DB)", name);
    match set_player_name(&name) {
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
