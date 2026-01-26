use crate::updater::env::get_hycore_data_dir;
use std::sync::Arc;

pub type DbPool = Arc<redb::Database>;

// Centralized table definitions
pub const LAUNCHER_SETTINGS_TABLE: redb::TableDefinition<'static, &str, &[u8]> =
    redb::TableDefinition::new("launcher_settings");
pub const INSTALLED_VERSIONS_TABLE: redb::TableDefinition<'static, u32, &[u8]> =
    redb::TableDefinition::new("installed_versions");
pub const PROFILES_TABLE: redb::TableDefinition<'static, &str, &[u8]> =
    redb::TableDefinition::new("profiles");
pub const MODS_TABLE: redb::TableDefinition<'static, &str, &[u8]> =
    redb::TableDefinition::new("mods");
pub const SETTINGS_TABLE: redb::TableDefinition<'static, &str, &[u8]> =
    redb::TableDefinition::new("settings");

/// Initializes the redb database from scratch
pub fn init_db() -> Result<DbPool, Box<dyn std::error::Error>> {
    let db_dir = get_hycore_data_dir();
    let db_path = db_dir.join("hycore.redb");

    std::fs::create_dir_all(&db_dir)?;

    log::info!("Initializing redb database at {:?}", db_path);

    let db = redb::Database::create(&db_path)?;

    // Create tables
    {
        let write_txn = db.begin_write()?;
        {
            let _ = write_txn.open_table(LAUNCHER_SETTINGS_TABLE)?;
            let _ = write_txn.open_table(INSTALLED_VERSIONS_TABLE)?;
            let _ = write_txn.open_table(PROFILES_TABLE)?;
            let _ = write_txn.open_table(MODS_TABLE)?;
            let _ = write_txn.open_table(SETTINGS_TABLE)?;
        }
        write_txn.commit()?;
    }

    log::info!("Database initialized successfully");
    Ok(Arc::new(db))
}
