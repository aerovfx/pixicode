//! Session export/import — read/write session + messages as JSON.

use anyhow::{Context, Result};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::db::Database;

#[derive(Serialize, Deserialize)]
pub struct SessionRow {
    pub id: String,
    pub project_id: String,
    pub parent_id: Option<String>,
    pub workspace_id: Option<String>,
    pub slug: String,
    pub directory: String,
    pub title: String,
    pub version: String,
    pub share_url: Option<String>,
    pub summary_additions: Option<i64>,
    pub summary_deletions: Option<i64>,
    pub summary_files: Option<i64>,
    pub summary_diffs: Option<String>,
    pub revert: Option<String>,
    pub permission: Option<String>,
    pub time_created: i64,
    pub time_updated: i64,
    pub time_compacting: Option<i64>,
    pub time_archived: Option<i64>,
}

#[derive(Serialize, Deserialize)]
pub struct MessageRow {
    pub id: String,
    pub session_id: String,
    pub time_created: i64,
    pub time_updated: i64,
    pub data: String,
}

#[derive(Serialize, Deserialize)]
pub struct SessionExport {
    pub session: SessionRow,
    pub messages: Vec<MessageRow>,
}

/// Export one session and its messages to JSON.
pub fn export_session(db: &Database, session_id: &str) -> Result<SessionExport> {
    db.with(|conn| {
        let session: SessionRow = conn
            .query_row(
                "SELECT id, project_id, parent_id, workspace_id, slug, directory, title, version,
                 share_url, summary_additions, summary_deletions, summary_files, summary_diffs,
                 revert, permission, time_created, time_updated, time_compacting, time_archived
                 FROM session WHERE id = ?1",
                params![session_id],
                |row| {
                    Ok(SessionRow {
                        id: row.get(0)?,
                        project_id: row.get(1)?,
                        parent_id: row.get(2)?,
                        workspace_id: row.get(3)?,
                        slug: row.get(4)?,
                        directory: row.get(5)?,
                        title: row.get(6)?,
                        version: row.get(7)?,
                        share_url: row.get(8)?,
                        summary_additions: row.get(9)?,
                        summary_deletions: row.get(10)?,
                        summary_files: row.get(11)?,
                        summary_diffs: row.get(12)?,
                        revert: row.get(13)?,
                        permission: row.get(14)?,
                        time_created: row.get(15)?,
                        time_updated: row.get(16)?,
                        time_compacting: row.get(17)?,
                        time_archived: row.get(18)?,
                    })
                },
            )
            .context("session not found")?;

        let mut stmt = conn.prepare(
            "SELECT id, session_id, time_created, time_updated, data
             FROM message WHERE session_id = ?1 ORDER BY time_created ASC",
        )?;
        let messages: Vec<MessageRow> = stmt
            .query_map(params![session_id], |row| {
                Ok(MessageRow {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    time_created: row.get(2)?,
                    time_updated: row.get(3)?,
                    data: row.get(4)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(SessionExport { session, messages })
    })
}

/// Import session + messages from JSON; returns the session id.
pub fn import_session(db: &Database, export: &SessionExport) -> Result<String> {
    let now = chrono::Utc::now().timestamp();
    db.transaction(|conn| {
        conn.execute(
            "INSERT OR IGNORE INTO project (id, worktree, time_created, time_updated, sandboxes)
             VALUES (?1, ?2, ?3, ?4, '[]')",
            params![
                export.session.project_id,
                export.session.directory,
                now,
                now,
            ],
        )?;
        conn.execute(
            "INSERT OR REPLACE INTO session (
             id, project_id, parent_id, workspace_id, slug, directory, title, version,
             share_url, summary_additions, summary_deletions, summary_files, summary_diffs,
             revert, permission, time_created, time_updated, time_compacting, time_archived)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)",
            params![
                export.session.id,
                export.session.project_id,
                export.session.parent_id,
                export.session.workspace_id,
                export.session.slug,
                export.session.directory,
                export.session.title,
                export.session.version,
                export.session.share_url,
                export.session.summary_additions,
                export.session.summary_deletions,
                export.session.summary_files,
                export.session.summary_diffs,
                export.session.revert,
                export.session.permission,
                export.session.time_created,
                export.session.time_updated,
                export.session.time_compacting,
                export.session.time_archived,
            ],
        )?;
        for msg in &export.messages {
            conn.execute(
                "INSERT OR REPLACE INTO message (id, session_id, time_created, time_updated, data)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![msg.id, msg.session_id, msg.time_created, msg.time_updated, msg.data],
            )?;
        }
        Ok(export.session.id.clone())
    })
}

/// Export session to JSON string.
pub fn export_session_json(db: &Database, session_id: &str) -> Result<String> {
    let data = export_session(db, session_id)?;
    serde_json::to_string_pretty(&data).context("serialise export")
}

/// Export session to markdown (title + messages as role: content blocks).
pub fn export_session_markdown(db: &Database, session_id: &str) -> Result<String> {
    let data = export_session(db, session_id)?;
    let mut out = format!("# {}\n\n", data.session.title);
    for msg in &data.messages {
        let obj: serde_json::Value =
            serde_json::from_str(&msg.data).unwrap_or(serde_json::Value::Null);
        let role = obj.get("role").and_then(|r| r.as_str()).unwrap_or("unknown");
        let content = obj.get("content").and_then(|c| c.as_str()).unwrap_or("");
        out.push_str(&format!("## {}\n\n{}\n\n", role, content));
    }
    Ok(out)
}

/// Read export from a JSON file and import into db.
pub fn import_session_from_path(db: &Database, path: &Path) -> Result<String> {
    let s =
        std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let export: SessionExport =
        serde_json::from_str(&s).context("parse session export JSON")?;
    import_session(db, &export)
}
