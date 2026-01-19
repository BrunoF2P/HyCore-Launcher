mod game;
mod mods;
mod news;
mod player;
mod system;
mod updater;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

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
    use tauri::Manager;
    tauri::Builder::default()
        .setup(|app| {
            use tauri::menu::{Menu, MenuItem};
            use tauri::tray::TrayIconBuilder;

            let show_i = MenuItem::with_id(app, "show", "Exibir Launcher", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "Sair", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_i, &quit_i])?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(false) // Better to toggle on click, menu on right click
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::Click {
                        button: tauri::tray::MouseButton::Left,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let is_visible = window.is_visible().unwrap_or(false);
                            if is_visible {
                                let _ = window.hide();
                            } else {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            let _ = app
                .get_webview_window("main")
                .expect("no main window")
                .show();
            let _ = app
                .get_webview_window("main")
                .expect("no main window")
                .set_focus();
        }))
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
            mods::api::get_categories,
            mods::manager::get_active_profile,
            mods::manager::set_active_profile,
            mods::manager::list_profiles,
            mods::manager::create_profile,
            mods::manager::delete_profile,
            mods::manager::check_mods_updates,
            system::open_game_folder,
            system::open_url,
            system::wipe_game_data,
            system::uninstall_game,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
