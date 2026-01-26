pub mod check;
pub mod cleanup;
pub mod download;
pub mod env;
pub mod java;
pub mod system;
pub mod types;

pub use download::download_with_retry;
pub use system::check_system_requirements;
pub use types::{LocalManifest, LocalVersionInfo, SystemRequirements, UpdateStatus};

use crate::database::DbPool;
use crate::error::AppError;
use once_cell::sync::Lazy;
use redb::ReadableTable;
use std::fs;
use std::process::Command;
use std::sync::Mutex;
use tauri::{Emitter, Window};

static UPDATE_LOCK: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(false));

use crate::platform::{get_hytale_arch, get_hytale_os};

const INSTALLED_VERSIONS_TABLE: redb::TableDefinition<'static, u32, &[u8]> =
    redb::TableDefinition::new("installed_versions");

pub fn get_local_manifest(pool: &DbPool) -> LocalManifest {
    let mut installed = Vec::new();

    let read_txn = match pool.begin_read() {
        Ok(txn) => txn,
        Err(e) => {
            log::error!("Failed to begin read transaction: {}. Using default.", e);
            return LocalManifest::default();
        }
    };

    let table = match read_txn.open_table(INSTALLED_VERSIONS_TABLE) {
        Ok(t) => t,
        Err(e) => {
            log::error!(
                "Failed to open installed_versions table: {}. Using default.",
                e
            );
            return LocalManifest::default();
        }
    };

    // Iterate over all installed versions
    let iter = match table.iter() {
        Ok(i) => i,
        Err(e) => {
            log::error!("Failed to iterate installed versions: {}", e);
            return LocalManifest::default();
        }
    };

    for item in iter {
        if let Ok((version_key, data)) = item {
            let version: u32 = version_key.value();

            match bincode::deserialize::<LocalVersionInfo>(data.value()) {
                Ok(info) => {
                    // Validate existence of folder
                    let dir = env::get_version_dir(info.version);
                    if dir.exists() {
                        installed.push(info);
                    } else {
                        // Note: We can't delete from the table during iteration
                        // This cleanup will happen on next save
                        log::warn!(
                            "Version {} directory not found, will be cleaned up",
                            version
                        );
                    }
                }
                Err(e) => {
                    log::error!("Failed to deserialize version {}: {}", version, e);
                }
            }
        }
    }

    // Migration from old JSON manifest
    if installed.is_empty() {
        let json_path = env::get_versions_manifest_path();
        if json_path.exists() {
            if let Ok(content) = fs::read_to_string(&json_path) {
                if let Ok(manifest) = serde_json::from_str::<LocalManifest>(&content) {
                    log::info!(
                        "Migrating {} versions from JSON manifest",
                        manifest.installed.len()
                    );
                    for v in &manifest.installed {
                        let _ = save_installed_version(pool, v);
                    }
                    installed = manifest.installed;
                }
            }
        }
    }

    let settings = crate::settings::load_settings(pool);
    LocalManifest {
        installed,
        active_version: settings.active_version,
    }
}

fn save_installed_version(
    pool: &DbPool,
    version_info: &LocalVersionInfo,
) -> Result<(), Box<dyn std::error::Error>> {
    let data = bincode::serialize(version_info)?;

    let write_txn = pool.begin_write()?;
    {
        let mut table = write_txn.open_table(INSTALLED_VERSIONS_TABLE)?;
        table.insert(version_info.version, data.as_slice())?;
    }
    write_txn.commit()?;

    Ok(())
}

