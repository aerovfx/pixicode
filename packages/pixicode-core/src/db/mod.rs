//! Database layer — SQLite via `rusqlite`
//!
//! Features:
//!  - Automatic PRAGMA setup (WAL, busy_timeout, cache_size, foreign_keys)
//!  - Embedded migration system (sequential numbered SQL files)
//!  - Backward-compat: detects `opencode.db` and copies to `pixicode.db`
//!  - Typed transaction wrapper

pub mod migrate;
pub mod models;
pub mod session_io;

use anyhow::{Context, Result};
use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tracing::{info, warn};

// ─────────────────────────────────────────────────────────────────────────────
//  Database wrapper
// ─────────────────────────────────────────────────────────────────────────────

/// Thread-safe SQLite database handle.
#[derive(Clone)]
pub struct Database {
    inner: Arc<Mutex<Connection>>,
}

impl Database {
    /// Open (or create) the pixicode.db, running migrations automatically.
    ///
    /// If `$data_dir/opencode.db` exists and `pixicode.db` does not, the
    /// legacy database is copied first (backward compatibility).
    pub async fn open(data_dir: &Path) -> Result<Self> {
        let db_path = data_dir.join("pixicode.db");
        let legacy_path = data_dir.join("opencode.db");

        // Backward-compat migration: opencode.db → pixicode.db
        if !db_path.exists() && legacy_path.exists() {
            info!(
                legacy = %legacy_path.display(),
                target = %db_path.display(),
                "Detected legacy opencode.db — copying to pixicode.db"
            );
            Self::copy_legacy_db(&legacy_path, &db_path).await?;
        }

        // Open connection (creates the file when absent)
        let conn = Connection::open(&db_path)
            .with_context(|| format!("open SQLite at {}", db_path.display()))?;

        // PRAGMA configuration before any queries
        Self::configure_pragmas(&conn)?;

        let db = Database {
            inner: Arc::new(Mutex::new(conn)),
        };

        // Run embedded migrations
        db.with(|c| migrate::run(c))?;

        info!(path = %db_path.display(), "Database ready");
        Ok(db)
    }

    // ─── PRAGMA setup ──────────────────────────────────────────────────────

    fn configure_pragmas(conn: &Connection) -> Result<()> {
        conn.execute_batch(
            "
            PRAGMA journal_mode = WAL;
            PRAGMA busy_timeout = 5000;
            PRAGMA cache_size   = -20000;  /* 20 MB page cache */
            PRAGMA foreign_keys = ON;
            PRAGMA synchronous  = NORMAL;
            PRAGMA temp_store   = MEMORY;
            ",
        )
        .context("configure SQLite PRAGMAs")?;
        Ok(())
    }

    // ─── Backward-compat copy ──────────────────────────────────────────────

    async fn copy_legacy_db(src: &Path, dst: &Path) -> Result<()> {
        // Simple file copy for the db file.  WAL/SHM files are transient and
        // do not need to be carried over (WAL mode will be set on first open).
        tokio::fs::copy(src, dst)
            .await
            .with_context(|| format!("copy {} → {}", src.display(), dst.display()))?;

        // Best-effort copy of WAL/SHM if they exist (prevents data loss on
        // un-checkpointed writes from a running Bun process)
        for ext in &["-wal", "-shm"] {
            let src_aux = PathBuf::from(format!("{}{}", src.display(), ext));
            let dst_aux = PathBuf::from(format!("{}{}", dst.display(), ext));
            if src_aux.exists() {
                let _ = tokio::fs::copy(&src_aux, &dst_aux).await;
            }
        }
        Ok(())
    }

    // ─── Transaction helpers ───────────────────────────────────────────────

    /// Run a closure with exclusive access to the underlying connection.
    /// Mirrors `Database.use()` from the TypeScript API.
    pub fn with<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&Connection) -> Result<T>,
    {
        let conn = self.inner.lock().expect("db mutex poisoned");
        f(&conn)
    }

    /// Run a closure inside a BEGIN / COMMIT transaction.
    /// Mirrors `Database.transaction()` from the TypeScript API.
    pub fn transaction<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&Connection) -> Result<T>,
    {
        let conn = self.inner.lock().expect("db mutex poisoned");
        conn.execute("BEGIN IMMEDIATE", [])?;
        match f(&conn) {
            Ok(v) => {
                conn.execute("COMMIT", [])?;
                Ok(v)
            }
            Err(e) => {
                let _ = conn.execute("ROLLBACK", []);
                Err(e)
            }
        }
    }
}
