use super::types::ModManifest;
use crate::database::DbPool;
use crate::error::AppError;
use crate::updater::env::get_user_data_dir;
use redb::ReadableTable;
use std::fs;
use std::path::PathBuf;

use crate::database::{MODS_TABLE, PROFILES_TABLE, SETTINGS_TABLE};

pub fn get_mods_dir(pool: &DbPool) -> PathBuf {
    get_user_data_dir(pool).join("Mods")
}

pub fn get_modpacks_dir() -> PathBuf {
    let path = crate::updater::env::get_hycore_data_dir().join("Profiles");

    if !path.exists() {
        let old_path = crate::updater::env::get_hycore_data_dir()
            .join("UserData")
            .join("Mods")
            .join("Profiles");
        if old_path.exists() {
            log::info!("Migrating profiles directory from old location");
            let _ = fs::rename(&old_path, &path);
        }
    }

    path
}

pub fn get_active_profile_name_path() -> PathBuf {
    crate::updater::env::get_hycore_data_dir().join("active_profile.txt")
}

#[tauri::command]
pub fn get_active_profile_command(db_pool: tauri::State<DbPool>) -> String {
    get_active_profile(&db_pool)
}

pub fn get_active_profile(pool: &DbPool) -> String {
    let read_txn = match pool.begin_read() {
        Ok(txn) => txn,
        Err(_) => return "Default".to_string(),
    };

    let table = match read_txn.open_table(SETTINGS_TABLE) {
        Ok(t) => t,
        Err(_) => return "Default".to_string(),
    };

    if let Ok(Some(value)) = table.get("active_profile") {
        if let Ok(name) = std::str::from_utf8(value.value()) {
            return name.to_string();
        }
    }

    // Migration from old file-based storage
    let path = get_active_profile_name_path();
    if path.exists() {
        if let Ok(s) = fs::read_to_string(&path) {
            let name = s.trim().to_string();
            let _ = set_active_profile_name(pool, &name);
            return name;
        }
    }

    "Default".to_string()
}

pub fn set_active_profile_name(pool: &DbPool, name: &str) -> Result<(), AppError> {
    log::info!("Switching active profile to: {}", name);

    let write_txn = pool
        .begin_write()
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    {
        // Ensure profile exists in profiles table
        let mut profiles_table = write_txn
            .open_table(PROFILES_TABLE)
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        #[derive(serde::Serialize, serde::Deserialize)]
        struct Profile {
            name: String,
            created_at: String,
        }

        let profile = Profile {
            name: name.to_string(),
            created_at: time::OffsetDateTime::now_utc().to_string(),
        };

        let data =
            bincode::serialize(&profile).map_err(|e| AppError::DatabaseError(e.to_string()))?;

        profiles_table
            .insert(name, data.as_slice())
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        // Set active profile in settings
        let mut settings_table = write_txn
            .open_table(SETTINGS_TABLE)
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        settings_table
            .insert("active_profile", name.as_bytes())
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;
    }

    write_txn
        .commit()
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    Ok(())
}

pub fn get_manifest_path(pool: &DbPool) -> PathBuf {
    let active = get_active_profile(pool);
    get_modpacks_dir().join(format!("{}.json", active))
}

pub fn load_manifest(pool: &DbPool) -> Result<ModManifest, AppError> {
    let active = get_active_profile(pool);

    let read_txn = pool
        .begin_read()
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let table = read_txn
        .open_table(MODS_TABLE)
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    log::info!("Loading mods for profile {} from DB", active);

    let mut mods = Vec::new();
    let prefix = format!("{}::", active);

    // Iterate through all entries and filter by profile prefix
    let iter = table
        .iter()
        .map_err(|e: redb::StorageError| AppError::DatabaseError(e.to_string()))?;

    for item in iter {
        if let Ok((key, data)) = item {
            let key_str: &str = key.value();
            if key_str.starts_with(&prefix) {
                match bincode::deserialize::<super::types::Mod>(data.value()) {
                    Ok(mod_data) => mods.push(mod_data),
                    Err(e) => log::error!("Failed to deserialize mod {}: {}", key_str, e),
                }
            }
        }
    }

    // Migration logic from JSON
    if mods.is_empty() {
        let json_path = get_manifest_path(pool);
        if json_path.exists() {
            log::info!("Migrating mod manifest from JSON: {:?}", json_path);
            if let Ok(data) = fs::read(&json_path) {
                if let Ok(manifest) = serde_json::from_slice::<ModManifest>(&data) {
                    let _ = save_manifest(pool, &manifest);
                    return Ok(manifest);
                }
            }
        }
    }

    Ok(ModManifest {
        mods,
        version: "1.0".to_string(),
    })
}

pub fn save_manifest(pool: &DbPool, manifest: &ModManifest) -> Result<(), AppError> {
    let active = get_active_profile(pool);

    log::info!("Saving mod manifest for profile {} to DB", active);

    let write_txn = pool
        .begin_write()
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    {
        let mut table = write_txn
            .open_table(MODS_TABLE)
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        // Delete all mods for this profile
        let prefix = format!("{}::", active);
        let mut keys_to_delete = Vec::new();

        // Collect keys to delete
        let iter = table
            .iter()
            .map_err(|e: redb::StorageError| AppError::DatabaseError(e.to_string()))?;

        for item in iter {
            if let Ok((key, _)) = item {
                let key_str: &str = key.value();
                if key_str.starts_with(&prefix) {
                    keys_to_delete.push(key_str.to_string());
                }
            }
        }

        // Delete collected keys
        for key in keys_to_delete {
            table
                .remove(key.as_str())
                .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        }

        // Insert all mods for this profile
        for m in &manifest.mods {
            let key = format!("{}::{}", active, m.id);
            let data = bincode::serialize(m).map_err(|e| AppError::DatabaseError(e.to_string()))?;

            table
                .insert(key.as_str(), data.as_slice())
                .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        }
    }

    write_txn
        .commit()
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    Ok(())
}
