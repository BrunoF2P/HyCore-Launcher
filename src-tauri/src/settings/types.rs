use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GameSettings {
    pub ram_min_gb: u32,
    pub ram_max_gb: u32,
    pub custom_java_args: String,
    pub close_on_launch: bool,
    pub minimize_to_tray: bool,
    pub discord_rpc_enabled: bool,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            ram_min_gb: 2,
            ram_max_gb: 4,
            custom_java_args: "".to_string(),
            close_on_launch: false,
            minimize_to_tray: true,
            discord_rpc_enabled: true,
        }
    }
}
