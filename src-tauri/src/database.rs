use crate::updater::env::get_hycore_data_dir;
use rusqlite::{Connection, Result};
use std::sync::{Arc, Mutex};

/// Thread-safe SQLite connection pool
pub type DbPool = Arc<Mutex<Connection>>;

/// Initializes the database with versioned migrations
pub fn init_db() -> Result<DbPool> {
    let db_dir = get_hycore_data_dir();
    let db_path = db_dir.join("hycore.db");

    std::fs::create_dir_all(&db_dir)
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

    log::info!("Initializing database at {:?}", db_path);

    let mut conn = Connection::open(&db_path)?;
    configure_connection(&conn)?;

    // Smooth migration: detect legacy state and migrate if necessary
    migrate_from_legacy_system(&mut conn)?;

    run_migrations(&mut conn)?;

    log::info!("Database initialized successfully");
    Ok(Arc::new(Mutex::new(conn)))
}

fn configure_connection(conn: &Connection) -> Result<()> {
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.busy_timeout(std::time::Duration::from_secs(5))?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "cache_size", -10000)?;
    conn.pragma_update(None, "automatic_index", "ON")?;
    Ok(())
}

const SCHEMA_VERSION: i32 = 5;

/// Migrates from the legacy system (rusqlite_migration) to the new custom migration system
fn migrate_from_legacy_system(conn: &mut Connection) -> Result<()> {
    // Check if the legacy table exists
    let has_legacy: bool = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='rusqlite_migrations'",
        [],
        |row| {
            let count: i32 = row.get(0)?;
            Ok(count > 0)
        },
    )?;

    if !has_legacy {
        log::debug!("No legacy migration system found, starting fresh");
        return Ok(());
    }

    log::info!("Detected legacy migration system, performing smooth migration...");

    // Create version table if it doesn't exist
    conn.execute(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;

    // Detect database schema version based on existing tables/columns
    let detected_version = detect_current_schema(conn)?;
    log::info!("Detected current schema level: {}", detected_version);

    // Register detected version (if not already registered)
    let has_version: bool = conn.query_row("SELECT COUNT(*) FROM schema_version", [], |row| {
        let count: i32 = row.get(0)?;
        Ok(count > 0)
    })?;

    if !has_version && detected_version > 0 {
        log::info!("Registering detected schema version: {}", detected_version);
        // Register all versions up to the detected one
        for v in 1..=detected_version {
            conn.execute(
                "INSERT OR IGNORE INTO schema_version (version) VALUES (?)",
                [v],
            )?;
        }
    }

    // Optional: Drop legacy table to cleanup
    // conn.execute("DROP TABLE IF EXISTS rusqlite_migrations", [])?;

    log::info!("Legacy migration completed successfully");
    Ok(())
}

/// Detects the schema version based on existing tables and columns
fn detect_current_schema(conn: &Connection) -> Result<i32> {
    // Check which tables exist
    let has_launcher_settings = table_exists(conn, "launcher_settings")?;
    let has_installed_versions = table_exists(conn, "installed_versions")?;
    let has_profiles = table_exists(conn, "profiles")?;
    let has_mods = table_exists(conn, "mods")?;

    if !has_launcher_settings {
        return Ok(0); // Banco novo
    }

    // Version 1: has basic launcher_settings
    let mut version = 1;

    // Version 2: has installed_versions
    if has_installed_versions {
        version = 2;
    }

    // Version 3: has profiles and mods
    if has_profiles && has_mods {
        version = 3;
    }

    // Version 4: has override_os and override_arch columns
    if column_exists(conn, "launcher_settings", "override_os")? {
        version = 4;
    }

    // Version 5: has authentication columns
    if column_exists(conn, "launcher_settings", "online_mode")? {
        version = 5;
    }

    Ok(version)
}

fn table_exists(conn: &Connection, table: &str) -> Result<bool> {
    let count: i32 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?",
        [table],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

fn get_schema_version(conn: &Connection) -> Result<i32> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;

    match conn.query_row("SELECT MAX(version) FROM schema_version", [], |row| {
        row.get::<_, Option<i32>>(0)
    }) {
        Ok(Some(v)) => Ok(v),
        Ok(None) => Ok(0),
        Err(_) => Ok(0),
    }
}

fn set_schema_version(conn: &Connection, version: i32) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO schema_version (version) VALUES (?)",
        [version],
    )?;
    log::debug!("Schema version set to {}", version);
    Ok(())
}

fn column_exists(conn: &Connection, table: &str, column: &str) -> Result<bool> {
    let query = format!(
        "SELECT COUNT(*) FROM pragma_table_info('{}') WHERE name = ?",
        table
    );
    let count: i32 = conn.query_row(&query, [column], |row| row.get(0))?;
    Ok(count > 0)
}

fn add_column_if_not_exists(
    conn: &Connection,
    table: &str,
    column: &str,
    column_def: &str,
) -> Result<()> {
    if !column_exists(conn, table, column)? {
        let sql = format!("ALTER TABLE {} ADD COLUMN {}", table, column_def);
        conn.execute(&sql, [])?;
        log::info!("Added column '{}' to table '{}'", column, table);
    } else {
        log::debug!("Column '{}' already exists in table '{}'", column, table);
    }
    Ok(())
}

