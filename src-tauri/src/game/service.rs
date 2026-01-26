use crate::database::DbPool;
use crate::error::AppError;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;

pub trait GameHost: Send + Sync {
    fn ensure_java(&self) -> Pin<Box<dyn Future<Output = Result<PathBuf, AppError>> + Send + '_>>;
    fn exit(&self, code: i32);
}

pub struct TauriGameHost {
    pub app: tauri::AppHandle,
    pub window: tauri::Window,
}

impl TauriGameHost {
    pub fn new(app: tauri::AppHandle, window: tauri::Window) -> Self {
        Self { app, window }
    }
}

impl GameHost for TauriGameHost {
    fn ensure_java(&self) -> Pin<Box<dyn Future<Output = Result<PathBuf, AppError>> + Send + '_>> {
        let window = self.window.clone();
        Box::pin(async move {
            crate::updater::java::ensure_java(&window)
                .await
                .map_err(AppError::from)
        })
    }

    fn exit(&self, code: i32) {
        self.app.exit(code);
    }
}

pub struct GameService;

impl GameService {
    pub async fn launch_game<H: GameHost>(pool: &DbPool, host: &H) -> Result<(), AppError> {
        let game_dir = crate::updater::env::get_game_dir(pool);
        let client_dir = crate::updater::env::get_client_dir(pool);
        let user_dir = crate::updater::env::get_user_data_dir(pool);
        let settings = crate::settings::load_settings(pool);

        crate::social::discord::update_discord_status(
            "Jogando",
            &format!(
                "Perfil: {}",
                crate::mods::manifest::get_active_profile(pool)
            ),
        );

        // 1. Validation
        super::launch::validate_installation(&client_dir).await?;

        // 2. RAM Check
        super::launch::check_ram(&settings)?;

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
        let java_exec = host.ensure_java().await?;

        #[cfg(target_os = "windows")]
        let executable = client_dir.join("HytaleClient.exe");
        #[cfg(not(target_os = "windows"))]
        let executable = client_dir.join("HytaleClient");

        // Online Mode Patching
        if settings.online_mode {
            Self::handle_online_patches(&settings, &executable, &client_dir).await?;
        }

        // 5. Ensure Permissions (Linux/Unix) - MUST happen after patching
        #[cfg(not(target_os = "windows"))]
        {
            log::info!("Ensuring executable permissions after potential patching...");
            super::launch::ensure_permissions(&executable).await?;
        }

        // 6. Launch
        let player_name = crate::player::get_player_name(pool);
        let offline_uuid = settings.player_id.clone();

        let (auth_mode, identity_token, session_token) = if settings.online_mode {
            Self::authenticate(&offline_uuid, &player_name, &settings.auth_domain).await
        } else {
            ("offline".to_string(), None, None)
        };

        let mut args = super::launch::LaunchArgs {
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

        let jvm_args = super::launch::construct_jvm_args(&settings);

        super::launch::spawn_game_process(&executable, &java_exec, args, jvm_args, &client_dir)
            .await?;

        log::info!("Game launched successfully");

        if settings.close_on_launch {
            log::info!("Closing launcher as requested by settings");
            host.exit(0);
        }

        Ok(())
    }

    async fn handle_online_patches(
        settings: &crate::settings::GameSettings,
        executable: &PathBuf,
        client_dir: &PathBuf,
    ) -> Result<(), AppError> {
        log::info!("Online mode enabled, ensuring game is patched...");
        let patcher = super::patcher::ClientPatcher::new(Some(settings.auth_domain.clone()));

        let client_result = patcher.patch_client(executable);
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
        Ok(())
    }

    async fn authenticate(
        offline_uuid: &str,
        player_name: &str,
        auth_domain: &str,
    ) -> (String, Option<String>, Option<String>) {
        match crate::player::auth::fetch_auth_tokens(offline_uuid, player_name, auth_domain).await {
            Ok(tokens) => (
                "authenticated".to_string(),
                Some(tokens.identity_token),
                Some(tokens.session_token),
            ),
            Err(e) => {
                log::warn!(
                    "Failed to fetch online tokens, falling back to offline: {}",
                    e
                );
                ("offline".to_string(), None, None)
            }
        }
    }
}
