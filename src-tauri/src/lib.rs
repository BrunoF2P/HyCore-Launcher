mod game;
mod mods;
mod news;
mod player;
mod system;
mod updater;

#[tauri::command]
async fn get_news() -> Result<Vec<news::NewsItem>, String> {
    match news::fetch_news().await {
        Ok(items) => Ok(items),
        Err(_) => news::load_cache(),
    }
}

#[tauri::command]
async fn check_update_requirements() -> Result<updater::SystemRequirements, String> {
    Ok(updater::check_system_requirements().await)
}

#[tauri::command]
async fn check_for_game_update() -> Result<(bool, u32), String> {
    updater::is_update_available().await
}

#[tauri::command]
async fn start_game_update(window: tauri::Window) -> Result<(), String> {
    updater::run_update(window).await
}

#[tauri::command]
async fn log_updater_error(error: String) {
    updater::log_error(&error);
}

#[tauri::command]
async fn search_mods_cf(
    params: mods::api::SearchModsParams,
) -> Result<mods::types::SearchResult, String> {
    mods::api::search_mods(params).await
}

#[tauri::command]
async fn get_installed_mods() -> Result<Vec<mods::types::Mod>, String> {
    mods::manager::get_installed_mods()
}

#[tauri::command]
async fn install_mod_cf(
    window: tauri::Window,
    mod_id: i32,
    file_id: Option<i32>,
) -> Result<(), String> {
    mods::manager::install_mod_by_id(window, mod_id, file_id).await
}

#[tauri::command]
async fn remove_mod(mod_id: String) -> Result<(), String> {
    mods::manager::remove_mod(mod_id)
}

#[tauri::command]
async fn toggle_mod(mod_id: String, enabled: bool) -> Result<(), String> {
    mods::manager::toggle_mod(mod_id, enabled)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .invoke_handler(tauri::generate_handler![
            get_news,
            check_update_requirements,
            check_for_game_update,
            start_game_update,
            log_updater_error,
            updater::download::validate_pwr_file,
            game::launch_game,
            player::get_player_name_command,
            player::set_player_name_command,
            updater::java::get_java_bin_path,
            search_mods_cf,
            get_installed_mods,
            install_mod_cf,
            remove_mod,
            toggle_mod,
            system::open_game_folder,
            system::wipe_game_data,
            system::uninstall_game
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