fn run_migrations(conn: &mut Connection) -> Result<()> {
    let current_version = get_schema_version(conn)?;
    log::info!("Current database schema version: {}", current_version);

    if current_version >= SCHEMA_VERSION {
        log::info!("Database is up to date (version {})", current_version);
        return Ok(());
    }

    // Migration 1: Basic tables
    if current_version < 1 {
        log::info!("Applying migration 1: Basic tables");
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS launcher_settings (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                ram_gb INTEGER NOT NULL DEFAULT 4,
                custom_java_args TEXT NOT NULL DEFAULT '',
                close_on_launch INTEGER NOT NULL DEFAULT 0,
                minimize_to_tray INTEGER NOT NULL DEFAULT 1,
                discord_rpc_enabled INTEGER NOT NULL DEFAULT 1,
                channel TEXT NOT NULL DEFAULT 'release',
                language TEXT NOT NULL DEFAULT 'auto',
                active_version INTEGER NOT NULL DEFAULT 0,
                player_name TEXT NOT NULL DEFAULT 'Player'
            );

            INSERT OR IGNORE INTO launcher_settings (id) VALUES (1);
            "#,
        )?;
        set_schema_version(conn, 1)?;
        log::info!("Migration 1 applied successfully");
    }

    // Migration 2: Installed versions system
    if current_version < 2 {
        log::info!("Applying migration 2: Installed versions");
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS installed_versions (
                version INTEGER PRIMARY KEY,
                channel TEXT NOT NULL,
                installed_at TEXT NOT NULL,
                last_modified TEXT,
                size INTEGER,
                etag TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_installed_versions_channel 
                ON installed_versions(channel);
            "#,
        )?;
        set_schema_version(conn, 2)?;
        log::info!("Migration 2 applied successfully");
    }

    // Migration 3: Profiles and mods system
    if current_version < 3 {
        log::info!("Applying migration 3: Profiles and mods");
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS profiles (
                name TEXT PRIMARY KEY,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS mods (
                id TEXT PRIMARY KEY,
                profile_name TEXT NOT NULL,
                name TEXT NOT NULL,
                slug TEXT,
                version TEXT,
                author TEXT,
                description TEXT,
                download_url TEXT,
                curse_forge_id INTEGER,
                file_id INTEGER,
                enabled INTEGER NOT NULL DEFAULT 1,
                installed_at TEXT NOT NULL,
                updated_at TEXT,
                file_path TEXT,
                icon_url TEXT,
                downloads INTEGER,
                category TEXT,
                latest_version TEXT,
                latest_file_id INTEGER,
                FOREIGN KEY(profile_name) REFERENCES profiles(name) 
                    ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_mods_profile 
                ON mods(profile_name);
            CREATE INDEX IF NOT EXISTS idx_mods_enabled 
                ON mods(enabled);
            CREATE INDEX IF NOT EXISTS idx_mods_curse_forge 
                ON mods(curse_forge_id);

            INSERT OR IGNORE INTO profiles (name, created_at) 
                VALUES ('Default', datetime('now'));
            "#,
        )?;
        set_schema_version(conn, 3)?;
        log::info!("Migration 3 applied successfully");
    }

    // Migration 4: System overrides
    if current_version < 4 {
        log::info!("Applying migration 4: System overrides");
        add_column_if_not_exists(conn, "launcher_settings", "override_os", "override_os TEXT")?;
        add_column_if_not_exists(
            conn,
            "launcher_settings",
            "override_arch",
            "override_arch TEXT",
        )?;
        set_schema_version(conn, 4)?;
        log::info!("Migration 4 applied successfully");
    }

    // Migration 5: Authentication system
    if current_version < 5 {
        log::info!("Applying migration 5: Authentication");
        add_column_if_not_exists(
            conn,
            "launcher_settings",
            "online_mode",
            "online_mode INTEGER NOT NULL DEFAULT 1",
        )?;
        add_column_if_not_exists(
            conn,
            "launcher_settings",
            "auth_domain",
            "auth_domain TEXT NOT NULL DEFAULT 'sanasol.ws'",
        )?;
        add_column_if_not_exists(
            conn,
            "launcher_settings",
            "player_id",
            "player_id TEXT NOT NULL DEFAULT ''",
        )?;
        set_schema_version(conn, 5)?;
        log::info!("Migration 5 applied successfully");
    }

    log::info!(
        "All migrations applied successfully. Database is now at version {}",
        SCHEMA_VERSION
    );
    Ok(())
}

pub fn get_conn(pool: &DbPool) -> Result<std::sync::MutexGuard<'_, Connection>> {
    pool.lock().map_err(|e| {
        log::error!("Failed to acquire database lock: {}", e);
        rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Mutex poisoned",
        )))
    })
}

pub fn with_transaction<F, T>(pool: &DbPool, f: F) -> Result<T>
where
    F: FnOnce(&Connection) -> Result<T>,
{
    let conn = get_conn(pool)?;
    conn.execute("BEGIN IMMEDIATE", [])?;

    match f(&conn) {
        Ok(result) => {
            conn.execute("COMMIT", [])?;
            Ok(result)
        }
        Err(e) => {
            let _ = conn.execute("ROLLBACK", []);
            Err(e)
        }
    }
}
