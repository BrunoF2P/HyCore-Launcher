pub fn get_native_hytale_os() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "darwin"
    } else {
        "linux"
    }
}

pub fn get_native_hytale_arch() -> &'static str {
    if cfg!(target_arch = "x86_64") {
        "amd64"
    } else {
        "arm64"
    }
}

pub fn get_hytale_os() -> String {
    let settings = crate::settings::load_settings();
    settings
        .override_os
        .unwrap_or_else(|| get_native_hytale_os().to_string())
}

pub fn get_hytale_arch() -> String {
    let settings = crate::settings::load_settings();
    settings
        .override_arch
        .unwrap_or_else(|| get_native_hytale_arch().to_string())
}

pub fn get_java_os() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "mac"
    } else {
        "linux"
    }
}

pub fn get_java_arch() -> &'static str {
    if cfg!(target_arch = "x86_64") {
        "x64"
    } else {
        "aarch64"
    }
}

pub fn get_butler_os() -> &'static str {
    // Butler follows same convention as Hytale patches mostly
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "darwin"
    } else {
        "linux"
    }
}

pub fn is_game_running() -> bool {
    use sysinfo::System;
    let s = System::new_all();
    for process in s.processes().values() {
        let name = process.name().to_string_lossy().to_lowercase();
        if name.contains("hytaleclient") {
            return true;
        }
    }
    false
}

pub fn verify_file_checksum(path: &std::path::Path, expected_sha256: &str) -> anyhow::Result<bool> {
    use sha2::{Digest, Sha256};
    use std::io::Read;

    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];

    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }

    let hash = format!("{:x}", hasher.finalize());
    log::info!("File checksum for {:?}: {}", path, hash);
    log::info!("Expected checksum: {}", expected_sha256);

    Ok(hash == expected_sha256.to_lowercase())
}
