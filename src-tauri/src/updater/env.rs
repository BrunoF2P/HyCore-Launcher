use std::fs;
use std::path::PathBuf;

pub fn get_hycore_data_dir() -> PathBuf {
    let mut path = dirs::data_dir().unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    path.push("hycore");
    let _ = fs::create_dir_all(&path);
    path
}

pub fn get_game_dir() -> PathBuf {
    let active = crate::mods::manifest::get_active_profile();
    if active == "Default" {
        get_hycore_data_dir().join("game")
    } else {
        get_hycore_data_dir()
            .join("instances")
            .join(active)
            .join("game")
    }
}

pub fn get_client_dir() -> PathBuf {
    get_game_dir().join("Client")
}

pub fn get_user_data_dir() -> PathBuf {
    let active = crate::mods::manifest::get_active_profile();
    if active == "Default" {
        get_hycore_data_dir().join("UserData")
    } else {
        get_hycore_data_dir()
            .join("instances")
            .join(active)
            .join("UserData")
    }
}

pub fn get_version_file_path() -> PathBuf {
    let active = crate::mods::manifest::get_active_profile();
    if active == "Default" {
        get_hycore_data_dir().join("version.json")
    } else {
        get_hycore_data_dir()
            .join("instances")
            .join(active)
            .join("version.json")
    }
}

pub fn get_legacy_version_file_path() -> PathBuf {
    let active = crate::mods::manifest::get_active_profile();
    if active == "Default" {
        get_hycore_data_dir().join("version.txt")
    } else {
        get_hycore_data_dir()
            .join("instances")
            .join(active)
            .join("version.txt")
    }
}

pub fn get_jre_dir() -> PathBuf {
    get_hycore_data_dir().join("jre")
}

#[allow(dead_code)]
pub fn get_butler_dir() -> PathBuf {
    get_hycore_data_dir().join("butler")
}
