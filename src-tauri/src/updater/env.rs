use std::fs;
use std::path::PathBuf;

pub fn get_hycore_data_dir() -> PathBuf {
    let mut path = dirs::data_dir().unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    path.push("hycore");
    let _ = fs::create_dir_all(&path);
    path
}

pub fn get_game_dir() -> PathBuf {
    get_hycore_data_dir().join("game")
}

pub fn get_client_dir() -> PathBuf {
    get_game_dir().join("Client")
}

pub fn get_user_data_dir() -> PathBuf {
    get_hycore_data_dir().join("UserData")
}

pub fn get_jre_dir() -> PathBuf {
    get_hycore_data_dir().join("jre")
}

#[allow(dead_code)]
pub fn get_butler_dir() -> PathBuf {
    get_hycore_data_dir().join("butler")
}

pub fn get_java_binary() -> PathBuf {
    let jre_dir = get_jre_dir();

    #[cfg(target_os = "windows")]
    return jre_dir.join("bin").join("java.exe");

    #[cfg(not(target_os = "windows"))]
    return jre_dir.join("bin").join("java");
}