pub async fn is_update_available(pool: &DbPool) -> Result<(bool, u32), AppError> {
    let settings = crate::settings::load_settings(pool);
    let active_version = settings.active_version;

    // If no version is active or latest isn't found, find latest
    let latest = check::find_latest_version(&settings, &settings.channel)
        .await
        .map_err(AppError::from)?;

    let target_version = if active_version == 0 || active_version > latest {
        latest
    } else {
        active_version
    };

    let manifest = get_local_manifest(pool);
    let local_info = manifest
        .installed
        .into_iter()
        .find(|v| v.version == target_version);

    // If target version isn't installed at all, it's "available"
    let is_installed = local_info.is_some();
    if !is_installed {
        return Ok((true, target_version));
    }

    let local = local_info.unwrap();

    // If it's installed, check for hotfixes/metadata changes
    match check::get_remote_metadata(&settings, target_version, &settings.channel).await {
        Ok(remote) => {
            let size_changed = local.size.is_some() && remote.size != local.size;
            let modified_changed =
                local.last_modified.is_some() && remote.last_modified != local.last_modified;
            let etag_changed =
                local.etag.is_some() && remote.etag.is_some() && remote.etag != local.etag;

            if size_changed || modified_changed || etag_changed {
                log::info!("Update detected for version {}: size_changed={}, modified_changed={}, etag_changed={}", 
                    target_version, size_changed, modified_changed, etag_changed);
                return Ok((true, target_version));
            }
        }
        Err(e) => {
            log::warn!(
                "Failed to fetch remote metadata for version {}: {}",
                target_version,
                e
            );
        }
    }

    Ok((false, target_version))
}

pub async fn run_update(pool: &DbPool, window: Window) -> Result<(), AppError> {
    let reqs = check_system_requirements().await;
    if !reqs.meets_requirements {
        return Err(AppError::Unknown(format!(
            "System requirements not met. Space: {}GB, Internet: {}",
            reqs.free_space_gb, reqs.has_internet
        )));
    }

    let butler = system::ensure_butler(&window)
        .await
        .map_err(AppError::from)?;

    let settings = crate::settings::load_settings(pool);
    let latest = check::find_latest_version(&settings, &settings.channel)
        .await
        .map_err(AppError::from)?;
    let remote_metadata = check::get_remote_metadata(&settings, latest, &settings.channel)
        .await
        .ok();

    let os = get_hytale_os(&settings);
    let arch = get_hytale_arch(&settings);

    let patch_url = format!(
        "https://game-patches.hytale.com/patches/{}/{}/{}/0/{}.pwr",
        os, arch, &settings.channel, latest
    );

    let pwr_path = env::get_hycore_data_dir().join(format!("{}.pwr", latest));

    download_with_retry(&patch_url, &pwr_path, &window, 5, None)
        .await
        .map_err(AppError::from)?;

    if pwr_path.exists() {
        let file_size = fs::metadata(&pwr_path)
            .map_err(|e| AppError::Unknown(e.to_string()))?
            .len();

        if file_size < 100_000 {
            let _ = fs::remove_file(&pwr_path);
            return Err(AppError::Unknown(
                "Downloaded file too small or corrupted. Please try again.".to_string(),
            ));
        }
    }

    let _ = window.emit(
        "update-status",
        UpdateStatus {
            stage: "install".to_string(),
            progress: 50.0,
            message: "Applying patch with Butler...".to_string(),
        },
    );

    let game_dir = env::get_version_dir(latest);
    let _ = fs::create_dir_all(&game_dir);
    let staging_dir = game_dir.join("staging");

    // Robust cleanup: ensure staging is completely empty before Butler starts
    if staging_dir.exists() {
        let _ = fs::remove_dir_all(&staging_dir);
    }
    let _ = fs::create_dir_all(&staging_dir);

    log::info!("Applying patch with Butler to {:?}", game_dir);
    let output = Command::new(butler)
        .arg("apply")
        .arg("--staging-dir")
        .arg(&staging_dir)
        .arg(&pwr_path)
        .arg(&game_dir)
        .output()
        .map_err(|e| AppError::Unknown(e.to_string()))?;

    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let err_msg = format!(
            "Butler failed!\nSTDOUT: {}\nSTDERR: {}",
            stdout.trim(),
            stderr.trim()
        );
        log::error!("{}", err_msg);

        if pwr_path.exists() {
            let _ = fs::remove_file(&pwr_path);
        }

        return Err(AppError::Unknown(
            "Installation tool error. The update may be corrupted. Check logs.".to_string(),
        ));
    }

    let _ = fs::remove_file(pwr_path);
    let _ = fs::remove_dir_all(staging_dir);

    // Update local manifest in DB
    let version_info = if let Some(mut info) = remote_metadata {
        info.installed_at = Some(time::OffsetDateTime::now_utc().to_string());
        info
    } else {
        LocalVersionInfo {
            version: latest,
            channel: settings.channel.clone(),
            installed_at: Some(time::OffsetDateTime::now_utc().to_string()),
            ..Default::default()
        }
    };

    let _ = save_installed_version(pool, &version_info);

    // Update active version in settings
    let mut settings = crate::settings::load_settings(pool);
    settings.active_version = latest;
    let _ = crate::settings::save_settings(pool, &settings);

    log::info!("Update complete! Version installed to {:?}", game_dir);

    let _ = window.emit(
        "update-status",
        UpdateStatus {
            stage: "done".to_string(),
            progress: 100.0,
            message: "Update complete!".to_string(),
        },
    );

    Ok(())
}

