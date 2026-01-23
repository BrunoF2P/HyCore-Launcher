use crate::error::AppError;
use uuid::Uuid;

pub mod launch;
pub mod patcher;

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
        launch::ensure_permissions(&client_dir.join("HytaleClient")).await?;
    }

    #[cfg(target_os = "windows")]
    let executable = client_dir.join("HytaleClient.exe");
    #[cfg(not(target_os = "windows"))]
    let executable = client_dir.join("HytaleClient");

    // Online Mode Patching
    if settings.online_mode {
        log::info!("Online mode enabled, ensuring game is patched...");
        let patcher = patcher::ClientPatcher::new(Some(settings.auth_domain.clone()));

        let client_result = patcher.patch_client(&executable);
        if !client_result.success {
            return Err(AppError::Unknown(format!(
                "Patching failed: {}",
                client_result.error.unwrap_or_default()
            )));
        }

        let server_jar = client_dir
            .parent()
            .unwrap()
            .join("Server")
            .join("HytaleServer.jar");
        if server_jar.exists() {
            let server_result = patcher.patch_server(&server_jar);
            if !server_result.success {
                log::warn!(
                    "Server patching failed (non-fatal): {}",
                    server_result.error.unwrap_or_default()
                );
            }
        }

        #[cfg(target_os = "macos")]
        {
            let app_path = client_dir.join("Hytale.app");
            if app_path.exists() {
                patcher
                    .sign_macos_app(&app_path)
                    .await
                    .map_err(|e| AppError::Unknown(e.to_string()))?;
            }
        }
    }

    // 6. Launch
    let player_name = crate::player::get_player_name();
    let offline_uuid = settings.player_id.clone();

    let mut auth_mode = "offline".to_string();
    let mut identity_token = None;
    let mut session_token = None;

    if settings.online_mode {
        match crate::player::auth::fetch_auth_tokens(
            &offline_uuid,
            &player_name,
            &settings.auth_domain,
        )
        .await
        {
            Ok(tokens) => {
                auth_mode = "authenticated".to_string();
                identity_token = Some(tokens.identity_token);
                session_token = Some(tokens.session_token);
            }
            Err(e) => {
                log::warn!(
                    "Failed to fetch online tokens, falling back to offline: {}",
                    e
                );
            }
        }
    }

    let mut args = launch::LaunchArgs {
        app_dir: app_dir_str,
        user_dir: user_dir_str,
        uuid: offline_uuid,
        name: player_name,
        auth_mode,
        identity_token: None,
        session_token: None,
    };

    if let (Some(ident), Some(sess)) = (identity_token, session_token) {
        args.identity_token = Some(ident);
        args.session_token = Some(sess);
    }

    let jvm_args = launch::construct_jvm_args(&settings);

    launch::spawn_game_process(&executable, &java_exec, args, jvm_args, &client_dir).await?;

    log::info!("Game launched successfully");

    if settings.close_on_launch {
        log::info!("Closing launcher as requested by settings");
        app.exit(0);
    }

    Ok(())
}
