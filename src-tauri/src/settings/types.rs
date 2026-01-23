use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GameSettings {
    pub ram_gb: u32,
    pub custom_java_args: String,
    pub close_on_launch: bool,
    pub minimize_to_tray: bool,
    pub discord_rpc_enabled: bool,
    pub channel: String,
    pub language: String,
    pub active_version: u32,
    pub player_name: String,
    pub override_os: Option<String>,
    pub override_arch: Option<String>,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            ram_gb: 4,
            custom_java_args: "".to_string(),
            close_on_launch: false,
            minimize_to_tray: true,
            discord_rpc_enabled: true,
            channel: "release".to_string(),
            language: "auto".to_string(),
            active_version: 0,
            player_name: std::env::var("USER")
                .or_else(|_| std::env::var("USERNAME"))
                .unwrap_or_else(|_| "Player".to_string()),
            override_os: None,
            override_arch: None,
        }
    }
}
