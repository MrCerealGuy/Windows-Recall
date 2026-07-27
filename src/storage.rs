use anyhow::{Context, Result};
use rusqlite::{Connection, params};
use std::path::Path;
use zstd::{Encoder, Decoder};

pub struct Screenshot {
    pub id: i64,
    pub timestamp: String,
    pub width: u32,
    pub height: u32,
    pub ocr_text: Option<String>,
    pub png_bytes: Option<Vec<u8>>,
}

pub struct Stats {
    pub total_count: i64,
    pub total_size_bytes: i64,
    pub first_screenshot: Option<String>,
    pub last_screenshot: Option<String>,
}

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path).context("Fehler beim Oeffnen der Datenbank")?;

        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;",
        )?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS screenshots (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp   TEXT NOT NULL,
                image_zst   BLOB NOT NULL,
                ocr_text    TEXT,
                width       INTEGER NOT NULL,
                height      INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_timestamp ON screenshots(timestamp);
            CREATE INDEX IF NOT EXISTS idx_ocr_text ON screenshots(ocr_text);",
        )?;

        Ok(Database { conn })
    }

    pub fn save_screenshot(&self, data: &crate::capture::CapturedScreen, ocr_text: &str) -> Result<i64> {
        let compressed = compress(&data.png_bytes)?;

        let timestamp = chrono::Utc::now().to_rfc3339();

        self.conn.execute(
            "INSERT INTO screenshots (timestamp, image_zst, ocr_text, width, height)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![timestamp, compressed, ocr_text, data.width, data.height],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    pub fn list_screenshots(&self, date: Option<&str>, limit: usize) -> Result<Vec<Screenshot>> {
        let mut stmt = if let Some(d) = date {
            let mut s = self.conn.prepare(
                "SELECT id, timestamp, width, height, ocr_text, image_zst
                 FROM screenshots
                 WHERE timestamp LIKE ?1
                 ORDER BY timestamp DESC
                 LIMIT ?2",
            )?;
            let pattern = format!("{}%", d);
            let rows: Vec<Screenshot> = s
                .query_map(params![pattern, limit as i64], |row| {
                    Ok(Screenshot {
                        id: row.get(0)?,
                        timestamp: row.get(1)?,
                        width: row.get(2)?,
                        height: row.get(3)?,
                        ocr_text: row.get(4)?,
                        png_bytes: None,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;
            return Ok(rows);
        } else {
            self.conn.prepare(
                "SELECT id, timestamp, width, height, ocr_text, image_zst
                 FROM screenshots
                 ORDER BY timestamp DESC
                 LIMIT ?1",
            )?
        };

        let rows: Vec<Screenshot> = stmt
            .query_map(params![limit as i64], |row| {
                Ok(Screenshot {
                    id: row.get(0)?,
                    timestamp: row.get(1)?,
                    width: row.get(2)?,
                    height: row.get(3)?,
                    ocr_text: row.get(4)?,
                    png_bytes: None,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(rows)
    }

    pub fn get_screenshot(&self, id: i64) -> Result<Option<Screenshot>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, timestamp, width, height, ocr_text, image_zst
             FROM screenshots WHERE id = ?1",
        )?;

        let mut rows = stmt.query_map(params![id], |row| {
            let zst: Vec<u8> = row.get(5)?;
            let png = decompress(&zst).map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
            Ok(Screenshot {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                width: row.get(2)?,
                height: row.get(3)?,
                ocr_text: row.get(4)?,
                png_bytes: Some(png),
            })
        })?;

        match rows.next() {
            Some(r) => Ok(Some(r?)),
            None => Ok(None),
        }
    }

    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<Screenshot>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, timestamp, width, height, ocr_text
             FROM screenshots
             WHERE ocr_text LIKE ?1
             ORDER BY timestamp DESC
             LIMIT ?2",
        )?;

        let pattern = format!("%{}%", query);
        let rows: Vec<Screenshot> = stmt
            .query_map(params![pattern, limit as i64], |row| {
                Ok(Screenshot {
                    id: row.get(0)?,
                    timestamp: row.get(1)?,
                    width: row.get(2)?,
                    height: row.get(3)?,
                    ocr_text: row.get(4)?,
                    png_bytes: None,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(rows)
    }

    pub fn stats(&self) -> Result<Stats> {
        let total_count: i64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM screenshots", [], |r| r.get(0))?;

        let total_size_bytes: i64 = self.conn.query_row(
            "SELECT COALESCE(SUM(LENGTH(image_zst)), 0) FROM screenshots",
            [],
            |r| r.get(0),
        )?;

        let first_screenshot: Option<String> = self
            .conn
            .query_row(
                "SELECT MIN(timestamp) FROM screenshots",
                [],
                |r| r.get(0),
            )
            .ok();

        let last_screenshot: Option<String> = self
            .conn
            .query_row(
                "SELECT MAX(timestamp) FROM screenshots",
                [],
                |r| r.get(0),
            )
            .ok();

        Ok(Stats {
            total_count,
            total_size_bytes,
            first_screenshot,
            last_screenshot,
        })
    }

    pub fn all_screenshots_for_export(&self) -> Result<Vec<Screenshot>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, timestamp, width, height, ocr_text, image_zst
             FROM screenshots ORDER BY timestamp ASC",
        )?;

        let rows: Vec<Screenshot> = stmt
            .query_map([], |row| {
                let zst: Vec<u8> = row.get(5)?;
                let png = decompress(&zst).map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
                Ok(Screenshot {
                    id: row.get(0)?,
                    timestamp: row.get(1)?,
                    width: row.get(2)?,
                    height: row.get(3)?,
                    ocr_text: row.get(4)?,
                    png_bytes: Some(png),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(rows)
    }

    pub fn cleanup(&self, max_age_days: u64) -> Result<u64> {
        let cutoff = (chrono::Utc::now() - chrono::Duration::days(max_age_days as i64))
            .to_rfc3339();

        let deleted = self.conn.execute(
            "DELETE FROM screenshots WHERE timestamp < ?1",
            params![cutoff],
        )?;

        self.conn.execute("VACUUM", [])?;

        Ok(deleted as u64)
    }
}

fn compress(data: &[u8]) -> Result<Vec<u8>> {
    let mut encoder = Encoder::new(Vec::new(), 3).context("Fehler beim Erstellen des Zstd-Encoders")?;
    std::io::Write::write_all(&mut encoder, data)?;
    encoder.finish().context("Fehler bei der Zstd-Komprimierung")
}

fn decompress(data: &[u8]) -> Result<Vec<u8>> {
    let mut decoder = Decoder::new(data).context("Fehler beim Erstellen des Zstd-Decoders")?;
    let mut output = Vec::new();
    std::io::Read::read_to_end(&mut decoder, &mut output)?;
    Ok(output)
}
