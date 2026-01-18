use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UpdateStatus {
    pub stage: String,
    pub progress: f64,
    pub message: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SystemRequirements {
    pub has_internet: bool,
    pub free_space_gb: u64,
    pub meets_requirements: bool,
}
