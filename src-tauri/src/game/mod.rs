use crate::database::DbPool;
use crate::error::AppError;
use uuid::Uuid;

pub mod launch;
pub mod patcher;
pub mod service;

pub fn get_offline_uuid(nick: &str) -> String {
    let name = format!("OfflinePlayer:{}", nick.trim());
    let mut hash = *md5::compute(name.as_bytes());

    hash[6] = (hash[6] & 0x0f) | 0x30; // UUID v3 version
    hash[8] = (hash[8] & 0x3f) | 0x80; // UUID variant 1

    Uuid::from_bytes(hash).to_string()
}

static LAUNCHING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[tauri::command]
pub async fn launch_game(
    db_pool: tauri::State<'_, DbPool>,
    app: tauri::AppHandle,
    window: tauri::Window,
) -> Result<(), AppError> {
    if LAUNCHING.swap(true, std::sync::atomic::Ordering::SeqCst) {
        log::warn!("Launch already in progress, ignoring duplicate request");
        return Ok(());
    }

    let host = service::TauriGameHost::new(app, window);
    let result = service::GameService::launch_game(&db_pool, &host).await;

    LAUNCHING.store(false, std::sync::atomic::Ordering::SeqCst);

    if let Err(ref e) = result {
        log::error!("Game launch failed: {}", e);
    }

    result
}
