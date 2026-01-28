use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tauri::Manager;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

async fn ensure_asm_libs() -> anyhow::Result<PathBuf> {
    const ASM_9_7: &[(&str, &str)] = &[
        ("asm-9.7.jar", "https://repo1.maven.org/maven2/org/ow2/asm/asm/9.7/asm-9.7.jar"),
        ("asm-tree-9.7.jar", "https://repo1.maven.org/maven2/org/ow2/asm/asm-tree/9.7/asm-tree-9.7.jar"),
        ("asm-util-9.7.jar", "https://repo1.maven.org/maven2/org/ow2/asm/asm-util/9.7/asm-util-9.7.jar"),
        ("asm-commons-9.7.jar", "https://repo1.maven.org/maven2/org/ow2/asm/asm-commons/9.7/asm-commons-9.7.jar"),
    ];
    let lib = crate::updater::env::get_hycore_data_dir().join("patcher_lib");
    let _ = fs::create_dir_all(&lib);
    for (name, url) in ASM_9_7 {
        let p = lib.join(name);
        if !p.exists() {
            log::info!("Downloading {} for DualAuthPatcher...", name);
            let bytes = crate::http::HTTP_CLIENT
                .get(*url)
                .send()
                .await?
                .error_for_status()?
                .bytes()
                .await?;
            tokio::fs::write(&p, &bytes).await?;
        }
    }
    Ok(lib)
}

/// Redacts output that may contain a JWT (tokens start with "eyJ" in base64). Never log tokens.
fn redact_jwt_if_present(s: &str) -> Cow<'_, str> {
    if s.contains("eyJ") {
        Cow::Borrowed("[REDACTED: output may have contained token]")
    } else {
        Cow::Borrowed(s)
    }
}

