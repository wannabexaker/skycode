//! SHA256-based extraction cache backed by SQLite

use anyhow::Result;
use rusqlite::{params, Connection};
use sha2::{Digest, Sha256};

pub struct ExtractionCache {
    conn: Connection,
}

impl ExtractionCache {
    pub fn open(db_path: &str) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS cache (
                file_path   TEXT NOT NULL,
                sha256      TEXT NOT NULL,
                nodes_json  TEXT NOT NULL,
                edges_json  TEXT NOT NULL,
                updated_at  INTEGER NOT NULL,
                PRIMARY KEY (file_path)
            );",
        )?;
        Ok(Self { conn })
    }

    /// Returns `(nodes_json, edges_json)` if the cached hash matches.
    pub fn get(&self, file_path: &str, content_hash: &str) -> Result<Option<(String, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT nodes_json, edges_json FROM cache \
             WHERE file_path = ?1 AND sha256 = ?2",
        )?;
        let result = stmt.query_row(params![file_path, content_hash], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        });
        match result {
            Ok(pair) => Ok(Some(pair)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn set(
        &self,
        file_path: &str,
        content_hash: &str,
        nodes_json: &str,
        edges_json: &str,
    ) -> Result<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        self.conn.execute(
            "INSERT OR REPLACE INTO cache \
             (file_path, sha256, nodes_json, edges_json, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![file_path, content_hash, nodes_json, edges_json, now],
        )?;
        Ok(())
    }
}

pub fn sha256_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}
