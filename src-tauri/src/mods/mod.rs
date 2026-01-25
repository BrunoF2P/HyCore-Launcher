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
    let res = api::search_mods(params).await?;
    log::info!("Mod search returned {} results", res.mods.len());
    Ok(res)
}

#[tauri::command]
pub async fn get_installed_mods() -> Result<Vec<types::Mod>, AppError> {
    log::info!("Fetching installed mods...");
    let mods_list = operations::get_installed_mods()?;
    log::info!("Found {} installed mods", mods_list.len());
    Ok(mods_list)
}

#[tauri::command]
pub async fn install_mod_cf(
    window: tauri::Window,
    mod_id: i32,
    file_id: Option<i32>,
) -> Result<(), AppError> {
    log::info!("Installing mod_id: {:?}, file_id: {:?}", mod_id, file_id);
    operations::install_mod_by_id(window, mod_id, file_id).await?;
    log::info!("Mod installed successfully");
    Ok(())
}

#[tauri::command]
pub async fn remove_mod(mod_id: String) -> Result<(), AppError> {
    log::info!("Removing mod: {}", mod_id);
    operations::remove_mod(mod_id)?;
    log::info!("Mod removed successfully");
    Ok(())
}

#[tauri::command]
pub async fn toggle_mod(mod_id: String, enabled: bool) -> Result<(), AppError> {
    log::info!("Toggling mod {} (enabled={})", mod_id, enabled);
    operations::toggle_mod(mod_id, enabled)?;
    log::info!("Mod toggled successfully");
    Ok(())
}
