use crate::updater::env::get_hycore_data_dir;
use once_cell::sync::Lazy;
use rusqlite::{Connection, Result};
use std::sync::Mutex;

pub static DB_CONN: Lazy<Mutex<Option<Connection>>> = Lazy::new(|| Mutex::new(None));

pub fn init_db() -> Result<()> {
    let db_path = get_hycore_data_dir().join("hycore.db");
    log::info!("Initializing database at {:?}", db_path);

    let conn = Connection::open(db_path)?;

    // Create tables
    conn.execute(
        "CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS launcher_settings (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            ram_gb INTEGER DEFAULT 4,
            custom_java_args TEXT DEFAULT '',
            close_on_launch INTEGER DEFAULT 0,
            minimize_to_tray INTEGER DEFAULT 1,
            discord_rpc_enabled INTEGER DEFAULT 1,
            channel TEXT DEFAULT 'release',
            language TEXT DEFAULT 'auto',
            active_version INTEGER DEFAULT 0,
            player_name TEXT DEFAULT 'Player',
            override_os TEXT,
            override_arch TEXT,
            online_mode INTEGER DEFAULT 1,
            auth_domain TEXT DEFAULT 'sanasol.ws',
            player_id TEXT DEFAULT ''
        )",
        [],
    )?;

    let _ = conn.execute(
        "ALTER TABLE launcher_settings ADD COLUMN override_os TEXT",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE launcher_settings ADD COLUMN override_arch TEXT",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE launcher_settings ADD COLUMN online_mode INTEGER DEFAULT 1",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE launcher_settings ADD COLUMN auth_domain TEXT DEFAULT 'sanasol.ws'",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE launcher_settings ADD COLUMN player_id TEXT DEFAULT ''",
        [],
    );

    // Ensure single row exists
    conn.execute(
        "INSERT OR IGNORE INTO launcher_settings (id) VALUES (1)",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS installed_versions (
            version INTEGER PRIMARY KEY,
            channel TEXT NOT NULL,
            installed_at TEXT,
            last_modified TEXT,
            size INTEGER,
            etag TEXT
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS profiles (
            name TEXT PRIMARY KEY,
            created_at TEXT NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS mods (
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
            enabled INTEGER DEFAULT 1,
            installed_at TEXT,
            updated_at TEXT,
            file_path TEXT,
            icon_url TEXT,
            downloads INTEGER,
            category TEXT,
            latest_version TEXT,
            latest_file_id INTEGER,
            FOREIGN KEY(profile_name) REFERENCES profiles(name) ON DELETE CASCADE
        )",
        [],
    )?;

    // Ensure default profile exists
    conn.execute(
        "INSERT OR IGNORE INTO profiles (name, created_at) VALUES (?, ?)",
        ["Default", &time::OffsetDateTime::now_utc().to_string()],
    )?;

    let mut guard = DB_CONN.lock().unwrap();
    *guard = Some(conn);

    Ok(())
}

pub fn get_conn() -> rusqlite::Connection {
    let db_path = get_hycore_data_dir().join("hycore.db");
    match Connection::open(&db_path) {
        Ok(c) => c,
        Err(e) => {
            log::error!("Critical: Failed to open database at {:?}: {}", db_path, e);
            // This is still a point where the app can't really function without DB,
            // but we at least log it properly before the inevitable panic if caller doesn't handle.
            // Ideally we'd return Result, but this function is used everywhere.
            // For now, let's keep it returning Connection but handle the open better or refactor callers.
            // Actually, if we want to remove 'expect', we MUST return Result or have a fallback.
            // Since this is a local DB, if we can't open it, the app is broken.
            panic!("Critical database error: {}", e);
        }
    }
}
