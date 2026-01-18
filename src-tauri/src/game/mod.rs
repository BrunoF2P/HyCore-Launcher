use std::fs;
use std::process::Command;

#[tauri::command]
pub async fn launch_game(window: tauri::Window) -> Result<(), String> {
    let game_dir = crate::updater::env::get_game_dir();
    let client_dir = crate::updater::env::get_client_dir();
    let user_dir = crate::updater::env::get_user_data_dir();

    if !client_dir.exists() {
        return Err("Game not installed. Please update first.".to_string());
    }

    let app_dir_str = game_dir.to_str().ok_or("Invalid game directory path")?;
    let user_dir_str = user_dir
        .to_str()
        .ok_or("Invalid UserData path")?
        .to_string();

    let _ = fs::create_dir_all(&user_dir);

    let java_exec = crate::updater::java::ensure_java(&window).await?;

    let player_name = crate::player::get_player_name();

    #[cfg(target_os = "windows")]
    let executable = client_dir.join("HytaleClient.exe");

    #[cfg(not(target_os = "windows"))]
    let executable = client_dir.join("HytaleClient");

    if !executable.exists() {
        return Err("Game executable not found. Please update the game.".to_string());
    }

    crate::updater::system::log_error(&format!("Launching game: {:?}", executable));
    crate::updater::system::log_error(&format!("Player: {}", player_name));

    let mut cmd = Command::new(&executable);
    cmd.arg("--app-dir")
        .arg(app_dir_str)
        .arg("--user-dir")
        .arg(&user_dir_str)
        .arg("--uuid")
        .arg("13371337-1337-1337-1337-133713371337")
        .arg("--name")
        .arg(&player_name)
        .arg("--auth-mode")
        .arg("Offline");

    #[cfg(target_os = "linux")]
    {
        // Filter out AppImage mount paths from LD_LIBRARY_PATH to prevent library conflicts
        if let Some(path) = std::env::var_os("LD_LIBRARY_PATH") {
            let path_str = path.to_string_lossy();
            let filtered_paths: Vec<&str> = path_str
                .split(':')
                .filter(|p| !p.contains("/.mount_") && !p.contains("/tmp/.mount"))
                .collect();

            let mut new_path = std::ffi::OsString::from(&client_dir);
            if !filtered_paths.is_empty() {
                new_path.push(":");
                new_path.push(filtered_paths.join(":"));
            }
            cmd.env("LD_LIBRARY_PATH", new_path);
        } else {
            cmd.env("LD_LIBRARY_PATH", &client_dir);
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

    crate::updater::system::log_error(&format!("Executing command: {:?}", cmd));
    eprintln!("DEBUG LAUNCH ARGS: {:?}", cmd);

    cmd.arg("--java-exec").arg(java_exec);

    crate::updater::system::log_error(&format!("Launching with Java: {:?}", cmd));

    cmd.spawn()
        .map_err(|e| format!("Failed to launch game: {}", e))?;

    crate::updater::system::log_error("Game launched successfully");
    Ok(())
}
