use super::types::ModManifest;
use crate::error::AppError;
use crate::updater::env::get_user_data_dir;
use std::fs;
use std::path::PathBuf;

pub fn get_mods_dir() -> PathBuf {
    get_user_data_dir().join("Mods")
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
pub fn get_active_profile() -> String {
    let conn = match crate::database::get_conn() {
        Ok(c) => c,
        Err(_) => return "Default".to_string(),
    };
    let stmt = conn
        .prepare("SELECT value FROM settings WHERE key = 'active_profile'")
        .ok();

    if let Some(mut stmt) = stmt {
        if let Ok(value) = stmt.query_row([], |row| row.get::<_, String>(0)) {
            return value;
        }
    }

    // Migration
    let path = get_active_profile_name_path();
    if path.exists() {
        if let Ok(s) = fs::read_to_string(&path) {
            let name = s.trim().to_string();
            let _ = set_active_profile_name(&name);
            return name;
        }
    }

    "Default".to_string()
}

pub fn set_active_profile_name(name: &str) -> Result<(), AppError> {
    log::info!("Switching active profile to: {}", name);
    let conn = crate::database::get_conn()?;

    // Ensure profile exists in profiles table
    conn.execute(
        "INSERT OR IGNORE INTO profiles (name, created_at) VALUES (?, ?)",
        [name, &time::OffsetDateTime::now_utc().to_string()],
    )
    .map_err(|e| AppError::Unknown(e.to_string()))?;

    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES ('active_profile', ?)",
        [name],
    )
    .map_err(|e| AppError::Unknown(e.to_string()))?;

    Ok(())
}

pub fn get_manifest_path() -> PathBuf {
    let active = get_active_profile();
    get_modpacks_dir().join(format!("{}.json", active))
}

pub fn load_manifest() -> Result<ModManifest, AppError> {
    let active = get_active_profile();
    let conn = crate::database::get_conn()?;

    log::info!("Loading mods for profile {} from DB", active);

    let mut stmt = conn
        .prepare(
            "SELECT id, name, slug, version, author, description, download_url, curse_forge_id, 
                file_id, enabled, installed_at, updated_at, file_path, icon_url, downloads, 
                category, latest_version, latest_file_id 
         FROM mods WHERE profile_name = ?",
        )
        .map_err(|e| AppError::Unknown(e.to_string()))?;

    let mod_iter = stmt
        .query_map([&active], |row| {
            Ok(super::types::Mod {
                id: row.get(0)?,
                name: row.get(1)?,
                slug: row.get(2)?,
                version: row.get(3)?,
                author: row.get(4)?,
                description: row.get(5)?,
                download_url: row.get(6)?,
                curse_forge_id: row.get(7)?,
                file_id: row.get(8)?,
                enabled: row.get::<_, i32>(9)? != 0,
                installed_at: row.get(10)?,
                updated_at: row.get(11)?,
                file_path: row.get(12)?,
                icon_url: row.get(13)?,
                downloads: row.get(14)?,
                category: row.get(15)?,
                latest_version: row.get(16)?,
                latest_file_id: row.get(17)?,
            })
        })
        .map_err(|e| AppError::Unknown(e.to_string()))?;

    let mut mods = Vec::new();
    for m in mod_iter {
        mods.push(m.map_err(|e| AppError::Unknown(e.to_string()))?);
    }

    // Migration logic
    if mods.is_empty() {
        let json_path = get_manifest_path();
        if json_path.exists() {
            log::info!("Migrating mod manifest from JSON: {:?}", json_path);
            if let Ok(data) = fs::read(&json_path) {
                if let Ok(manifest) = serde_json::from_slice::<ModManifest>(&data) {
                    let _ = save_manifest(&manifest);
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

pub fn save_manifest(manifest: &ModManifest) -> Result<(), AppError> {
    let active = get_active_profile();
    let conn = crate::database::get_conn()?;

    log::info!("Saving mod manifest for profile {} to DB", active);

    // Simple approach: delete and re-insert for the current profile
    // In a more complex app we'd diff, but for a launcher this is safe and easy.
    conn.execute("DELETE FROM mods WHERE profile_name = ?", [&active])
        .map_err(|e| AppError::Unknown(e.to_string()))?;

    for m in &manifest.mods {
        conn.execute(
            "INSERT INTO mods (id, profile_name, name, slug, version, author, description, 
                              download_url, curse_forge_id, file_id, enabled, installed_at, 
                              updated_at, file_path, icon_url, downloads, category, 
                              latest_version, latest_file_id) 
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            rusqlite::params![
                m.id,
                active,
                m.name,
                m.slug,
                m.version,
                m.author,
                m.description,
                m.download_url,
                m.curse_forge_id,
                m.file_id,
                if m.enabled { 1 } else { 0 },
                m.installed_at,
                m.updated_at,
                m.file_path,
                m.icon_url,
                m.downloads,
                m.category,
                m.latest_version,
                m.latest_file_id
            ],
        )
        .map_err(|e| AppError::Unknown(e.to_string()))?;
    }

    Ok(())
}
