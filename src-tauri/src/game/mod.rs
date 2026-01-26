use crate::database::DbPool;
use crate::error::AppError;
use uuid::Uuid;

pub mod launch;
pub mod patcher;
pub mod service;

pub fn get_offline_uuid(nick: &str) -> String {
    let name = format!("OfflinePlayer:{}", nick.trim());
    let mut hash = *md5::compute(name.as_bytes());

    // Standard UUID v3 format (MD5 based)
    // Set version to 3
    hash[6] = (hash[6] & 0x0f) | 0x30;
    // Set variant to 1
    hash[8] = (hash[8] & 0x3f) | 0x80;

    Uuid::from_bytes(hash).to_string()
}

#[tauri::command]
pub async fn launch_game(
    db_pool: tauri::State<'_, DbPool>,
    app: tauri::AppHandle,
    window: tauri::Window,
) -> Result<(), AppError> {
    let host = service::TauriGameHost::new(app, window);
    service::GameService::launch_game(&db_pool, &host).await
}
