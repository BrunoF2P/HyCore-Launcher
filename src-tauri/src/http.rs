use once_cell::sync::Lazy;
use reqwest::Client;
use std::time::Duration;

pub static HTTP_CLIENT: Lazy<Client> = Lazy::new(|| {
    Client::builder()
        .timeout(Duration::from_secs(30))
        .pool_idle_timeout(Duration::from_secs(90))
        .pool_max_idle_per_host(10)
        .user_agent("HyCore-Launcher/1.0")
        .build()
        .expect("Failed to build global HTTP client")
});
