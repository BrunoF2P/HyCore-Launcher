use crate::database::{get_conn, with_transaction, DbPool};
use crate::error::AppError;
use crate::updater::env::get_hycore_data_dir;
use rusqlite::params;
use std::fs;
pub mod types;
pub use types::GameSettings;

pub fn get_settings_path() -> std::path::PathBuf {
    get_hycore_data_dir().join("settings.json")
}

pub fn load_settings(pool: &DbPool) -> GameSettings {
    let conn = match get_conn(pool) {
        Ok(c) => c,
        Err(e) => {
            log::error!(
                "Failed to open DB for loading settings: {}. Using defaults.",
                e
            );
            return GameSettings::default();
        }
    };

    // Try to load from the new structured table
    let settings = conn.query_row(
        "SELECT ram_gb, custom_java_args, close_on_launch, minimize_to_tray, discord_rpc_enabled, channel, language, active_version, player_name, override_os, override_arch, online_mode, auth_domain, player_id FROM launcher_settings WHERE id = 1",
        [],
        |row| {
            Ok(GameSettings {
                ram_gb: row.get(0).unwrap_or(4),
                custom_java_args: row.get(1).unwrap_or_else(|_| "".to_string()),
                close_on_launch: row.get::<_, Option<i32>>(2).unwrap_or(Some(0)).unwrap_or(0) != 0,
                minimize_to_tray: row.get::<_, Option<i32>>(3).unwrap_or(Some(1)).unwrap_or(1) != 0,
                discord_rpc_enabled: row.get::<_, Option<i32>>(4).unwrap_or(Some(1)).unwrap_or(1) != 0,
                channel: row.get(5).unwrap_or_else(|_| "release".to_string()),
                language: row.get(6).unwrap_or_else(|_| "auto".to_string()),
                active_version: row.get(7).unwrap_or(0),
                player_name: row.get(8).unwrap_or_else(|_| "Player".to_string()),
                override_os: row.get(9).ok(),
                override_arch: row.get(10).ok(),
                online_mode: row.get::<_, Option<i32>>(11).unwrap_or(Some(1)).unwrap_or(1) != 0,
                auth_domain: row.get(12).unwrap_or_else(|_| "sanasol.ws".to_string()),
                player_id: row.get(13).unwrap_or_else(|_| "".to_string()),
            })
        }
    );

    let mut current_settings = match settings {
        Ok(s) => {
            log::info!(
                "Settings loaded successfully from DB (Player: {}, ID: {})",
                s.player_name,
                s.player_id
            );
            s
        }
        Err(e) => {
            log::warn!(
                "Settings column missing or row not found: {}. Using temporary defaults.",
                e
            );
            return GameSettings::default(); // Return default but DO NOT SAVE it yet
        }
    };

    // Ensure player_id is never empty
    if current_settings.player_id.is_empty() {
        // If we have a name, use it as seed for reproducibility if data was lost
        if !current_settings.player_name.is_empty() && current_settings.player_name != "Player" {
            log::info!(
                "Generating persistent ID based on current name: {}",
                current_settings.player_name
            );
            current_settings.player_id =
                crate::game::get_offline_uuid(&current_settings.player_name);
        } else {
            log::info!("Generating new random persistent ID");
            current_settings.player_id = uuid::Uuid::new_v4().to_string();
        }

        // Save the generated ID immediately so it stays fixed
        let _ = save_settings(pool, &current_settings);
    }

    if current_settings.player_name == "Player" {
        // Check old 'global' JSON in 'settings' table
        let global_migration: std::result::Result<String, _> = conn.query_row(
            "SELECT value FROM settings WHERE key = 'global'",
            [],
            |row| row.get(0),
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

                let _ = save_settings(pool, &old_settings);
                let _ = conn.execute("DELETE FROM settings WHERE key = 'global'", []);
                return old_settings;
            }
        }

        // check for player.txt only
        let player_txt = get_hycore_data_dir().join("player.txt");
        if player_txt.exists() {
            if let Ok(name) = fs::read_to_string(&player_txt) {
                current_settings.player_name = name.trim().to_string();
                let _ = fs::remove_file(player_txt);
                let _ = save_settings(pool, &current_settings);
            }
        }
    }
    current_settings
}

pub fn save_settings(pool: &DbPool, settings: &GameSettings) -> anyhow::Result<()> {
    // using with_transaction for concurrent write safety
    with_transaction(pool, |conn| {
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
                override_arch = ?,
                online_mode = ?,
                auth_domain = ?,
                player_id = ?
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
                settings.online_mode as i32,
                settings.auth_domain,
                settings.player_id,
            ],
        )?;

        log::info!(
            "Settings saved successfully (Player: {}, ID: {})",
            settings.player_name,
            settings.player_id
        );
        Ok(())
    })
    .map_err(|e| {
        log::error!("Failed to save settings: {}", e);
        anyhow::anyhow!("Database error: {}", e)
    })
}

#[tauri::command]
pub fn get_game_settings(db_pool: tauri::State<DbPool>) -> GameSettings {
    load_settings(&db_pool)
}

#[tauri::command]
pub fn set_game_settings(
    db_pool: tauri::State<DbPool>,
    mut settings: GameSettings,
) -> Result<(), AppError> {
    let current = load_settings(&db_pool);

    // Safety check: Don't allow clearing player_id or player_name via simple setting updates
    if settings.player_id.is_empty() && !current.player_id.is_empty() {
        settings.player_id = current.player_id;
    }

    if (settings.player_name.is_empty() || settings.player_name == "Player")
        && current.player_name != "Player"
        && !current.player_name.is_empty()
    {
        settings.player_name = current.player_name;
    }

    save_settings(&db_pool, &settings)?;
    crate::social::discord::set_rpc_enabled(settings.discord_rpc_enabled);
    Ok(())
}
