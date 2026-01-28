mod database;
pub mod error;
mod game;
pub mod http;
mod mods;
mod news;
pub mod platform;
mod player;
pub mod settings;
pub mod social;
mod system;
mod updater;
use tauri::{Emitter, Manager};

static APP_HANDLE: once_cell::sync::OnceCell<tauri::AppHandle> = once_cell::sync::OnceCell::new();

pub fn get_app_handle() -> &'static tauri::AppHandle {
    APP_HANDLE.get().expect("App handle not initialized")
}

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            APP_HANDLE.set(app.handle().clone()).unwrap();
            use tauri::menu::{Menu, MenuItem};
            use tauri::tray::TrayIconBuilder;
            use tauri::Manager;

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

            // Initialize database
            match database::init_db() {
                Ok(pool) => {
                    app.manage(pool);
                }
                Err(e) => {
                    let error_msg = format!(
                        "Falha ao inicializar o banco de dados: {}. O launcher não pode continuar.",
                        e
                    );
                    log::error!("{}", error_msg);
                    let _ = app.handle().emit("fatal-error", error_msg);
                    // We might want to exit or return early, but let's just log for now as per original logic,
                    // though managing a non-existent pool will panic commands.
                    // ideally we should probably exit.
                }
            }

            // Cleanup incomplete downloads/temp files
            if let Err(e) = updater::cleanup::cleanup_incomplete_downloads() {
                log::warn!("Cleanup failed (not fatal): {}", e);
            }

            // Initialize social integrations
            social::discord::init_discord();

            let show_i = MenuItem::with_id(app, "show", "Exibir Launcher", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "Sair", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_i, &quit_i])?;

            let _tray = if let Some(icon) = app.default_window_icon() {
                TrayIconBuilder::new()
                    .icon(icon.clone())
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
            } else {
                log::warn!("No default window icon found, skipping tray icon creation");
            };

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                use tauri::Manager;
                let pool = window.state::<crate::database::DbPool>();
                let settings = crate::settings::load_settings(&pool);
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
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .invoke_handler(tauri::generate_handler![
            news::get_news,
            updater::check_update_requirements,
            updater::check_for_game_update,
            updater::start_game_update,
            updater::download::validate_pwr_file,
            game::launch_game,
            player::get_player_name_command,
            player::set_player_name_command,
            updater::java_bin_path_command,
            mods::search_mods_cf,
            mods::get_installed_mods,
            mods::install_mod_cf,
            mods::remove_mod,
            mods::toggle_mod,
            mods::api::get_categories,
            mods::manifest::get_active_profile_command,
            mods::profiles::set_active_profile,
            mods::profiles::list_profiles,
            mods::profiles::create_profile,
            mods::profiles::delete_profile,
            mods::operations::check_mods_updates,
            system::open_game_folder,
            system::open_url,
            system::wipe_game_data,
            system::uninstall_game,
            system::info::get_system_ram_gb,
            settings::get_game_settings,
            settings::set_game_settings,
            updater::get_local_manifest_command,
            updater::switch_version_command,
            updater::get_available_versions_command,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
