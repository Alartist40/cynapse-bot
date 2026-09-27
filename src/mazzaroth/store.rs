use crate::mazzaroth::node::{MemoryNode, MemoryTier, NodeTaxonomy};
use anyhow::{Context, Result};
use rusqlite::{params, Connection, OpenFlags};
use std::path::Path;
use std::sync::Mutex;
use std::time::Duration;

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS mazzaroth_nodes (
    id                 TEXT PRIMARY KEY,
    title              TEXT NOT NULL,
    content            TEXT NOT NULL DEFAULT '',
    tier               TEXT NOT NULL,
    taxonomy           TEXT NOT NULL,
    tags               TEXT NOT NULL DEFAULT '[]',
    links              TEXT NOT NULL DEFAULT '[]',
    access_count       INTEGER NOT NULL DEFAULT 1,
    last_accessed_secs INTEGER NOT NULL,
    created_at_secs    INTEGER NOT NULL,
    strength           REAL NOT NULL DEFAULT 0.5,
    decay_rate         REAL NOT NULL DEFAULT 0.1,
    x                  REAL NOT NULL DEFAULT 0.0,
    y                  REAL NOT NULL DEFAULT 0.0,
    z                  REAL NOT NULL DEFAULT 0.0,
    mass               REAL NOT NULL DEFAULT 1.0
);

CREATE INDEX IF NOT EXISTS idx_mazzaroth_tier ON mazzaroth_nodes(tier);
CREATE INDEX IF NOT EXISTS idx_mazzaroth_updated ON mazzaroth_nodes(last_accessed_secs DESC);

CREATE VIRTUAL TABLE IF NOT EXISTS mazzaroth_fts USING fts5(
    id UNINDEXED,
    title,
    content,
    tags,
    tokenize = 'porter unicode61'
);

CREATE TRIGGER IF NOT EXISTS mazzaroth_ai AFTER INSERT ON mazzaroth_nodes BEGIN
    INSERT INTO mazzaroth_fts(id, title, content, tags)
    VALUES (new.id, new.title, new.content, new.tags);
END;

CREATE TRIGGER IF NOT EXISTS mazzaroth_au AFTER UPDATE ON mazzaroth_nodes BEGIN
    DELETE FROM mazzaroth_fts WHERE id = old.id;
    INSERT INTO mazzaroth_fts(id, title, content, tags)
    VALUES (new.id, new.title, new.content, new.tags);
END;

CREATE TRIGGER IF NOT EXISTS mazzaroth_ad AFTER DELETE ON mazzaroth_nodes BEGIN
    DELETE FROM mazzaroth_fts WHERE id = old.id;
END;
"#;

pub struct MazzarothStore {
    conn: Mutex<Connection>,
}

impl MazzarothStore {
    pub fn open<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        let conn = Connection::open_with_flags(
            db_path.as_ref(),
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .context("open mazzaroth db")?;

        conn.busy_timeout(Duration::from_millis(5000))?;
        conn.pragma_update(None, "journal_mode", "WAL")?;

        conn.execute_batch(SCHEMA)?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn save_node(&self, node: &MemoryNode) -> Result<()> {
        let tags_json = serde_json::to_string(&node.tags)?;
        let links_json = serde_json::to_string(&node.links)?;
        let tier_str = format!("{:?}", node.tier);
        let tax_str = format!("{:?}", node.taxonomy);

        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"
            INSERT INTO mazzaroth_nodes (
                id, title, content, tier, taxonomy, tags, links,
                access_count, last_accessed_secs, created_at_secs,
                strength, decay_rate, x, y, z, mass
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                title = excluded.title,
                content = excluded.content,
                tier = excluded.tier,
                taxonomy = excluded.taxonomy,
                tags = excluded.tags,
                links = excluded.links,
                access_count = excluded.access_count,
                last_accessed_secs = excluded.last_accessed_secs,
                strength = excluded.strength,
                decay_rate = excluded.decay_rate,
                x = excluded.x,
                y = excluded.y,
                z = excluded.z,
                mass = excluded.mass
            "#,
            params![
                node.id,
                node.title,
                node.content,
                tier_str,
                tax_str,
                tags_json,
                links_json,
                node.access_count,
                node.last_accessed_secs,
                node.created_at_secs,
                node.strength,
                node.decay_rate,
                node.x,
                node.y,
                node.z,
                node.mass,
            ],
        )?;

        Ok(())
    }

    pub fn load_all(&self) -> Result<Vec<MemoryNode>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, title, content, tier, taxonomy, tags, links,
                   access_count, last_accessed_secs, created_at_secs,
                   strength, decay_rate, x, y, z, mass
            FROM mazzaroth_nodes
            ORDER BY last_accessed_secs DESC
            "#,
        )?;

        let rows = stmt.query_map([], |row| {
            let tier_str: String = row.get(3)?;
            let tax_str: String = row.get(4)?;
            let tags_raw: String = row.get(5)?;
            let links_raw: String = row.get(6)?;

            let tier = match tier_str.as_str() {
                "L1Working" => MemoryTier::L1Working,
                "L2Episodic" => MemoryTier::L2Episodic,
                "L3Semantic" => MemoryTier::L3Semantic,
                _ => MemoryTier::L4Core,
            };

            let taxonomy = match tax_str.as_str() {
                "Person" => NodeTaxonomy::Person,
                "Concept" => NodeTaxonomy::Concept,
                "Project" => NodeTaxonomy::Project,
                "Procedure" => NodeTaxonomy::Procedure,
                "Lesson" => NodeTaxonomy::Lesson,
                "Event" => NodeTaxonomy::Event,
                "AtomicFact" => NodeTaxonomy::AtomicFact,
                _ => NodeTaxonomy::Identity,
            };

            let tags = serde_json::from_str(&tags_raw).unwrap_or_default();
            let links = serde_json::from_str(&links_raw).unwrap_or_default();

            Ok(MemoryNode {
                id: row.get(0)?,
                title: row.get(1)?,
                content: row.get(2)?,
                tier,
                taxonomy,
                tags,
                links,
                access_count: row.get(7)?,
                last_accessed_secs: row.get(8)?,
                created_at_secs: row.get(9)?,
                strength: row.get(10)?,
                decay_rate: row.get(11)?,
                x: row.get(12)?,
                y: row.get(13)?,
                z: row.get(14)?,
                mass: row.get(15)?,
            })
        })?;

        let mut nodes = Vec::new();
        for row in rows {
            nodes.push(row?);
        }
        Ok(nodes)
    }

    pub fn search_fts_ids(&self, query: &str, limit: usize) -> Result<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        let safe_query = query.replace(['"', '*'], "");
        let formatted = format!("\"{}\"*", safe_query);

        let mut stmt = conn.prepare(
            r#"
            SELECT id FROM mazzaroth_fts
            WHERE mazzaroth_fts MATCH ?
            ORDER BY rank
            LIMIT ?
            "#,
        )?;

        let rows = stmt.query_map(params![formatted, limit as i64], |row| row.get(0))?;
        let mut ids = Vec::new();
        for r in rows {
            ids.push(r?);
        }
        Ok(ids)
    }
}
