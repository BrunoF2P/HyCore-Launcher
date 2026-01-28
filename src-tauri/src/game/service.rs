use crate::database::DbPool;
use crate::error::AppError;
use std::future::Future;
use tauri::Emitter;
use std::path::PathBuf;
use std::pin::Pin;

pub trait GameHost: Send + Sync {
    fn ensure_java(&self) -> Pin<Box<dyn Future<Output = Result<PathBuf, AppError>> + Send + '_>>;
    fn exit(&self, code: i32);
    /// Emite "launch-step" para o frontend (ex.: patching_client, patching_server, starting). Default: no-op.
    fn emit_launch_step(&self, _step: &str) {}
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

    fn emit_launch_step(&self, step: &str) {
        let _ = self.window.emit("launch-step", serde_json::json!({ "step": step }));
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

        super::launch::validate_installation(&client_dir).await?;
        super::launch::check_ram(&settings)?;

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

        let java_exec = host.ensure_java().await?;

        #[cfg(target_os = "windows")]
        let executable = client_dir.join("HytaleClient.exe");
        #[cfg(not(target_os = "windows"))]
        let executable = client_dir.join("HytaleClient");

        if settings.online_mode {
            Self::handle_online_patches(host, &settings, &executable, &client_dir, &java_exec).await?;
        }

        #[cfg(not(target_os = "windows"))]
        {
            log::info!("Ensuring executable permissions after potential patching...");
            super::launch::ensure_permissions(&executable).await?;
        }

        let player_name = crate::player::get_player_name(pool);
        let offline_uuid = settings.player_id.clone();

        let (auth_mode, identity_token, session_token, final_uuid, final_name) =
            if settings.online_mode {
                let res = Self::authenticate(
                    &offline_uuid,
                    &player_name,
                    &settings.auth_domain,
                    &client_dir,
                )
                .await;

                if res.4 != settings.player_name || res.3 != settings.player_id {
                    log::info!(
                        "Identity synchronization: {} -> {}, id: {} -> {}",
                        settings.player_name,
                        res.4,
                        settings.player_id,
                        res.3
                    );

                    let mut new_settings = settings.clone();
                    new_settings.player_name = res.4.clone();
                    new_settings.player_id = res.3.clone();

                    if let Err(e) = crate::settings::save_settings(pool, &new_settings) {
                        log::error!("Failed to save synchronized identity: {}", e);
                    }
                }
                res
            } else {
                (
                    "offline".to_string(),
                    None,
                    None,
                    offline_uuid.clone(),
                    player_name,
                )
            };

        let mut args = super::launch::LaunchArgs {
            app_dir: app_dir_str,
            user_dir: user_dir_str,
            uuid: final_uuid,
            name: final_name,
            auth_mode,
            identity_token: None,
            session_token: None,
        };

        if let (Some(ident), Some(sess)) = (identity_token, session_token) {
            args.identity_token = Some(ident);
            args.session_token = Some(sess);
        }

        let jvm_args = super::launch::construct_jvm_args(&settings);

        host.emit_launch_step("starting");
        super::launch::spawn_game_process(&executable, &java_exec, args, jvm_args, &client_dir)
            .await?;

        log::info!("Game launched successfully");

        if settings.close_on_launch {
            log::info!("Closing launcher as requested by settings");
            host.exit(0);
        }

        Ok(())
    }

    async fn handle_online_patches<H: GameHost>(
        host: &H,
        settings: &crate::settings::GameSettings,
        executable: &PathBuf,
        client_dir: &PathBuf,
        java_exec: &PathBuf,
    ) -> Result<(), AppError> {
        log::info!("Online mode enabled, ensuring game is patched...");
        let patcher = super::patcher::ClientPatcher::new(Some(settings.auth_domain.clone()));

        host.emit_launch_step("patching_client");
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
            host.emit_launch_step("patching_server");
            log::info!("Applying advanced bytecode patching to server...");
            patcher
                .run_dual_auth_patcher(java_exec, &server_jar, &server_jar)
                .await
                .map_err(|e| AppError::Unknown(format!("Server patching failed: {}", e)))?;
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
}

#[derive(serde::Serialize)]
struct TokenConfig {
    #[serde(rename = "sessionToken")]
    session_token: String,
    #[serde(rename = "authServerUrl")]
    auth_server_url: String,
    issuer: String,
    #[serde(rename = "userId")]
    user_id: String,
}

impl GameService {
    fn inject_auth_token(
        client_dir: &std::path::Path,
        tokens: &crate::player::auth::AuthTokens,
        domain: &str,
    ) -> anyhow::Result<()> {
        let user_id = tokens
            .user_id
            .clone()
            .unwrap_or_else(|| "00000000-0000-0000-0000-000000000000".to_string());

        let config = TokenConfig {
            session_token: tokens.session_token.clone(),
            auth_server_url: format!("https://{}", domain),
            issuer: format!("https://{}", domain),
            user_id,
        };

        let config_path = client_dir.join("auth_token.json");
        let json = serde_json::to_string_pretty(&config)?;
        std::fs::write(config_path, json)?;

        log::info!("Auth token injected successfully into auth_token.json");
        Ok(())
    }

    async fn authenticate(
        offline_uuid: &str,
        player_name: &str,
        auth_domain: &str,
        client_dir: &std::path::Path,
    ) -> (String, Option<String>, Option<String>, String, String) {
        match crate::player::auth::fetch_custom_auth_tokens(player_name, auth_domain).await {
            Ok(tokens) => {
                let uuid = tokens
                    .user_id
                    .clone()
                    .unwrap_or_else(|| offline_uuid.to_string());
                let name = tokens.name.clone();

                if let Err(e) = Self::inject_auth_token(client_dir, &tokens, auth_domain) {
                    log::error!("Failed to inject auth token: {}", e);
                }

                return (
                    "authenticated".to_string(),
                    Some(tokens.identity_token),
                    Some(tokens.session_token),
                    uuid,
                    name,
                );
            }
            Err(e) => {
                log::info!("Custom auth failed ({}), trying legacy Hytale auth...", e);
            }
        }

        match crate::player::auth::fetch_auth_tokens(offline_uuid, player_name, auth_domain).await {
            Ok(tokens) => {
                if let Err(e) = Self::inject_auth_token(client_dir, &tokens, auth_domain) {
                    log::error!("Failed to inject auth token (fallback): {}", e);
                }

                (
                    "authenticated".to_string(),
                    Some(tokens.identity_token),
                    Some(tokens.session_token),
                    offline_uuid.to_string(),
                    tokens.name,
                )
            }
            Err(e) => {
                log::warn!(
                    "Failed to fetch online tokens, falling back to offline: {}",
                    e
                );
                (
                    "offline".to_string(),
                    None,
                    None,
                    offline_uuid.to_string(),
                    player_name.to_string(),
                )
            }
        }
    }
}
