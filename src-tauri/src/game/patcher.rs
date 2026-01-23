use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

pub const ORIGINAL_DOMAIN: &str = "hytale.com";
pub const DEFAULT_AUTH_DOMAIN: &str = "sanasol.ws";

#[derive(Serialize, Deserialize, Debug)]
pub struct PatchFlag {
    pub patched_at: String,
    pub original_domain: String,
    pub target_domain: String,
    pub patcher_version: String,
}

pub struct PatchResult {
    pub success: bool,
    pub error: Option<String>,
}

pub struct ClientPatcher {
    target_domain: String,
}

impl ClientPatcher {
    pub fn new(target_domain: Option<String>) -> Self {
        let domain = target_domain.unwrap_or_else(|| DEFAULT_AUTH_DOMAIN.to_string());

        // Domain length must match original for binary patching to work
        if domain.len() != ORIGINAL_DOMAIN.len() {
            log::warn!(
                "Domain {} length ({}) doesn't match original {} ({}), using default",
                domain,
                domain.len(),
                ORIGINAL_DOMAIN,
                ORIGINAL_DOMAIN.len()
            );
            return Self {
                target_domain: DEFAULT_AUTH_DOMAIN.to_string(),
            };
        }

        Self {
            target_domain: domain,
        }
    }

    fn to_utf16le(s: &str) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(s.len() * 2);
        for c in s.chars() {
            let u = c as u16;
            bytes.extend_from_slice(&u.to_le_bytes());
        }
        bytes
    }

    pub fn find_and_replace_domain_smart(&self, data: &mut [u8]) -> usize {
        let mut count = 0;

        // Get UTF-16LE patterns for old and new domains (without last char)
        let old_no_last = Self::to_utf16le(&ORIGINAL_DOMAIN[..ORIGINAL_DOMAIN.len() - 1]);
        let new_no_last = Self::to_utf16le(&self.target_domain[..self.target_domain.len() - 1]);

        let old_last_char_byte = ORIGINAL_DOMAIN.as_bytes()[ORIGINAL_DOMAIN.len() - 1];
        let new_last_char_byte = self.target_domain.as_bytes()[self.target_domain.len() - 1];

        let mut pos = 0;
        while pos < data.len() {
            let remaining = &data[pos..];
            if let Some(idx_in_remaining) = remaining
                .windows(old_no_last.len())
                .position(|window| window == old_no_last.as_slice())
            {
                let idx = pos + idx_in_remaining;
                let last_char_pos = idx + old_no_last.len();

                if last_char_pos < data.len() {
                    let last_char_first_byte = data[last_char_pos];

                    // Check if this looks like a valid domain occurrence
                    if last_char_first_byte == old_last_char_byte {
                        // Copy new domain (without last char) in-place
                        data[idx..idx + new_no_last.len()].copy_from_slice(&new_no_last);
                        // Update last char
                        data[last_char_pos] = new_last_char_byte;
                        count += 1;
                    }
                }
                pos = idx + 1;
            } else {
                break;
            }
        }
        count
    }

    pub fn find_and_replace_utf8(&self, data: &mut [u8]) -> usize {
        let old_bytes = ORIGINAL_DOMAIN.as_bytes();
        let new_bytes = self.target_domain.as_bytes();
        let mut count = 0;

        let mut pos = 0;
        while pos <= data.len().saturating_sub(old_bytes.len()) {
            if &data[pos..pos + old_bytes.len()] == old_bytes {
                data[pos..pos + new_bytes.len()].copy_from_slice(new_bytes);
                count += 1;
                pos += old_bytes.len();
            } else {
                pos += 1;
            }
        }
        count
    }

    fn get_flag_path(&self, binary_path: &Path) -> PathBuf {
        let mut path = binary_path.to_path_buf();
        let mut name = path.file_name().unwrap_or_default().to_os_string();
        name.push(".patched_custom");
        path.set_file_name(name);
        path
    }

    fn is_already_patched(&self, binary_path: &Path) -> bool {
        let flag_path = self.get_flag_path(binary_path);
        if let Ok(content) = fs::read_to_string(flag_path) {
            if let Ok(flag) = serde_json::from_str::<PatchFlag>(&content) {
                return flag.target_domain == self.target_domain;
            }
        }
        false
    }

    fn backup_binary(&self, binary_path: &Path) -> anyhow::Result<PathBuf> {
        let mut backup_path = binary_path.to_path_buf();
        let mut name = backup_path.file_name().unwrap_or_default().to_os_string();
        name.push(".original");
        backup_path.set_file_name(name);

        if !backup_path.exists() {
            fs::copy(binary_path, &backup_path)?;
        }
        Ok(backup_path)
    }

    pub fn patch_client(&self, client_path: &Path) -> PatchResult {
        if !client_path.exists() {
            return PatchResult {
                success: false,
                error: Some(format!("Client binary not found: {:?}", client_path)),
            };
        }

        if self.is_already_patched(client_path) {
            return PatchResult {
                success: true,
                error: None,
            };
        }

        let backup_path = match self.backup_binary(client_path) {
            Ok(p) => p,
            Err(e) => {
                return PatchResult {
                    success: false,
                    error: Some(format!("Failed to create backup: {}", e)),
                };
            }
        };

        let mut data = match fs::read(&backup_path) {
            Ok(d) => d,
            Err(e) => {
                return PatchResult {
                    success: false,
                    error: Some(format!("Failed to read original binary: {}", e)),
                }
            }
        };

        let mut count = self.find_and_replace_domain_smart(&mut data);
        count += self.find_and_replace_utf8(&mut data);

        log::info!(
            "Client patching found {} occurrences of {}",
            count,
            self.target_domain
        );

        if count > 0 {
            if let Err(e) = fs::write(client_path, data) {
                return PatchResult {
                    success: false,
                    error: Some(format!("Failed to write patched client: {}", e)),
                };
            }

            let flag = PatchFlag {
                patched_at: time::OffsetDateTime::now_utc().to_string(),
                original_domain: ORIGINAL_DOMAIN.to_string(),
                target_domain: self.target_domain.clone(),
                patcher_version: env!("CARGO_PKG_VERSION").to_string(),
            };

            if let Ok(json) = serde_json::to_string_pretty(&flag) {
                let _ = fs::write(self.get_flag_path(client_path), json);
            }
        }

        PatchResult {
            success: true,
            error: None,
        }
    }

    pub fn patch_server(&self, server_path: &Path) -> PatchResult {
        if !server_path.exists() {
            return PatchResult {
                success: false,
                error: Some(format!("Server JAR not found: {:?}", server_path)),
            };
        }

        if self.is_already_patched(server_path) {
            return PatchResult {
                success: true,
                error: None,
            };
        }

        let backup_path = match self.backup_binary(server_path) {
            Ok(p) => p,
            Err(e) => {
                return PatchResult {
                    success: false,
                    error: Some(format!("Failed to create backup: {}", e)),
                };
            }
        };

        let file = match fs::File::open(&backup_path) {
            Ok(f) => f,
            Err(e) => {
                return PatchResult {
                    success: false,
                    error: Some(format!("Failed to open original JAR: {}", e)),
                }
            }
        };

        let mut archive = match ZipArchive::new(file) {
            Ok(a) => a,
            Err(e) => {
                return PatchResult {
                    success: false,
                    error: Some(format!("Failed to parse JAR: {}", e)),
                }
            }
        };

        let mut new_jar_data = Vec::new();
        let mut writer = ZipWriter::new(std::io::Cursor::new(&mut new_jar_data));
        let mut total_count = 0;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i).unwrap();
            let name = file.name().to_string();
            let mut content = Vec::new();
            file.read_to_end(&mut content).unwrap();

            let should_patch = name.ends_with(".class")
                || name.ends_with(".properties")
                || name.ends_with(".json")
                || name.ends_with(".xml")
                || name.ends_with(".yml");

            if should_patch {
                let count = self.find_and_replace_utf8(&mut content);
                if count > 0 {
                    log::debug!("Patched {} in JAR entry {}", self.target_domain, name);
                    total_count += count;
                }
            }

            let options = SimpleFileOptions::default()
                .compression_method(file.compression())
                .unix_permissions(file.unix_mode().unwrap_or(0o644));

            writer.start_file(name, options).unwrap();
            writer.write_all(&content).unwrap();
        }

        writer.finish().unwrap();

        if total_count > 0 {
            if let Err(e) = fs::write(server_path, new_jar_data) {
                return PatchResult {
                    success: false,
                    error: Some(format!("Failed to write patched server: {}", e)),
                };
            }

            let flag = PatchFlag {
                patched_at: time::OffsetDateTime::now_utc().to_string(),
                original_domain: ORIGINAL_DOMAIN.to_string(),
                target_domain: self.target_domain.clone(),
                patcher_version: env!("CARGO_PKG_VERSION").to_string(),
            };

            if let Ok(json) = serde_json::to_string_pretty(&flag) {
                let _ = fs::write(self.get_flag_path(server_path), json);
            }
        }

        PatchResult {
            success: true,
            error: None,
        }
    }

    #[cfg(target_os = "macos")]
    pub async fn sign_macos_app(&self, app_path: &Path) -> anyhow::Result<()> {
        log::info!("Signing macOS app bundle: {:?}", app_path);

        // Remove quarantine
        let _ = tokio::process::Command::new("xattr")
            .arg("-cr")
            .arg(app_path)
            .status()
            .await;

        // Sign ad-hoc
        let status = tokio::process::Command::new("codesign")
            .arg("--force")
            .arg("--deep")
            .arg("--sign")
            .arg("-")
            .arg(app_path)
            .status()
            .await?;

        if !status.success() {
            anyhow::bail!("Failed to sign macOS app bundle");
        }

        Ok(())
    }
}