pub const ORIGINAL_DOMAIN: &str = "hytale.com";
pub const DEFAULT_AUTH_DOMAIN: &str = "auth.sanasol.ws";
pub const MIN_DOMAIN_LENGTH: usize = 4;
pub const MAX_DOMAIN_LENGTH: usize = 16;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PatchMode {
    Direct,
    Split,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DomainStrategy {
    pub mode: PatchMode,
    pub main_domain: String,
    pub subdomain_prefix: String,
    pub description: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PatchFlag {
    pub patched_at: String,
    pub original_domain: String,
    pub target_domain: String,
    pub patch_mode: PatchMode,
    pub main_domain: String,
    pub subdomain_prefix: String,
    pub patcher_version: String,
    pub verified: String,
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
        let mut domain = target_domain.unwrap_or_else(|| DEFAULT_AUTH_DOMAIN.to_string());

        if domain.len() < MIN_DOMAIN_LENGTH || domain.len() > MAX_DOMAIN_LENGTH {
            log::warn!(
                "Domain \"{}\" length ({}) is invalid (min {}, max {}), using default",
                domain,
                domain.len(),
                MIN_DOMAIN_LENGTH,
                MAX_DOMAIN_LENGTH
            );
            domain = DEFAULT_AUTH_DOMAIN.to_string();
        }

        Self {
            target_domain: domain,
        }
    }

    #[cfg(test)]
    /// Constrói um patcher com o domínio dado, sem validação. Apenas para testes.
    pub fn new_for_test(domain: &str) -> Self {
        Self {
            target_domain: domain.to_string(),
        }
    }

    /// Domain strategy: main_domain must fit in "hytale.com" (9 chars), and
    /// "https://" + subdomain_prefix must fit in "https://tools." (15 chars) → prefix ≤ 7.
    ///
    /// | len  | prefix | main  | supports |
    /// |------|--------|-------|----------|
    /// | 4–9  | 0      | all   | Direct   |
    /// | 10   | 1      | 9     | Split    |
    /// | 11–15| 6      | 5–9   | Split    |
    /// | 16   | 7      | 9     | Split    |
    pub fn get_domain_strategy(&self) -> DomainStrategy {
        let target = &self.target_domain;
        let len = target.len();

        if len <= 9 {
            DomainStrategy {
                mode: PatchMode::Direct,
                main_domain: target.clone(),
                subdomain_prefix: String::new(),
                description: format!("Direct: \"{}\" ({} chars)", target, len),
            }
        } else {
            let prefix_len = match len {
                10 => 1,
                16 => 7,
                11..=15 => 6,
                _ => 6,
            };
            let subdomain_prefix = target[..prefix_len].to_string();
            let main_domain = target[prefix_len..].to_string();

            DomainStrategy {
                mode: PatchMode::Split,
                main_domain: main_domain.clone(),
                subdomain_prefix: subdomain_prefix.clone(),
                description: format!(
                    "Split: prefix=\"{}\" ({}), main=\"{}\" ({})",
                    subdomain_prefix,
                    prefix_len,
                    main_domain,
                    main_domain.len()
                ),
            }
        }
    }

    /// Convert a string to the length-prefixed byte format used by the client
    /// Format: [length:u8] [00 00 00 padding] [char1] [00] [char2] [00] ... [lastChar]
    /// Note: No null byte after the last character
    fn to_length_prefixed(s: &str) -> Vec<u8> {
        let length = s.len();
        let mut result = Vec::with_capacity(4 + length + length.saturating_sub(1));

        result.push(length as u8);
        result.extend_from_slice(&[0x00, 0x00, 0x00]);

        let bytes = s.as_bytes();
        for i in 0..length {
            result.push(bytes[i]);
            if i < length - 1 {
                result.push(0x00);
            }
        }

        result
    }

    fn replace_bytes(data: &mut Vec<u8>, old_bytes: &[u8], new_bytes: &[u8]) -> usize {
        let mut count = 0;
        let mut i = 0;
        let old_len = old_bytes.len();
        let new_len = new_bytes.len();

        if new_len > old_len {
            log::error!(
                "Cannot replace bytes: new length {} > old length {}",
                new_len,
                old_len
            );
            return 0;
        }

        while i <= data.len().saturating_sub(old_len) {
            if &data[i..i + old_len] == old_bytes {
                data[i..i + new_len].copy_from_slice(new_bytes);
                count += 1;
                i += old_len;
            } else {
                match data[i + 1..].windows(old_len).position(|w| w == old_bytes) {
                    Some(pos) => i += 1 + pos,
                    None => break,
                }
            }
        }
        count
    }

    pub fn apply_domain_patches(&self, data: &mut Vec<u8>) -> usize {
        let strategy = self.get_domain_strategy();
        let mut total_count = 0;

        log::info!("Patching strategy: {}", strategy.description);

        let old_sentry = "https://ca900df42fcf57d4dd8401a86ddd7da2@sentry.hytale.com/2";
        let new_sentry = format!("https://t@{}/2", self.target_domain);

        total_count += Self::replace_bytes(
            data,
            &Self::to_length_prefixed(old_sentry),
            &Self::to_length_prefixed(&new_sentry),
        );

        let replacement_prefix = format!("https://{}", strategy.subdomain_prefix);
        let prefixes = [
            "https://tools.",
            "https://sessions.",
            "https://account-data.",
            "https://telemetry.",
        ];

        for prefix in prefixes {
            total_count += Self::replace_bytes(
                data,
                &Self::to_length_prefixed(prefix),
                &Self::to_length_prefixed(&replacement_prefix),
            );

            let old_utf16 = Self::to_utf16le(prefix);
            let new_utf16 = Self::to_utf16le(&replacement_prefix);
            total_count += Self::replace_bytes(data, &old_utf16, &new_utf16);
        }

        total_count += Self::replace_bytes(
            data,
            &Self::to_length_prefixed(ORIGINAL_DOMAIN),
            &Self::to_length_prefixed(&strategy.main_domain),
        );

        let domain_utf16_old = Self::to_utf16le(ORIGINAL_DOMAIN);
        let domain_utf16_new = Self::to_utf16le(&strategy.main_domain);
        total_count += Self::replace_bytes(data, &domain_utf16_old, &domain_utf16_new);

        let old_sessions = format!("sessions.{}", ORIGINAL_DOMAIN);
        let new_sessions = self.target_domain.clone();

        total_count += Self::replace_bytes(
            data,
            &Self::to_length_prefixed(&old_sessions),
            &Self::to_length_prefixed(&new_sessions),
        );

        total_count += Self::replace_bytes(
            data,
            &Self::to_utf16le(&old_sessions),
            &Self::to_utf16le(&new_sessions),
        );

        if strategy.main_domain.len() == ORIGINAL_DOMAIN.len() {
            total_count += Self::replace_bytes(
                data,
                ORIGINAL_DOMAIN.as_bytes(),
                strategy.main_domain.as_bytes(),
            );
        }

        total_count
    }

    pub fn patch_discord_url(&self, data: &mut Vec<u8>) -> usize {
        let old_url = ".gg/hytale";
        let new_url = ".gg/MHkEjepMQ7";

        let engine_new_url = if new_url.len() > old_url.len() {
            log::warn!(
                "Discord URL too long for engine binary ({}), truncating to {}",
                new_url.len(),
                old_url.len()
            );
            &new_url[..old_url.len()]
        } else {
            new_url
        };

        let count = Self::replace_bytes(
            data,
            &Self::to_length_prefixed(old_url),
            &Self::to_length_prefixed(engine_new_url),
        );

        let utf16_new_url = if new_url.len() > old_url.len() {
            &new_url[..old_url.len()]
        } else {
            new_url
        };

        let old_utf16 = Self::to_utf16le(old_url);
        let new_utf16 = Self::to_utf16le(utf16_new_url);

        let utf16_count = Self::replace_bytes(data, &old_utf16, &new_utf16);
        count + utf16_count
    }

    fn to_utf16le(s: &str) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(s.len() * 2);
        for c in s.chars() {
            let u = c as u16;
            bytes.extend_from_slice(&u.to_le_bytes());
        }
        bytes
    }

    /// Legacy UTF-16 replace. Only safe when target_domain.len() <= 9 (ORIGINAL_DOMAIN length);
    /// otherwise would write past the slot. For 10–16 chars the length-prefixed path must be used.
    pub fn find_and_replace_domain_smart(&self, data: &mut [u8]) -> usize {
        if self.target_domain.len() > ORIGINAL_DOMAIN.len() {
            log::debug!(
                "Legacy smart replace skipped: target domain {} chars > {}",
                self.target_domain.len(),
                ORIGINAL_DOMAIN.len()
            );
            return 0;
        }

        let mut count = 0;
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

                    if last_char_first_byte == old_last_char_byte {
                        data[idx..idx + new_no_last.len()].copy_from_slice(&new_no_last);
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

    fn get_flag_path(&self, binary_path: &Path) -> PathBuf {
        let mut path = binary_path.to_path_buf();
        let mut name = path.file_name().unwrap_or_default().to_os_string();
        name.push(".patched_custom");
        path.set_file_name(name);
        path
    }

    fn is_already_patched(&self, binary_path: &Path) -> bool {
        let flag_path = self.get_flag_path(binary_path);
        log::debug!("Checking if already patched: {:?}", binary_path);

        if let Ok(content) = fs::read_to_string(&flag_path) {
            if let Ok(flag) = serde_json::from_str::<PatchFlag>(&content) {
                if flag.target_domain == self.target_domain {
                    let ext = binary_path
                        .extension()
                        .and_then(|s| s.to_str())
                        .map(|s| s.to_lowercase());

                    if ext == Some("jar".to_string()) {
                        log::info!("Verifying server JAR patch status: {:?}", binary_path);
                        if let Ok(file) = fs::File::open(binary_path) {
                            if let Ok(mut archive) = zip::ZipArchive::new(file) {
                                if archive
                                    .by_name(
                                        "com/hypixel/hytale/server/core/auth/DualJwksFetcher.class",
                                    )
                                    .is_ok()
                                {
                                    log::info!(
                                        "Server JAR verification passed for {:?}",
                                        binary_path
                                    );
                                    return true;
                                }
                            }
                        }
                        log::warn!(
                            "Server flag exists but JAR not patched, re-patching {:?}",
                            binary_path
                        );
                        return false;
                    }

                    let backup_path = binary_path.with_extension("original");
                    if backup_path.exists() {
                        if let (Ok(curr), Ok(orig)) =
                            (fs::metadata(binary_path), fs::metadata(&backup_path))
                        {
                            if curr.len() == orig.len() {
                                return true;
                            }
                        }
                    }

                    if let Ok(data) = fs::read(binary_path) {
                        let strategy = self.get_domain_strategy();
                        let pattern = Self::to_length_prefixed(&strategy.main_domain);

                        log::info!(
                            "Performing full binary verification for {:?}...",
                            binary_path
                        );
                        if data.windows(pattern.len()).any(|w| w == pattern) {
                            log::info!(
                                "Binary verification passed (full scan) for {:?}",
                                binary_path
                            );
                            return true;
                        } else {
                            log::warn!(
                                "Flag exists but binary not patched, re-patching {:?}",
                                binary_path
                            );
                            return false;
                        }
                    }
                }
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
            log::info!("Creating initial backup at {:?}", backup_path);
            fs::copy(binary_path, &backup_path)?;
            return Ok(backup_path);
        }

        let current_meta = fs::metadata(binary_path)?;
        let backup_meta = fs::metadata(&backup_path)?;

        if current_meta.len() != backup_meta.len() {
            let timestamp = time::OffsetDateTime::now_utc()
                .format(&time::format_description::parse(
                    "[year]-[month]-[day]T[hour]-[minute]-[second]",
                )?)
                .unwrap();

            let mut old_backup_path = binary_path.to_path_buf();
            let mut old_name = old_backup_path
                .file_name()
                .unwrap_or_default()
                .to_os_string();
            old_name.push(format!(".original.{}", timestamp));
            old_backup_path.set_file_name(old_name);

            log::info!(
                "File updated, archiving old backup to {:?}",
                old_backup_path
            );
            fs::rename(&backup_path, &old_backup_path)?;
            fs::copy(binary_path, &backup_path)?;
        } else {
            log::debug!("Backup already exists and is up to date");
        }

        Ok(backup_path)
    }

    /// Restore the original client binary from backup
    #[allow(dead_code)]
    pub fn restore_client(&self, binary_path: &Path) -> anyhow::Result<bool> {
        let mut backup_path = binary_path.to_path_buf();
        let mut name = backup_path.file_name().unwrap_or_default().to_os_string();
        name.push(".original");
        backup_path.set_file_name(name);

        if backup_path.exists() {
            fs::copy(&backup_path, binary_path)?;

            let flag_path = self.get_flag_path(binary_path);
            if flag_path.exists() {
                fs::remove_file(flag_path)?;
            }

            log::info!("Client restored from backup: {:?}", binary_path);
            Ok(true)
        } else {
            log::warn!("No backup found to restore for {:?}", binary_path);
            Ok(false)
        }
    }

    pub fn patch_client(&self, client_path: &Path) -> PatchResult {
        if !client_path.exists() {
            return PatchResult {
                success: false,
                error: Some(format!("Client binary not found: {:?}", client_path)),
            };
        }

        if self.is_already_patched(client_path) {
            log::info!(
                "Client already patched for {}, skipping",
                self.target_domain
            );
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

        log::info!("Patching client binary: {:?}", client_path);

        let mut total_count = self.apply_domain_patches(&mut data);
        total_count += self.patch_discord_url(&mut data);

        if total_count == 0 {
            log::info!("No occurrences found with length-prefixed format, trying legacy format...");
            let mut legacy_data = data.clone();
            let legacy_count = self.find_and_replace_domain_smart(&mut legacy_data);

            if legacy_count > 0 {
                log::info!("Found {} occurrences with legacy format", legacy_count);
                data = legacy_data;
                total_count = legacy_count;
            } else {
                log::warn!(
                    "No occurrences found - binary may already be modified or has different format"
                );
            }
        }

        if total_count > 0 {
            if let Err(e) = fs::write(client_path, data) {
                return PatchResult {
                    success: false,
                    error: Some(format!("Failed to write patched client: {}", e)),
                };
            }

            let strategy = self.get_domain_strategy();
            let flag = PatchFlag {
                patched_at: time::OffsetDateTime::now_utc().to_string(),
                original_domain: ORIGINAL_DOMAIN.to_string(),
                target_domain: self.target_domain.clone(),
                patch_mode: strategy.mode,
                main_domain: strategy.main_domain,
                subdomain_prefix: strategy.subdomain_prefix,
                patcher_version: env!("CARGO_PKG_VERSION").to_string(),
                verified: "binary_contents".to_string(),
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

    pub async fn run_dual_auth_patcher(
        &self,
        java_exec: &Path,
        server_jar: &Path,
        _output_jar: &Path,
    ) -> anyhow::Result<()> {
        log::info!("Starting server patching verification...");
        if self.is_already_patched(server_jar) {
            log::info!(
                "Server already patched for {}, skipping",
                self.target_domain
            );
            return Ok(());
        }

        let asm_path = ensure_asm_libs().await?;
        let cp_sep = if cfg!(windows) { ";" } else { ":" };
        let javac_cp = format!("{}/*", asm_path.display());
        let java_cp = format!(".{}{}/*", cp_sep, asm_path.display());

        let resource_dir: Option<PathBuf> = crate::get_app_handle()
            .path()
            .resolve(
                "resources/patcher/DualAuthPatcher.java",
                tauri::path::BaseDirectory::Resource,
            )
            .ok()
            .and_then(|p: PathBuf| p.parent().map(PathBuf::from))
            .filter(|d: &PathBuf| d.join("DualAuthPatcher.java").exists());

        let resource_dir = if let Some(d) = resource_dir {
            log::info!("Using bundled patcher resource directory: {:?}", d);
            d
        } else {
            let mut d = env::current_dir()?;
            if !d.ends_with("src-tauri") && d.join("src-tauri").exists() {
                d = d.join("src-tauri");
            }
            let d = d.join("resources").join("patcher");
            log::info!("Using dev patcher resource directory: {:?}", d);
            d
        };


        let patcher_src = resource_dir.join("DualAuthPatcher.java");
        if !patcher_src.exists() {
            anyhow::bail!("DualAuthPatcher.java not found at {:?}", patcher_src);
        }

        let work_dir = crate::updater::env::get_hycore_data_dir().join("server_patcher");
        let _ = fs::create_dir_all(&work_dir);

        let patcher_java = work_dir.join("DualAuthPatcher.java");
        if let Err(e) = fs::copy(&patcher_src, &patcher_java) {
            anyhow::bail!(
                "Failed to copy DualAuthPatcher.java to writable dir {:?}: {}",
                work_dir,
                e
            );
        }
        let javac_exec =
            java_exec
                .parent()
                .unwrap()
                .join(if cfg!(windows) { "javac.exe" } else { "javac" });

        if !javac_exec.exists() {
            anyhow::bail!("javac not found in JRE/JDK bin directory: {:?}. Please ensure you have a JDK installed.", javac_exec);
        }

        log::info!("Checking if DualAuthPatcher is already compiled...");
        let patcher_class = work_dir.join("DualAuthPatcher.class");

        if !patcher_class.exists() {
            log::info!("Compiling DualAuthPatcher.java using {:?}...", javac_exec);
            let mut child = Command::new(&javac_exec)
                .arg("-cp")
                .arg(&javac_cp)
                .arg("DualAuthPatcher.java")
                .current_dir(&work_dir)
                .stderr(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()?;

            let stdout = child.stdout.take().unwrap();
            let stderr = child.stderr.take().unwrap();
            let mut stdout_reader = BufReader::new(stdout).lines();
            let mut stderr_reader = BufReader::new(stderr).lines();

            loop {
                tokio::select! {
                    result = stdout_reader.next_line() => {
                        match result {
                            Ok(Some(line)) => log::info!("[DualAuth-Compile] {}", line),
                            Ok(None) => break,
                            Err(e) => log::error!("[DualAuth-Compile] Error reading stdout: {}", e),
                        }
                    }
                    result = stderr_reader.next_line() => {
                        match result {
                            Ok(Some(line)) => log::error!("[DualAuth-Compile Error] {}", line),
                            Ok(None) => {},
                            Err(e) => log::error!("[DualAuth-Compile] Error reading stderr: {}", e),
                        }
                    }
                }
            }

            let status = child.wait().await?;
            if !status.success() {
                anyhow::bail!("Java compilation failed (see logs above)");
            }
            log::info!("Compilation successful.");
        } else {
            log::info!("DualAuthPatcher.class found, reusing existing compilation.");
        }

        log::info!("Running DualAuthPatcher against {:?}", server_jar);
        let mut child = Command::new(java_exec)
            .arg("-cp")
            .arg(&java_cp)
            .arg("DualAuthPatcher")
            .arg(server_jar)
            .env("HYTALE_AUTH_DOMAIN", &self.target_domain)
            .current_dir(&work_dir)
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;

        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        let mut stdout_reader = BufReader::new(stdout).lines();
        let mut stderr_reader = BufReader::new(stderr).lines();

        // DualAuthPatcher.java must use truncateToken() for any output that may contain tokens;
        // we never log raw tokens from child stdout/stderr.
        loop {
            tokio::select! {
                result = stdout_reader.next_line() => {
                    match result {
                        Ok(Some(line)) => log::info!("[DualAuth] {}", redact_jwt_if_present(&line)),
                        Ok(None) => break,
                        Err(e) => log::error!("[DualAuth] Error reading stdout: {}", e),
                    }
                }
                    result = stderr_reader.next_line() => {
                        match result {
                            Ok(Some(line)) => log::error!("[DualAuth Error] {}", redact_jwt_if_present(&line)),
                            Ok(None) => {}
                            Err(e) => log::error!("[DualAuth] Error reading stderr: {}", e),
                        }
                    }
            }
        }

        let status = child.wait().await?;
        if !status.success() {
            anyhow::bail!("Java patching failed (see logs above)");
        }

        let strategy = self.get_domain_strategy();
        let flag = PatchFlag {
            patched_at: time::OffsetDateTime::now_utc().to_string(),
            original_domain: ORIGINAL_DOMAIN.to_string(),
            target_domain: self.target_domain.clone(),
            patch_mode: strategy.mode,
            main_domain: strategy.main_domain,
            subdomain_prefix: strategy.subdomain_prefix,
            patcher_version: env!("CARGO_PKG_VERSION").to_string(),
            verified: "binary_contents".to_string(),
        };

        if let Ok(json) = serde_json::to_string_pretty(&flag) {
            let _ = fs::write(self.get_flag_path(server_jar), json);
        }

        Ok(())
    }

    #[cfg(target_os = "macos")]
    pub async fn sign_macos_app(&self, app_path: &Path) -> anyhow::Result<()> {
        log::info!("Signing macOS app bundle: {:?}", app_path);

        let _ = tokio::process::Command::new("xattr")
            .arg("-cr")
            .arg(app_path)
            .status()
            .await;

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

#[cfg(test)]
mod tests {
    use super::{ClientPatcher, DomainStrategy, PatchMode, ORIGINAL_DOMAIN};

    const MAIN_DOMAIN_MAX: usize = 9; // "hytale.com".len()
    const REPLACEMENT_PREFIX_MAX: usize = 15; // "https://tools.".len()
    const HTTPS_LEN: usize = 8; // "https://".len()

    fn patcher(domain: &str) -> ClientPatcher {
        ClientPatcher::new_for_test(domain)
    }

    fn strategy(domain: &str) -> DomainStrategy {
        patcher(domain).get_domain_strategy()
    }

    /// Garante que main_domain cabe no slot "hytale.com" (9 chars).
    #[test]
    fn main_domain_fits_in_hytale_com_slot() {
        for len in 4..=16 {
            let domain = "a".repeat(len);
            let s = strategy(&domain);
            assert!(
                s.main_domain.len() <= MAIN_DOMAIN_MAX,
                "len={}: main_domain \"{}\" has {} chars, max {}",
                len,
                s.main_domain,
                s.main_domain.len(),
                MAIN_DOMAIN_MAX
            );
        }
    }

    /// Garante que "https://" + subdomain_prefix cabe em "https://tools." (15 chars).
    #[test]
    fn replacement_prefix_fits_in_shortest_slot() {
        for len in 4..=16 {
            let domain = "a".repeat(len);
            let s = strategy(&domain);
            let pref = HTTPS_LEN + s.subdomain_prefix.len();
            assert!(
                pref <= REPLACEMENT_PREFIX_MAX,
                "len={}: \"https://\" + prefix \"{}\" = {} chars, max {}",
                len,
                s.subdomain_prefix,
                pref,
                REPLACEMENT_PREFIX_MAX
            );
        }
    }

    /// prefix + main == domain.
    #[test]
    fn prefix_plus_main_equals_domain() {
        for len in 4..=16 {
            let domain = "a".repeat(len);
            let s = strategy(&domain);
            let rebuilt = format!("{}{}", s.subdomain_prefix, s.main_domain);
            assert_eq!(rebuilt, domain, "len={}: prefix + main != domain", len);
        }
    }

    /// 4–9: Direct, subdomain_prefix vazia, main_domain == domain.
    #[test]
    fn direct_mode_4_to_9() {
        for len in 4..=9 {
            let domain = "x".repeat(len);
            let s = strategy(&domain);
            assert_eq!(s.mode, PatchMode::Direct, "len={}", len);
            assert!(s.subdomain_prefix.is_empty(), "len={}", len);
            assert_eq!(s.main_domain, domain, "len={}", len);
        }
    }

    /// 10: Split, prefix 1, main 9.
    #[test]
    fn split_mode_10_chars() {
        let domain = "abcdefghij";
        let s = strategy(domain);
        assert_eq!(s.mode, PatchMode::Split);
        assert_eq!(s.subdomain_prefix, "a");
        assert_eq!(s.main_domain, "bcdefghij");
        assert_eq!(s.main_domain.len(), 9);
    }

    /// 11–15: Split, prefix 6, main 5–9.
    #[test]
    fn split_mode_11_to_15() {
        for len in 11..=15 {
            let domain = "a".repeat(len);
            let s = strategy(&domain);
            assert_eq!(s.mode, PatchMode::Split, "len={}", len);
            assert_eq!(s.subdomain_prefix.len(), 6, "len={}", len);
            assert_eq!(s.main_domain.len(), len - 6, "len={}", len);
        }
    }

    /// 16: Split, prefix 7, main 9.
    #[test]
    fn split_mode_16_chars() {
        let domain = "auth.sanasol.wsx";
        assert_eq!(domain.len(), 16);

        let s = strategy(domain);
        assert_eq!(s.mode, PatchMode::Split);
        assert_eq!(s.subdomain_prefix, "auth.sa");
        assert_eq!(s.main_domain, "nasol.wsx");
        assert_eq!(s.subdomain_prefix.len(), 7);
        assert_eq!(s.main_domain.len(), 9);

        let rebuilt = format!("{}{}", s.subdomain_prefix, s.main_domain);
        assert_eq!(rebuilt, domain);
    }

    /// Exemplo 16 chars: prefix 7, main 9, tudo dentro dos limites do binário.
    #[test]
    fn real_domain_16_fits_binary_slots() {
        let s = strategy("auth.sanasol.wsx");
        assert_eq!(s.subdomain_prefix, "auth.sa");
        assert_eq!(s.main_domain, "nasol.wsx");
        assert!(s.main_domain.len() <= ORIGINAL_DOMAIN.len());
        assert!(HTTPS_LEN + s.subdomain_prefix.len() <= REPLACEMENT_PREFIX_MAX);
    }

    /// Exemplo 15 chars (auth.sanasol.ws): prefix 6, main 9.
    #[test]
    fn real_domain_15_auth_sanasol_ws() {
        let s = strategy("auth.sanasol.ws");
        assert_eq!(s.subdomain_prefix, "auth.s");
        assert_eq!(s.main_domain, "anasol.ws");
        assert_eq!(s.main_domain.len(), 9);
    }

    /// Exemplo: sanasol.ws (10) → prefix 1, main 9.
    #[test]
    fn real_domain_10_sanasol_ws() {
        let s = strategy("sanasol.ws");
        assert_eq!(s.mode, PatchMode::Split);
        assert_eq!(s.subdomain_prefix, "s");
        assert_eq!(s.main_domain, "anasol.ws");
    }
}
