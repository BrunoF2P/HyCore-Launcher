use crate::database::DbPool;
use crate::error::AppError;
use crate::updater::env::get_hycore_data_dir;
use std::fs;
pub mod types;
pub use types::GameSettings;

const LAUNCHER_SETTINGS_TABLE: redb::TableDefinition<'static, &str, &[u8]> =
    redb::TableDefinition::new("launcher_settings");

pub fn get_settings_path() -> std::path::PathBuf {
    get_hycore_data_dir().join("settings.json")
}

pub fn load_settings(pool: &DbPool) -> GameSettings {
    let read_txn = match pool.begin_read() {
        Ok(txn) => txn,
        Err(e) => {
            log::error!("Failed to begin read transaction: {}. Using defaults.", e);
            return GameSettings::default();
        }
    };

    let table = match read_txn.open_table(LAUNCHER_SETTINGS_TABLE) {
        Ok(t) => t,
        Err(e) => {
            log::error!(
                "Failed to open launcher_settings table: {}. Using defaults.",
                e
            );
            return GameSettings::default();
        }
    };

    let data = match table.get("default") {
        Ok(Some(value)) => value.value().to_vec(),
        Ok(None) => {
            log::info!("No settings found in database, using defaults");
            return GameSettings::default();
        }
        Err(e) => {
            log::error!("Failed to read settings: {}. Using defaults.", e);
            return GameSettings::default();
        }
    };

    let mut settings: GameSettings = match bincode::deserialize(&data) {
        Ok(s) => s,
        Err(e) => {
            log::error!("Failed to deserialize settings: {}. Using defaults.", e);
            return GameSettings::default();
        }
    };

    // Ensure player_id is never empty
    if settings.player_id.is_empty() {
        if !settings.player_name.is_empty() && settings.player_name != "Player" {
            log::info!(
                "Generating persistent ID based on current name: {}",
                settings.player_name
            );
            settings.player_id = crate::game::get_offline_uuid(&settings.player_name);
        } else {
            log::info!("Generating new random persistent ID");
            settings.player_id = uuid::Uuid::new_v4().to_string();
        }
        let _ = save_settings(pool, &settings);
    }

    // Migration check for old player.txt
    if settings.player_name == "Player" {
        let player_txt = get_hycore_data_dir().join("player.txt");
        if player_txt.exists() {
            if let Ok(name) = fs::read_to_string(&player_txt) {
                settings.player_name = name.trim().to_string();
                let _ = fs::remove_file(player_txt);
                let _ = save_settings(pool, &settings);
            }
        }
    }

    log::info!(
        "Settings loaded successfully (Player: {}, ID: {})",
        settings.player_name,
        settings.player_id
    );
    settings
}

pub fn save_settings(pool: &DbPool, settings: &GameSettings) -> anyhow::Result<()> {
    let mut clamped_settings = settings.clone();

    // Clamp RAM to system limits
    {
        let total_gb = crate::system::info::get_total_ram_gb_internal().max(1);
        if clamped_settings.ram_gb < 1 {
            clamped_settings.ram_gb = 1;
        } else if clamped_settings.ram_gb > total_gb {
            clamped_settings.ram_gb = total_gb;
        }
    }

    let data = bincode::serialize(&clamped_settings)
        .map_err(|e| anyhow::anyhow!("Failed to serialize settings: {}", e))?;

    let write_txn = pool
        .begin_write()
        .map_err(|e| anyhow::anyhow!("Failed to begin write transaction: {}", e))?;

    {
        let mut table = write_txn
            .open_table(LAUNCHER_SETTINGS_TABLE)
            .map_err(|e| anyhow::anyhow!("Failed to open launcher_settings table: {}", e))?;

        table
            .insert("default", data.as_slice())
            .map_err(|e| anyhow::anyhow!("Failed to insert settings: {}", e))?;
    }

    write_txn
        .commit()
        .map_err(|e| anyhow::anyhow!("Failed to commit transaction: {}", e))?;

    log::info!(
        "Settings saved successfully (Player: {}, ID: {})",
        clamped_settings.player_name,
        clamped_settings.player_id
    );
    Ok(())
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
