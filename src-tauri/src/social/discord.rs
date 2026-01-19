use crate::settings::load_settings;
use discord_rich_presence::{activity, DiscordIpc, DiscordIpcClient};
use once_cell::sync::Lazy;
use std::sync::Mutex;

static DISCORD_CLIENT: Lazy<Mutex<Option<DiscordIpcClient>>> = Lazy::new(|| Mutex::new(None));

pub fn init_discord() {
    let settings = load_settings();
    if !settings.discord_rpc_enabled {
        log::info!("Discord RPC is disabled in settings");
        return;
    }

    log::info!("Initializing Discord RPC...");
    // Using a placeholder Client ID.
    // If it's a Result, we unwrap. If it's not, we just assign.
    // Based on previous error, it seems it's NOT a result.
    let mut client = DiscordIpcClient::new("1200000000000000000");

    if let Err(e) = client.connect() {
        log::warn!(
            "Could not connect to Discord RPC: {}. Discord might not be running.",
            e
        );
        return;
    }

    let _ = client.set_activity(
        activity::Activity::new()
            .state("No Launcher")
            .details("Navegando"),
    );

    *DISCORD_CLIENT.lock().unwrap() = Some(client);
    log::info!("Discord RPC initialized successfully");
}

pub fn update_discord_status(state: &str, details: &str) {
    let mut guard = DISCORD_CLIENT.lock().unwrap();
    if let Some(client) = guard.as_mut() {
        if let Err(e) = client.set_activity(activity::Activity::new().state(state).details(details))
        {
            log::error!("Failed to update Discord RPC status: {}", e);
        }
    }
}

pub fn clear_discord() {
    let mut guard = DISCORD_CLIENT.lock().unwrap();
    if let Some(mut client) = guard.take() {
        let _ = client.close();
    }
}
