use std::fs;
use std::io::Write;

use crate::updater::env::get_hycore_data_dir;

pub fn get_player_name() -> String {
    let config_path = get_hycore_data_dir().join("player.txt");

    fs::read_to_string(&config_path)
        .ok()
        .and_then(|s| {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        })
        .unwrap_or_else(|| {
            std::env::var("USER")
                .or_else(|_| std::env::var("USERNAME"))
                .unwrap_or_else(|_| "Player".to_string())
        })
}

pub fn set_player_name(name: &str) -> Result<(), String> {
    let trimmed = name.trim();

    if trimmed.is_empty() {
        return Err("Player name cannot be empty".to_string());
    }

    if trimmed.len() > 32 {
        return Err("Player name too long (max 32 characters)".to_string());
    }

    let config_path = get_hycore_data_dir().join("player.txt");

    let mut file =
        fs::File::create(&config_path).map_err(|e| format!("Failed to save player name: {}", e))?;

    file.write_all(trimmed.as_bytes())
        .map_err(|e| format!("Failed to write player name: {}", e))?;

    Ok(())
}

#[tauri::command]
pub fn get_player_name_command() -> String {
    get_player_name()
}

#[tauri::command]
pub fn set_player_name_command(name: String) -> Result<(), String> {
    set_player_name(&name)
}
