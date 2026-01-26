use discord_rich_presence::{activity, DiscordIpc, DiscordIpcClient};
use once_cell::sync::Lazy;
use std::sync::Mutex;

static DISCORD_CLIENT: Lazy<Mutex<Option<DiscordIpcClient>>> = Lazy::new(|| Mutex::new(None));

pub fn init_discord() {
    // Discord RPC will be enabled/disabled via settings commands which have pool access
    // This just initializes the static client holder
    log::info!("Discord RPC client holder initialized (will be configured via settings)");
}

pub fn set_rpc_enabled(enabled: bool) {
    if enabled {
        let mut guard = DISCORD_CLIENT.lock().unwrap();
        if guard.is_some() {
            log::info!("Discord RPC already initialized");
            return;
        }

        log::info!("Initializing Discord RPC...");
        let mut client = DiscordIpcClient::new("1461306150497550376");

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

        *guard = Some(client);
        log::info!("Discord RPC initialized successfully");
    } else {
        clear_discord();
    }
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
