pub mod api;
pub mod manifest;
pub mod operations;
pub mod profiles;
pub mod types;

use crate::error::AppError;

#[tauri::command]
pub async fn search_mods_cf(
    params: api::SearchModsParams,
) -> Result<types::SearchResult, AppError> {
    log::info!("Searching mods with params: {:?}", params);
    match api::search_mods(params).await {
        Ok(res) => {
            log::info!("Mod search returned {} results", res.mods.len());
            Ok(res)
        }
        Err(e) => {
            log::error!("Mod search failed: {}", e);
            Err(e)
        }
    }
}

#[tauri::command]
pub async fn get_installed_mods() -> Result<Vec<types::Mod>, AppError> {
    log::info!("Fetching installed mods...");
    match operations::get_installed_mods() {
        Ok(mods_list) => {
            log::info!("Found {} installed mods", mods_list.len());
            Ok(mods_list)
        }
        Err(e) => {
            log::error!("Failed to fetch installed mods: {}", e);
            Err(e)
        }
    }
}

#[tauri::command]
pub async fn install_mod_cf(
    window: tauri::Window,
    mod_id: i32,
    file_id: Option<i32>,
) -> Result<(), AppError> {
    log::info!("Installing mod_id: {:?}, file_id: {:?}", mod_id, file_id);
    match operations::install_mod_by_id(window, mod_id, file_id).await {
        Ok(_) => {
            log::info!("Mod installed successfully");
            Ok(())
        }
        Err(e) => {
            log::error!("Failed to install mod: {}", e);
            Err(e)
        }
    }
}

#[tauri::command]
pub async fn remove_mod(mod_id: String) -> Result<(), AppError> {
    log::info!("Removing mod: {}", mod_id);
    match operations::remove_mod(mod_id) {
        Ok(_) => {
            log::info!("Mod removed successfully");
            Ok(())
        }
        Err(e) => {
            log::error!("Failed to remove mod: {}", e);
            Err(e)
        }
    }
}

#[tauri::command]
pub async fn toggle_mod(mod_id: String, enabled: bool) -> Result<(), AppError> {
    log::info!("Toggling mod {} (enabled={})", mod_id, enabled);
    match operations::toggle_mod(mod_id, enabled) {
        Ok(_) => {
            log::info!("Mod toggled successfully");
            Ok(())
        }
        Err(e) => {
            log::error!("Failed to toggle mod: {}", e);
            Err(e)
        }
    }
}