#[tauri::command]
pub async fn check_update_requirements() -> Result<SystemRequirements, AppError> {
    log::info!("Checking update requirements...");
    let reqs = check_system_requirements().await;
    log::info!(
        "Requirements checked: meets_requirements={}",
        reqs.meets_requirements
    );
    Ok(reqs)
}

#[tauri::command]
pub async fn check_for_game_update(
    db_pool: tauri::State<'_, DbPool>,
) -> Result<(bool, u32), AppError> {
    log::info!("Checking for game update...");
    match is_update_available(&db_pool).await {
        Ok(res) => {
            log::info!("Game update check: available={}, version={}", res.0, res.1);
            Ok(res)
        }
        Err(e) => {
            log::error!("Failed to check for game update: {}", e);
            Err(e)
        }
    }
}

#[tauri::command]
pub async fn start_game_update(
    db_pool: tauri::State<'_, DbPool>,
    window: tauri::Window,
) -> Result<(), AppError> {
    {
        let mut lock = UPDATE_LOCK.lock().unwrap();
        if *lock {
            log::warn!("Update already in progress, ignoring request");
            return Err(AppError::Unknown(
                "Uma atualização já está em andamento.".to_string(),
            ));
        }
        *lock = true;
    }

    // Access inner pool to keep it alive or clone if needed, but we can just pass reference to async fn
    // However, for async, we should clone the Arc<Database> (DbPool is Arc)
    let pool = db_pool.inner().clone();

    log::info!("Starting game update process...");
    let result = run_update(&pool, window).await;

    {
        let mut lock = UPDATE_LOCK.lock().unwrap();
        *lock = false;
    }

    match result {
        Ok(_) => {
            log::info!("Game update process finished successfully");
            Ok(())
        }
        Err(e) => {
            log::error!("Game update process failed: {}", e);
            Err(e)
        }
    }
}

#[tauri::command]
pub async fn java_bin_path_command() -> std::path::PathBuf {
    log::info!("Frontend requested Java binary path");
    java::get_java_bin_path()
}

#[tauri::command]
pub fn get_local_manifest_command(db_pool: tauri::State<DbPool>) -> LocalManifest {
    get_local_manifest(&db_pool)
}

#[tauri::command]
pub async fn get_available_versions_command(
    db_pool: tauri::State<'_, DbPool>,
) -> Result<Vec<u32>, AppError> {
    let settings = crate::settings::load_settings(&db_pool);
    check::find_all_versions(&settings, &settings.channel)
        .await
        .map_err(AppError::from)
}

#[tauri::command]
pub fn switch_version_command(db_pool: tauri::State<DbPool>, version: u32) -> Result<(), AppError> {
    let mut settings = crate::settings::load_settings(&db_pool);
    settings.active_version = version;
    crate::settings::save_settings(&db_pool, &settings)
        .map_err(|e| AppError::Unknown(e.to_string()))?;
    log::info!("Switched active version to {}", version);
    Ok(())
}
