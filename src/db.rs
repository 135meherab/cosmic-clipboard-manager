use chrono::{DateTime, Utc};
use rusqlite::{Connection, Result, params};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const MAX_HISTORY: usize = 100;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ClipKind {
    Text,
    Image,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipEntry {
    pub id: i64,
    pub kind: ClipKind,
    /// Plain text content, or base64-encoded PNG for images
    pub content: String,
    /// Short preview shown in the list
    pub preview: String,
    pub pinned: bool,
    pub created_at: DateTime<Utc>,
}

pub struct Db {
    conn: Connection,
}

impl Db {
    pub fn open() -> Result<Self> {
        let path = data_path();
        std::fs::create_dir_all(path.parent().unwrap()).ok();
        let conn = Connection::open(&path)?;
        let db = Db { conn };
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS clips (
                id         INTEGER PRIMARY KEY AUTOINCREMENT,
                kind       TEXT    NOT NULL,
                content    TEXT    NOT NULL,
                preview    TEXT    NOT NULL,
                pinned     INTEGER NOT NULL DEFAULT 0,
                created_at TEXT    NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_created ON clips(created_at DESC);",
        )
    }

    /// Insert a new clip. Enforces MAX_HISTORY by pruning oldest non-pinned entries.
    pub fn insert(&self, kind: ClipKind, content: &str, preview: &str) -> Result<i64> {
        let now = Utc::now().to_rfc3339();
        let kind_str = match kind {
            ClipKind::Text => "text",
            ClipKind::Image => "image",
        };
        self.conn.execute(
            "INSERT INTO clips (kind, content, preview, pinned, created_at) VALUES (?1, ?2, ?3, 0, ?4)",
            params![kind_str, content, preview, now],
        )?;
        let id = self.conn.last_insert_rowid();
        self.prune()?;
        Ok(id)
    }

    /// Delete oldest non-pinned entries beyond MAX_HISTORY.
    fn prune(&self) -> Result<()> {
        self.conn.execute(
            "DELETE FROM clips WHERE pinned = 0 AND id NOT IN (
                SELECT id FROM clips WHERE pinned = 0
                ORDER BY created_at DESC LIMIT ?1
            )",
            params![MAX_HISTORY as i64],
        )?;
        Ok(())
    }

    /// Load all clips ordered newest first (pinned entries always on top).
    pub fn all(&self) -> Result<Vec<ClipEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, kind, content, preview, pinned, created_at
             FROM clips
             ORDER BY pinned DESC, created_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            let kind_str: String = row.get(1)?;
            let kind = if kind_str == "image" { ClipKind::Image } else { ClipKind::Text };
            let created_str: String = row.get(5)?;
            let created_at = DateTime::parse_from_rfc3339(&created_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
            Ok(ClipEntry {
                id: row.get(0)?,
                kind,
                content: row.get(2)?,
                preview: row.get(3)?,
                pinned: row.get::<_, i64>(4)? != 0,
                created_at,
            })
        })?;
        rows.collect()
    }

    pub fn toggle_pin(&self, id: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE clips SET pinned = 1 - pinned WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }

    pub fn delete(&self, id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM clips WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn clear_unpinned(&self) -> Result<()> {
        self.conn.execute("DELETE FROM clips WHERE pinned = 0", [])?;
        Ok(())
    }

    /// Returns true if the most recent entry already has this content (dedup).
    pub fn is_duplicate(&self, content: &str) -> Result<bool> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM clips WHERE content = ?1 ORDER BY created_at DESC LIMIT 1",
            params![content],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }
}

fn data_path() -> PathBuf {
    let base = std::env::var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
            PathBuf::from(home).join(".local/share")
        });
    base.join("clipmgr").join("clips.db")
}
