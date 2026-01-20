pub fn get_hytale_os() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "darwin"
    } else {
        "linux"
    }
}

pub fn get_hytale_arch() -> &'static str {
    if cfg!(target_arch = "x86_64") {
        "amd64"
    } else {
        "arm64"
    }
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
