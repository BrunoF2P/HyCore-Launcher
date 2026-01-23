use crate::error::AppError;
use uuid::Uuid;

pub mod launch;

pub fn get_offline_uuid(nick: &str) -> String {
    // Deterministic UUID v3 (MD5 based) matching community standard
    // Namespace Nil + "OfflinePlayer:Nick"
    let namespace = Uuid::nil();
    let name = format!("OfflinePlayer:{}", nick.trim());
    Uuid::new_v3(&namespace, name.as_bytes()).to_string()
}

#[tauri::command]
pub async fn launch_game(app: tauri::AppHandle, window: tauri::Window) -> Result<(), AppError> {
    let game_dir = crate::updater::env::get_game_dir();
    let client_dir = crate::updater::env::get_client_dir();
    let user_dir = crate::updater::env::get_user_data_dir();
    let settings = crate::settings::load_settings();

    crate::social::discord::update_discord_status(
        "Jogando",
        &format!("Perfil: {}", crate::mods::manifest::get_active_profile()),
    );

    // 1. Validation
    launch::validate_installation(&client_dir).await?;

    // 2. RAM Check
    launch::check_ram(&settings)?;

    // 3. Prepare Paths
    let app_dir_str = game_dir
        .to_str()
        .ok_or_else(|| AppError::Unknown("Invalid game directory path".to_string()))?
        .to_string();
    let user_dir_str = user_dir
        .to_str()
        .ok_or_else(|| AppError::Unknown("Invalid UserData path".to_string()))?
        .to_string();

    tokio::fs::create_dir_all(&user_dir)
        .await
        .map_err(|e| AppError::DirCreation(e.to_string()))?;

    // 4. Ensure Java
    let java_exec = crate::updater::java::ensure_java(&window)
        .await
        .map_err(AppError::from)?;

    // 5. Ensure Permissions (Linux)
    #[cfg(not(target_os = "windows"))]
    {
        let executable = client_dir.join("HytaleClient");
        launch::ensure_permissions(&executable).await?;
    }

    #[cfg(target_os = "windows")]
    let executable = client_dir.join("HytaleClient.exe");
    #[cfg(not(target_os = "windows"))]
    let executable = client_dir.join("HytaleClient");

    // 6. Launch
    let player_name = crate::player::get_player_name();
    let offline_uuid = get_offline_uuid(&player_name);

    let args = launch::LaunchArgs {
        app_dir: app_dir_str,
        user_dir: user_dir_str,
        uuid: offline_uuid,
        name: player_name,
        auth_mode: "Offline".to_string(),
        width: 1280,
        height: 720,
    };

    let jvm_args = launch::construct_jvm_args(&settings);

    launch::spawn_game_process(&executable, &java_exec, args, jvm_args, &client_dir).await?;

    log::info!("Game launched successfully");

    if settings.close_on_launch {
        log::info!("Closing launcher as requested by settings");
        app.exit(0);
    }

    Ok(())
}
