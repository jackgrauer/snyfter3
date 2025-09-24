// Note storage and management using SQLite with FTS5

use anyhow::Result;
use rusqlite::{Connection, params, OptionalExtension};
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use std::path::Path;
use sha2::{Sha256, Digest};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub id: String,
    pub title: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub tags: Vec<String>,
    pub codes: Vec<CodedSegment>,  // QualCoder-style coded segments
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodedSegment {
    pub code_id: String,
    pub start_offset: usize,
    pub end_offset: usize,
    pub memo: Option<String>,
}

pub struct NoteStore {
    conn: Connection,
}

impl NoteStore {
    pub fn new(notes_dir: &Path) -> Result<Self> {
        let db_path = notes_dir.join("notes.db");
        let conn = Connection::open(&db_path)?;

        // Create tables if they don't exist
        conn.execute(
            "CREATE TABLE IF NOT EXISTS notes (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                tags TEXT,
                codes TEXT
            )",
            [],
        )?;

        // Note: FTS5 removed since rusqlite doesn't support it directly
        // We'll rely on Tantivy for full-text search instead

        Ok(NoteStore {
            conn,
        })
    }

    pub fn create_note(&mut self, title: &str, content: &str) -> Result<Note> {
        let now = Utc::now();
        let id = self.generate_id(title, &now);

        let note = Note {
            id: id.clone(),
            title: title.to_string(),
            content: content.to_string(),
            created_at: now,
            updated_at: now,
            tags: Vec::new(),
            codes: Vec::new(),
        };

        self.conn.execute(
            "INSERT INTO notes (id, title, content, created_at, updated_at, tags, codes)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                &note.id,
                &note.title,
                &note.content,
                &note.created_at.to_rfc3339(),
                &note.updated_at.to_rfc3339(),
                serde_json::to_string(&note.tags)?,
                serde_json::to_string(&note.codes)?,
            ],
        )?;

        Ok(note)
    }

    pub fn update_note(&mut self, note: &Note) -> Result<()> {
        let updated = Note {
            updated_at: Utc::now(),
            ..note.clone()
        };

        self.conn.execute(
            "UPDATE notes SET title = ?1, content = ?2, updated_at = ?3, tags = ?4, codes = ?5
             WHERE id = ?6",
            params![
                &updated.title,
                &updated.content,
                &updated.updated_at.to_rfc3339(),
                serde_json::to_string(&updated.tags)?,
                serde_json::to_string(&updated.codes)?,
                &updated.id,
            ],
        )?;

        Ok(())
    }

    pub fn delete_note(&mut self, id: &str) -> Result<()> {
        self.conn.execute("DELETE FROM notes WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn get_note(&mut self, id: &str) -> Result<Option<Note>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, content, created_at, updated_at, tags, codes
             FROM notes WHERE id = ?1"
        )?;

        let note = stmt.query_row(params![id], |row| {
            let tags_json: String = row.get(5)?;
            let codes_json: String = row.get(6)?;

            Ok(Note {
                id: row.get(0)?,
                title: row.get(1)?,
                content: row.get(2)?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(3)?)
                    .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                        3, rusqlite::types::Type::Text, Box::new(e)
                    ))?.with_timezone(&Utc),
                updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                    .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                        4, rusqlite::types::Type::Text, Box::new(e)
                    ))?.with_timezone(&Utc),
                tags: serde_json::from_str(&tags_json)
                    .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                        5, rusqlite::types::Type::Text, Box::new(e)
                    ))?,
                codes: serde_json::from_str(&codes_json)
                    .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                        6, rusqlite::types::Type::Text, Box::new(e)
                    ))?,
            })
        }).optional()?;

        Ok(note)
    }

    pub fn get_note_by_index(&self, index: usize) -> Result<Option<Note>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, content, created_at, updated_at, tags, codes
             FROM notes ORDER BY updated_at DESC LIMIT 1 OFFSET ?1"
        )?;

        let note = stmt.query_row(params![index], |row| {
            let tags_json: String = row.get(5)?;
            let codes_json: String = row.get(6)?;

            Ok(Note {
                id: row.get(0)?,
                title: row.get(1)?,
                content: row.get(2)?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(3)?)
                    .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                        3, rusqlite::types::Type::Text, Box::new(e)
                    ))?.with_timezone(&Utc),
                updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                    .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                        4, rusqlite::types::Type::Text, Box::new(e)
                    ))?.with_timezone(&Utc),
                tags: serde_json::from_str(&tags_json)
                    .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                        5, rusqlite::types::Type::Text, Box::new(e)
                    ))?,
                codes: serde_json::from_str(&codes_json)
                    .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                        6, rusqlite::types::Type::Text, Box::new(e)
                    ))?,
            })
        }).optional()?;

        Ok(note)
    }

    #[allow(dead_code)]
    pub fn search_notes(&self, query: &str) -> Result<Vec<Note>> {
        // Basic search without FTS5 - Tantivy will handle full-text search
        let query_pattern = format!("%{}%", query);
        let mut stmt = self.conn.prepare(
            "SELECT id, title, content, created_at, updated_at, tags, codes
             FROM notes
             WHERE title LIKE ?1 OR content LIKE ?1 OR tags LIKE ?1
             ORDER BY updated_at DESC"
        )?;

        let notes_iter = stmt.query_map(params![query_pattern], |row| {
            let tags_json: String = row.get(5)?;
            let codes_json: String = row.get(6)?;

            Ok(Note {
                id: row.get(0)?,
                title: row.get(1)?,
                content: row.get(2)?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(3)?)
                    .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                        3, rusqlite::types::Type::Text, Box::new(e)
                    ))?.with_timezone(&Utc),
                updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                    .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                        4, rusqlite::types::Type::Text, Box::new(e)
                    ))?.with_timezone(&Utc),
                tags: serde_json::from_str(&tags_json)
                    .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                        5, rusqlite::types::Type::Text, Box::new(e)
                    ))?,
                codes: serde_json::from_str(&codes_json)
                    .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                        6, rusqlite::types::Type::Text, Box::new(e)
                    ))?,
            })
        })?;

        let mut notes = Vec::new();
        for note in notes_iter {
            notes.push(note?);
        }

        Ok(notes)
    }

    pub fn get_all_notes(&self) -> Result<Vec<Note>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, content, created_at, updated_at, tags, codes
             FROM notes ORDER BY updated_at DESC"
        )?;

        let notes_iter = stmt.query_map([], |row| {
            let tags_json: String = row.get(5)?;
            let codes_json: String = row.get(6)?;

            Ok(Note {
                id: row.get(0)?,
                title: row.get(1)?,
                content: row.get(2)?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(3)?)
                    .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                        3, rusqlite::types::Type::Text, Box::new(e)
                    ))?.with_timezone(&Utc),
                updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                    .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                        4, rusqlite::types::Type::Text, Box::new(e)
                    ))?.with_timezone(&Utc),
                tags: serde_json::from_str(&tags_json)
                    .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                        5, rusqlite::types::Type::Text, Box::new(e)
                    ))?,
                codes: serde_json::from_str(&codes_json)
                    .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                        6, rusqlite::types::Type::Text, Box::new(e)
                    ))?,
            })
        })?;

        let mut notes = Vec::new();
        for note in notes_iter {
            notes.push(note?);
        }

        Ok(notes)
    }

    pub fn get_note_count(&self) -> usize {
        self.conn.query_row(
            "SELECT COUNT(*) FROM notes",
            [],
            |row| row.get(0)
        ).unwrap_or(0)
    }

    fn generate_id(&self, title: &str, created_at: &DateTime<Utc>) -> String {
        let mut hasher = Sha256::new();
        hasher.update(title.as_bytes());
        hasher.update(created_at.to_rfc3339().as_bytes());
        let result = hasher.finalize();
        format!("{:x}", result)[..12].to_string()
    }

    #[allow(dead_code)]
    pub fn add_code_to_note(&mut self, note_id: &str, segment: CodedSegment) -> Result<()> {
        if let Some(mut note) = self.get_note(note_id)? {
            note.codes.push(segment);
            self.update_note(&note)?;
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub fn remove_code_from_note(&mut self, note_id: &str, code_id: &str, start: usize) -> Result<()> {
        if let Some(mut note) = self.get_note(note_id)? {
            note.codes.retain(|c| !(c.code_id == code_id && c.start_offset == start));
            self.update_note(&note)?;
        }
        Ok(())
    }
}