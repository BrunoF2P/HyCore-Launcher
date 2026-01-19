mod game;
mod mods;
mod news;
mod player;
pub mod settings;
pub mod social;
mod system;
mod updater;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[tauri::command]
async fn get_news() -> Result<Vec<news::NewsItem>, String> {
    log::info!("Fetching news...");
    match news::fetch_news().await {
        Ok(items) => {
            log::info!("News fetched successfully ({} items)", items.len());
            Ok(items)
        }
        Err(e) => {
            log::warn!("Failed to fetch news from server, loading cache: {}", e);
            news::load_cache()
        }
    }
}

#[tauri::command]
async fn check_update_requirements() -> Result<updater::SystemRequirements, String> {
    log::info!("Checking update requirements...");
    let reqs = updater::check_system_requirements().await;
    log::info!(
        "Requirements checked: meets_requirements={}",
        reqs.meets_requirements
    );
    Ok(reqs)
}

#[tauri::command]
async fn check_for_game_update() -> Result<(bool, u32), String> {
    log::info!("Checking for game update...");
    match updater::is_update_available().await {
        Ok(res) => {
            log::info!("Game update check: available={}, version={}", res.0, res.1);
            Ok(res)
        }
        Err(e) => {
            log::error!("Failed to check for game update: {}", e);
            Err(e)
        }
    }
}

#[tauri::command]
async fn start_game_update(window: tauri::Window) -> Result<(), String> {
    log::info!("Starting game update process...");
    match updater::run_update(window).await {
        Ok(_) => {
            log::info!("Game update process finished successfully");
            Ok(())
        }
        Err(e) => {
            log::error!("Game update process failed: {}", e);
            Err(e)
        }
    }
}

#[tauri::command]
async fn search_mods_cf(
    params: mods::api::SearchModsParams,
) -> Result<mods::types::SearchResult, String> {
    log::info!("Searching mods with params: {:?}", params);
    match mods::api::search_mods(params).await {
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
async fn get_installed_mods() -> Result<Vec<mods::types::Mod>, String> {
    log::info!("Fetching installed mods...");
    match mods::operations::get_installed_mods() {
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
async fn install_mod_cf(
    window: tauri::Window,
    mod_id: i32,
    file_id: Option<i32>,
) -> Result<(), String> {
    log::info!("Installing mod_id: {:?}, file_id: {:?}", mod_id, file_id);
    match mods::operations::install_mod_by_id(window, mod_id, file_id).await {
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
async fn remove_mod(mod_id: String) -> Result<(), String> {
    log::info!("Removing mod: {}", mod_id);
    match mods::operations::remove_mod(mod_id) {
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
async fn toggle_mod(mod_id: String, enabled: bool) -> Result<(), String> {
    log::info!("Toggling mod {} (enabled={})", mod_id, enabled);
    match mods::operations::toggle_mod(mod_id, enabled) {
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

#[tauri::command]
async fn java_bin_path_command() -> std::path::PathBuf {
    log::info!("Frontend requested Java binary path");
    updater::java::get_java_bin_path()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    use tauri::Manager;
    tauri::Builder::default()
        .setup(|app| {
            use tauri::menu::{Menu, MenuItem};
            use tauri::tray::TrayIconBuilder;

            // Initialize logging
            let _ = app.handle().plugin(
                tauri_plugin_log::Builder::default()
                    .targets([
                        tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Stdout),
                        tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::LogDir {
                            file_name: Some("log".to_string()),
                        }),
                        tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Webview),
                    ])
                    .level(log::LevelFilter::Info)
                    .build(),
            );

            log::info!("Launcher starting...");

            // Initialize social integrations
            social::discord::init_discord();

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
                let settings = crate::settings::load_settings();
                if settings.minimize_to_tray {
                    api.prevent_close();
                    let _ = window.hide();
                }
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
            updater::download::validate_pwr_file,
            game::launch_game,
            player::get_player_name_command,
            player::set_player_name_command,
            java_bin_path_command,
            search_mods_cf,
            get_installed_mods,
            install_mod_cf,
            remove_mod,
            toggle_mod,
            mods::api::get_categories,
            mods::manifest::get_active_profile,
            mods::profiles::set_active_profile,
            mods::profiles::list_profiles,
            mods::profiles::create_profile,
            mods::profiles::delete_profile,
            mods::operations::check_mods_updates,
            system::open_game_folder,
            system::open_url,
            system::wipe_game_data,
            system::uninstall_game,
            settings::get_game_settings,
            settings::set_game_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
