use crate::error::AppError;
use crate::updater::env::get_hycore_data_dir;
use rusqlite::params;
use std::fs;
pub mod types;
pub use types::GameSettings;

pub fn get_settings_path() -> std::path::PathBuf {
    get_hycore_data_dir().join("settings.json")
}

pub fn load_settings() -> GameSettings {
    let conn = crate::database::get_conn();

    // Try to load from the new structured table
    let settings = conn.query_row(
        "SELECT ram_gb, custom_java_args, close_on_launch, minimize_to_tray, discord_rpc_enabled, channel, language, active_version, player_name, override_os, override_arch FROM launcher_settings WHERE id = 1",
        [],
        |row| {
            Ok(GameSettings {
                ram_gb: row.get(0)?,
                custom_java_args: row.get(1)?,
                close_on_launch: row.get::<_, i32>(2)? != 0,
                minimize_to_tray: row.get::<_, i32>(3)? != 0,
                discord_rpc_enabled: row.get::<_, i32>(4)? != 0,
                channel: row.get(5)?,
                language: row.get(6)?,
                active_version: row.get(7)?,
                player_name: row.get(8)?,
                override_os: row.get(9)?,
                override_arch: row.get(10)?,
            })
        }
    );

    if let Ok(settings) = settings {
        // Migration: If we just initialized and have 'Player' as name, check if we can migrate from old sources
        if settings.player_name == "Player" {
            // Check old 'global' JSON in 'settings' table
            let global_migration = conn.query_row(
                "SELECT value FROM settings WHERE key = 'global'",
                [],
                |row| row.get::<_, String>(0),
            );

            if let Ok(json_str) = global_migration {
                if let Ok(mut old_settings) = serde_json::from_str::<GameSettings>(&json_str) {
                    log::info!("Migrating settings from old JSON blob to columns");

                    // Also check for player.txt while we're at it (last chance)
                    let player_txt = get_hycore_data_dir().join("player.txt");
                    if player_txt.exists() {
                        if let Ok(name) = fs::read_to_string(&player_txt) {
                            old_settings.player_name = name.trim().to_string();
                            let _ = fs::remove_file(player_txt);
                        }
                    }

                    let _ = save_settings(&old_settings);
                    let _ = conn.execute("DELETE FROM settings WHERE key = 'global'", []);
                    return old_settings;
                }
            }

            // check for player.txt only
            let player_txt = get_hycore_data_dir().join("player.txt");
            if player_txt.exists() {
                if let Ok(name) = fs::read_to_string(&player_txt) {
                    let mut new_settings = settings.clone();
                    new_settings.player_name = name.trim().to_string();
                    let _ = fs::remove_file(player_txt);
                    let _ = save_settings(&new_settings);
                    return new_settings;
                }
            }
        }
        return settings;
    }

    GameSettings::default()
}

pub fn save_settings(settings: &GameSettings) -> anyhow::Result<()> {
    let conn = crate::database::get_conn();
    let mut clamped_ram = settings.ram_gb;
    {
        let total_gb = crate::system::info::get_total_ram_gb_internal().max(1);
        if clamped_ram < 1 {
            clamped_ram = 1;
        } else if clamped_ram > total_gb {
            clamped_ram = total_gb;
        }
    }

    conn.execute(
        "UPDATE launcher_settings SET 
            ram_gb = ?, 
            custom_java_args = ?, 
            close_on_launch = ?, 
            minimize_to_tray = ?, 
            discord_rpc_enabled = ?, 
            channel = ?, 
            language = ?, 
            active_version = ?, 
            player_name = ?,
            override_os = ?,
            override_arch = ?
        WHERE id = 1",
        params![
            clamped_ram,
            settings.custom_java_args,
            settings.close_on_launch as i32,
            settings.minimize_to_tray as i32,
            settings.discord_rpc_enabled as i32,
            settings.channel,
            settings.language,
            settings.active_version,
            settings.player_name,
            settings.override_os,
            settings.override_arch,
        ],
    )?;
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
