use crate::database::DbPool;
use std::fs;
use std::path::PathBuf;

pub fn get_hycore_data_dir() -> PathBuf {
    let mut path = dirs::data_dir().unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    path.push("hycore");
    let _ = fs::create_dir_all(&path);
    path
}

pub fn get_game_dir(pool: &DbPool) -> PathBuf {
    let active = crate::mods::manifest::get_active_profile(pool);

    // If we have an active version in settings, use the SHARED versions folder
    let settings = crate::settings::load_settings(pool);
    if settings.active_version > 0 {
        return get_hycore_data_dir()
            .join("game")
            .join("versions")
            .join(settings.active_version.to_string());
    }

    // Fallback to profile-specific game folder for legacy/non-versioned
    if active == "Default" {
        get_hycore_data_dir().join("game")
    } else {
        get_hycore_data_dir()
            .join("instances")
            .join(active)
            .join("game")
    }
}

pub fn get_version_dir(version: u32) -> PathBuf {
    // Versions are ALWAYS stored in the global 'game/versions' folder
    get_hycore_data_dir()
        .join("game")
        .join("versions")
        .join(version.to_string())
}

pub fn get_client_dir(pool: &DbPool) -> PathBuf {
    get_game_dir(pool).join("Client")
}

pub fn get_user_data_dir(pool: &DbPool) -> PathBuf {
    let active = crate::mods::manifest::get_active_profile(pool);
    get_user_data_dir_for_profile(&active)
}

pub fn get_user_data_dir_for_profile(profile_name: &str) -> PathBuf {
    if profile_name == "Default" {
        get_hycore_data_dir().join("UserData")
    } else {
        get_hycore_data_dir()
            .join("instances")
            .join(profile_name)
            .join("UserData")
    }
}

pub fn get_versions_manifest_path() -> PathBuf {
    // Shared manifest for ALL profiles to know what is downloaded
    get_hycore_data_dir().join("game").join("versions.json")
}

pub fn get_version_file_path() -> PathBuf {
    get_versions_manifest_path()
}

pub fn get_jre_dir() -> PathBuf {
    get_hycore_data_dir().join("jre")
}

#[allow(dead_code)]
pub fn get_butler_dir() -> PathBuf {
    get_hycore_data_dir().join("butler")
}
