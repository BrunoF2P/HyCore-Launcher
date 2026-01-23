use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Mod not found: {0}")]
    ModNotFound(String),

    #[error("Profile not found: {0}")]
    ProfileNotFound(String),

    #[error("Game not installed")]
    GameNotInstalled,

    #[error("O jogo está aberto. Feche o jogo antes de realizar esta operação.")]
    GameRunning,

    #[error("Memória RAM insuficiente. Você reservou {requested}GB, mas o sistema tem apenas {available}GB livres.")]
    InsufficientRam { requested: u32, available: u32 },

    #[error("Failed to create directory: {0}")]
    DirCreation(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

// Convert string errors for legacy support during refactor
impl From<String> for AppError {
    fn from(s: String) -> Self {
        AppError::Unknown(s)
    }
}

impl From<anyhow::Error> for AppError {
    fn from(e: anyhow::Error) -> Self {
        AppError::Unknown(e.to_string())
    }
}

impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
