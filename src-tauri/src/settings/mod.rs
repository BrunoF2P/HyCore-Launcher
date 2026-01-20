use crate::error::AppError;
use crate::updater::env::get_hycore_data_dir;
use std::fs;
pub mod types;
pub use types::GameSettings;

pub fn get_settings_path() -> std::path::PathBuf {
    get_hycore_data_dir().join("settings.json")
}

pub fn load_settings() -> GameSettings {
    let path = get_settings_path();
    if path.exists() {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(settings) = serde_json::from_str::<GameSettings>(&content) {
                return settings;
            }
        }
    }
    GameSettings::default()
}

pub fn save_settings(settings: &GameSettings) -> anyhow::Result<()> {
    let path = get_settings_path();
    let content = serde_json::to_string_pretty(settings)?;
    fs::write(path, content)?;
    Ok(())
}

#[tauri::command]
pub fn get_game_settings() -> GameSettings {
    load_settings()
}

#[tauri::command]
pub fn set_game_settings(settings: GameSettings) -> Result<(), AppError> {
    save_settings(&settings)?;
    crate::social::discord::set_rpc_enabled(settings.discord_rpc_enabled);
    Ok(())
}
