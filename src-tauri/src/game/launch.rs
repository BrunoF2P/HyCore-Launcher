use std::path::Path;
use tokio::process::Command;

use crate::error::AppError;
use crate::settings::GameSettings;

pub async fn validate_installation(client_dir: &Path) -> Result<(), AppError> {
    if !client_dir.exists() {
        return Err(AppError::GameNotInstalled);
    }

    #[cfg(target_os = "windows")]
    let executable = client_dir.join("HytaleClient.exe");
    #[cfg(not(target_os = "windows"))]
    let executable = client_dir.join("HytaleClient");

    if !executable.exists() {
        return Err(AppError::GameNotInstalled);
    }

    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub async fn ensure_permissions(executable: &Path) -> Result<(), AppError> {
    use std::os::unix::fs::PermissionsExt;

    // We use tokio::fs for async metadata check to avoid blocking
    if let Ok(metadata) = tokio::fs::metadata(executable).await {
        let mut perms = metadata.permissions();
        if perms.mode() & 0o111 == 0 {
            log::info!("Setting execution permissions for {:?}", executable);
            perms.set_mode(0o755);
            if let Err(e) = tokio::fs::set_permissions(executable, perms).await {
                log::warn!("Failed to set permissions (non-fatal): {}", e);
            }
        }
    }
    Ok(())
}

pub fn check_ram(settings: &GameSettings) -> Result<(), AppError> {
    let free_gb = crate::system::info::get_total_ram_gb_internal();

    if free_gb < settings.ram_gb {
        log::warn!(
            "System only has {}GB RAM, but {}GB was requested",
            free_gb,
            settings.ram_gb
        );
        return Err(AppError::InsufficientRam {
            requested: settings.ram_gb,
            available: free_gb,
        });
    }
    Ok(())
}

pub struct LaunchArgs {
    pub app_dir: String,
    pub user_dir: String,
    pub uuid: String,
    pub name: String,
    pub auth_mode: String,
    pub identity_token: Option<String>,
    pub session_token: Option<String>,
}

pub fn construct_jvm_args(settings: &GameSettings) -> String {
    let mut jvm_options = format!("-Xms{}G -Xmx{}G", settings.ram_gb, settings.ram_gb);

    if !settings.custom_java_args.is_empty() {
        jvm_options.push(' ');
        jvm_options.push_str(&settings.custom_java_args);
    }
    jvm_options
}

pub async fn spawn_game_process(
    executable: &Path,
    java_exec: &Path,
    args: LaunchArgs,
    jvm_args: String,
    client_dir: &Path,
) -> Result<(), AppError> {
    log::info!("Launching game: {:?}", executable);
    log::info!("Player: {} (UUID: {})", args.name, args.uuid);

    let mut cmd = Command::new(executable);

    cmd.arg("--app-dir")
        .arg(&args.app_dir)
        .arg("--user-dir")
        .arg(&args.user_dir)
        .arg("--uuid")
        .arg(&args.uuid)
        .arg("--name")
        .arg(&args.name)
        .arg("--auth-mode")
        .arg(&args.auth_mode);

    if let Some(token) = &args.identity_token {
        cmd.arg("--identity-token").arg(token);
    }

    if let Some(token) = &args.session_token {
        cmd.arg("--session-token").arg(token);
    }

    cmd.env("_JAVA_OPTIONS", jvm_args);
    cmd.arg("--java-exec").arg(java_exec);

    // Platform specific env vars
    #[cfg(target_os = "linux")]
    {
        if let Some(path) = std::env::var_os("LD_LIBRARY_PATH") {
            let path_str = path.to_string_lossy();
            let filtered_paths: Vec<&str> = path_str
                .split(':')
                .filter(|p| !p.contains("/.mount_") && !p.contains("/tmp/.mount"))
                .collect();

            let mut new_path = std::ffi::OsString::from(client_dir);
            if !filtered_paths.is_empty() {
                new_path.push(":");
                new_path.push(filtered_paths.join(":"));
            }
            cmd.env("LD_LIBRARY_PATH", new_path);
        } else {
            cmd.env("LD_LIBRARY_PATH", client_dir);
        }

        let wayland_display = std::env::var("WAYLAND_DISPLAY").unwrap_or_default();
        let xdg_session = std::env::var("XDG_SESSION_TYPE").unwrap_or_default();

        if !wayland_display.is_empty() || xdg_session.to_lowercase() == "wayland" {
            cmd.env("SDL_VIDEODRIVER", "wayland,x11");
        }
    }

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    // Spawn async
    cmd.spawn()
        .map_err(|e| AppError::Unknown(format!("Failed to launch game: {}", e)))?;

    Ok(())
}
