//! Embedded SQL migration runner.
//!
//! All migrations are stored as const strings in the binary so there are no
//! external file dependencies. Migrations run in version order and are
//! idempotent (tracked in `_migrations` table).

use anyhow::{Context, Result};
use rusqlite::Connection;
use tracing::info;

// ─────────────────────────────────────────────────────────────────────────────
//  Embedded migrations (ported from Drizzle SQL files — chronological order)
// ─────────────────────────────────────────────────────────────────────────────

struct Migration {
    version: u32,
    name: &'static str,
    sql: &'static str,
}

const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "initial_schema",
        sql: r#"
CREATE TABLE IF NOT EXISTS project (
    id             TEXT PRIMARY KEY,
    worktree       TEXT NOT NULL,
    vcs            TEXT,
    name           TEXT,
    icon_url       TEXT,
    icon_color     TEXT,
    commands       TEXT,
    time_created   INTEGER NOT NULL,
    time_updated   INTEGER NOT NULL,
    time_initialized INTEGER,
    sandboxes      TEXT NOT NULL DEFAULT '[]'
);

CREATE TABLE IF NOT EXISTS session (
    id                   TEXT PRIMARY KEY,
    project_id           TEXT NOT NULL,
    parent_id            TEXT,
    workspace_id         TEXT,
    slug                 TEXT NOT NULL,
    directory            TEXT NOT NULL,
    title                TEXT NOT NULL,
    version              TEXT NOT NULL,
    share_url            TEXT,
    summary_additions    INTEGER,
    summary_deletions    INTEGER,
    summary_files        INTEGER,
    summary_diffs        TEXT,
    revert               TEXT,
    permission           TEXT,
    time_created         INTEGER NOT NULL,
    time_updated         INTEGER NOT NULL,
    time_compacting      INTEGER,
    time_archived        INTEGER,
    FOREIGN KEY (project_id) REFERENCES project(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS message (
    id           TEXT PRIMARY KEY,
    session_id   TEXT NOT NULL,
    time_created INTEGER NOT NULL,
    time_updated INTEGER NOT NULL,
    data         TEXT NOT NULL,
    FOREIGN KEY (session_id) REFERENCES session(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS part (
    id           TEXT PRIMARY KEY,
    message_id   TEXT NOT NULL,
    session_id   TEXT NOT NULL,
    time_created INTEGER NOT NULL,
    time_updated INTEGER NOT NULL,
    data         TEXT NOT NULL,
    FOREIGN KEY (message_id) REFERENCES message(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS permission (
    project_id   TEXT PRIMARY KEY,
    time_created INTEGER NOT NULL,
    time_updated INTEGER NOT NULL,
    data         TEXT NOT NULL,
    FOREIGN KEY (project_id) REFERENCES project(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS todo (
    session_id   TEXT NOT NULL,
    content      TEXT NOT NULL,
    status       TEXT NOT NULL,
    priority     TEXT NOT NULL,
    position     INTEGER NOT NULL,
    time_created INTEGER NOT NULL,
    time_updated INTEGER NOT NULL,
    PRIMARY KEY (session_id, position),
    FOREIGN KEY (session_id) REFERENCES session(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS session_share (
    session_id   TEXT PRIMARY KEY,
    id           TEXT NOT NULL,
    secret       TEXT NOT NULL,
    url          TEXT NOT NULL,
    time_created INTEGER NOT NULL,
    time_updated INTEGER NOT NULL,
    FOREIGN KEY (session_id) REFERENCES session(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS workspace (
    id          TEXT PRIMARY KEY,
    branch      TEXT,
    project_id  TEXT NOT NULL,
    type        TEXT NOT NULL DEFAULT 'local',
    name        TEXT,
    directory   TEXT,
    extra       TEXT,
    FOREIGN KEY (project_id) REFERENCES project(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS control_account (
    email         TEXT NOT NULL,
    url           TEXT NOT NULL,
    access_token  TEXT NOT NULL,
    refresh_token TEXT NOT NULL,
    token_expiry  INTEGER,
    active        INTEGER NOT NULL,
    time_created  INTEGER NOT NULL,
    time_updated  INTEGER NOT NULL,
    PRIMARY KEY (email, url)
);

-- Indexes
CREATE INDEX IF NOT EXISTS message_session_idx ON message (session_id);
CREATE INDEX IF NOT EXISTS part_message_idx    ON part (message_id);
CREATE INDEX IF NOT EXISTS part_session_idx    ON part (session_id);
CREATE INDEX IF NOT EXISTS session_project_idx ON session (project_id);
CREATE INDEX IF NOT EXISTS session_parent_idx  ON session (parent_id);
CREATE INDEX IF NOT EXISTS session_workspace_idx ON session (workspace_id);
CREATE INDEX IF NOT EXISTS todo_session_idx    ON todo (session_id);
"#,
    },
];

// ─────────────────────────────────────────────────────────────────────────────
//  Runner
// ─────────────────────────────────────────────────────────────────────────────

/// Run all pending migrations against `conn`.
/// Safe to call multiple times — already-applied migrations are skipped.
pub fn run(conn: &Connection) -> Result<()> {
    // Ensure tracking table exists
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS _migrations (
            version      INTEGER PRIMARY KEY,
            name         TEXT NOT NULL,
            applied_at   INTEGER NOT NULL DEFAULT (unixepoch())
        );",
    )
    .context("create _migrations table")?;

    let applied: std::collections::HashSet<u32> = {
        let mut stmt = conn.prepare("SELECT version FROM _migrations")?;
        let rows = stmt.query_map([], |row| row.get::<_, u32>(0))?;
        rows.collect::<rusqlite::Result<_>>()?
    };

    for m in MIGRATIONS {
        if applied.contains(&m.version) {
            continue;
        }
        info!(version = m.version, name = m.name, "Applying migration");

        // Split on Drizzle-style `-->statement-breakpoint` or bare semicolons
        // then execute each statement.
        let statements: Vec<&str> = m
            .sql
            .split("-->statement-breakpoint")
            .flat_map(|chunk| chunk.split(";\n"))
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        for stmt in &statements {
            let sql = if stmt.ends_with(';') {
                stmt.to_string()
            } else {
                format!("{};", stmt)
            };
            conn.execute_batch(&sql)
                .with_context(|| format!("migration {}: {}", m.version, &sql[..sql.len().min(120)]))?;
        }

        conn.execute(
            "INSERT INTO _migrations (version, name) VALUES (?1, ?2)",
            rusqlite::params![m.version, m.name],
        )?;
    }

    Ok(())
}
