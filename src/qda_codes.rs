// Qualitative Data Analysis codes/tags system (QualCoder-style)

use anyhow::Result;
use serde::{Serialize, Deserialize};
use std::path::Path;
use std::collections::HashMap;
use rusqlite::{Connection, params};
use crossterm::style::Color;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Code {
    pub id: String,
    pub name: String,
    pub description: String,
    pub color: CodeColor,
    pub parent_id: Option<String>,  // For hierarchical codes
    pub shortcut: Option<char>,     // Keyboard shortcut for quick coding
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl CodeColor {
    #[allow(dead_code)]
    pub fn to_crossterm_color(&self) -> Color {
        Color::Rgb {
            r: self.r,
            g: self.g,
            b: self.b,
        }
    }
}

pub struct CodeManager {
    conn: Connection,
    codes: HashMap<String, Code>,
}

impl CodeManager {
    pub fn new(notes_dir: &Path) -> Result<Self> {
        let db_path = notes_dir.join("codes.db");
        let conn = Connection::open(&db_path)?;

        // Create codes table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS codes (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                description TEXT,
                color_r INTEGER NOT NULL,
                color_g INTEGER NOT NULL,
                color_b INTEGER NOT NULL,
                parent_id TEXT,
                shortcut TEXT,
                FOREIGN KEY (parent_id) REFERENCES codes(id)
            )",
            [],
        )?;

        // Create code applications table (links codes to text segments)
        conn.execute(
            "CREATE TABLE IF NOT EXISTS code_applications (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                code_id TEXT NOT NULL,
                note_id TEXT NOT NULL,
                start_offset INTEGER NOT NULL,
                end_offset INTEGER NOT NULL,
                memo TEXT,
                created_at TEXT NOT NULL,
                FOREIGN KEY (code_id) REFERENCES codes(id)
            )",
            [],
        )?;

        // Load all codes into memory
        let codes = Self::load_codes(&conn)?;

        // Create default codes if none exist
        let mut manager = CodeManager { conn, codes };
        if manager.codes.is_empty() {
            manager.create_default_codes()?;
        }

        Ok(manager)
    }

    fn load_codes(conn: &Connection) -> Result<HashMap<String, Code>> {
        let mut stmt = conn.prepare(
            "SELECT id, name, description, color_r, color_g, color_b, parent_id, shortcut
             FROM codes"
        )?;

        let codes_iter = stmt.query_map([], |row| {
            let shortcut_str: Option<String> = row.get(7)?;
            Ok(Code {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                color: CodeColor {
                    r: row.get(3)?,
                    g: row.get(4)?,
                    b: row.get(5)?,
                },
                parent_id: row.get(6)?,
                shortcut: shortcut_str.and_then(|s| s.chars().next()),
            })
        })?;

        let mut codes = HashMap::new();
        for code in codes_iter {
            let code = code?;
            codes.insert(code.id.clone(), code);
        }

        Ok(codes)
    }

    fn create_default_codes(&mut self) -> Result<()> {
        // Create some default qualitative codes
        let default_codes = vec![
            ("theme", "Theme", "Major theme or pattern", CodeColor { r: 100, g: 150, b: 200 }, Some('t')),
            ("concept", "Concept", "Key concept or idea", CodeColor { r: 150, g: 200, b: 100 }, Some('c')),
            ("question", "Question", "Research question or inquiry", CodeColor { r: 200, g: 150, b: 100 }, Some('q')),
            ("insight", "Insight", "Important insight or finding", CodeColor { r: 200, g: 100, b: 150 }, Some('i')),
            ("todo", "To Do", "Action item or follow-up", CodeColor { r: 200, g: 100, b: 100 }, Some('d')),
            ("quote", "Quote", "Notable quotation", CodeColor { r: 150, g: 150, b: 200 }, Some('u')),
            ("reference", "Reference", "Citation or reference", CodeColor { r: 100, g: 200, b: 150 }, Some('r')),
            ("method", "Method", "Methodology or approach", CodeColor { r: 180, g: 180, b: 100 }, Some('m')),
        ];

        for (_id, name, desc, color, shortcut) in default_codes {
            self.create_code(name, desc, color, None, shortcut)?;
        }

        Ok(())
    }

    pub fn create_code(
        &mut self,
        name: &str,
        description: &str,
        color: CodeColor,
        parent_id: Option<String>,
        shortcut: Option<char>,
    ) -> Result<Code> {
        let id = self.generate_id(name);

        let code = Code {
            id: id.clone(),
            name: name.to_string(),
            description: description.to_string(),
            color,
            parent_id,
            shortcut,
        };

        self.conn.execute(
            "INSERT INTO codes (id, name, description, color_r, color_g, color_b, parent_id, shortcut)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                &code.id,
                &code.name,
                &code.description,
                code.color.r,
                code.color.g,
                code.color.b,
                &code.parent_id,
                code.shortcut.map(|c| c.to_string()),
            ],
        )?;

        self.codes.insert(id, code.clone());
        Ok(code)
    }

    pub fn update_code(&mut self, code: &Code) -> Result<()> {
        self.conn.execute(
            "UPDATE codes SET name = ?1, description = ?2, color_r = ?3, color_g = ?4,
             color_b = ?5, parent_id = ?6, shortcut = ?7 WHERE id = ?8",
            params![
                &code.name,
                &code.description,
                code.color.r,
                code.color.g,
                code.color.b,
                &code.parent_id,
                code.shortcut.map(|c| c.to_string()),
                &code.id,
            ],
        )?;

        self.codes.insert(code.id.clone(), code.clone());
        Ok(())
    }

    pub fn delete_code(&mut self, id: &str) -> Result<()> {
        // Delete all applications of this code first
        self.conn.execute(
            "DELETE FROM code_applications WHERE code_id = ?1",
            params![id],
        )?;

        // Delete the code
        self.conn.execute(
            "DELETE FROM codes WHERE id = ?1",
            params![id],
        )?;

        self.codes.remove(id);
        Ok(())
    }

    pub fn get_code(&self, id: &str) -> Option<&Code> {
        self.codes.get(id)
    }

    pub fn get_code_by_name(&self, name: &str) -> Option<&Code> {
        self.codes.values().find(|c| c.name == name)
    }

    pub fn get_code_by_shortcut(&self, shortcut: char) -> Option<&Code> {
        self.codes.values().find(|c| c.shortcut == Some(shortcut))
    }

    pub fn get_all_codes(&self) -> Vec<&Code> {
        self.codes.values().collect()
    }

    #[allow(dead_code)]
    pub fn apply_code(
        &mut self,
        code_id: &str,
        note_id: &str,
        start_offset: usize,
        end_offset: usize,
        memo: Option<String>,
    ) -> Result<()> {
        let created_at = chrono::Utc::now().to_rfc3339();

        self.conn.execute(
            "INSERT INTO code_applications (code_id, note_id, start_offset, end_offset, memo, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                code_id,
                note_id,
                start_offset as i64,
                end_offset as i64,
                memo,
                created_at,
            ],
        )?;

        Ok(())
    }

    #[allow(dead_code)]
    pub fn remove_code_application(
        &mut self,
        code_id: &str,
        note_id: &str,
        start_offset: usize,
    ) -> Result<()> {
        self.conn.execute(
            "DELETE FROM code_applications
             WHERE code_id = ?1 AND note_id = ?2 AND start_offset = ?3",
            params![code_id, note_id, start_offset as i64],
        )?;

        Ok(())
    }

    #[allow(dead_code)]
    pub fn get_code_applications(&self, note_id: &str) -> Result<Vec<CodeApplication>> {
        let mut stmt = self.conn.prepare(
            "SELECT code_id, start_offset, end_offset, memo, created_at
             FROM code_applications WHERE note_id = ?1
             ORDER BY start_offset"
        )?;

        let apps_iter = stmt.query_map(params![note_id], |row| {
            Ok(CodeApplication {
                code_id: row.get(0)?,
                note_id: note_id.to_string(),
                start_offset: row.get::<_, i64>(1)? as usize,
                end_offset: row.get::<_, i64>(2)? as usize,
                memo: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?;

        let mut apps = Vec::new();
        for app in apps_iter {
            apps.push(app?);
        }

        Ok(apps)
    }

    pub fn create_code_interactive(&mut self) -> Result<()> {
        // This would be called from UI to create a new code interactively
        // For now, just a placeholder
        println!("Creating new code...");
        // TODO: Implement interactive code creation
        Ok(())
    }

    fn generate_id(&self, name: &str) -> String {
        name.to_lowercase().replace(' ', "_")
    }

    pub fn export_codebook(&self) -> Result<String> {
        // Export all codes as JSON for sharing/backup
        let codes: Vec<&Code> = self.codes.values().collect();
        Ok(serde_json::to_string_pretty(&codes)?)
    }

    pub fn import_codebook(&mut self, json: &str) -> Result<()> {
        let codes: Vec<Code> = serde_json::from_str(json)?;

        for code in codes {
            // Try to insert, ignore if already exists
            self.conn.execute(
                "INSERT OR IGNORE INTO codes (id, name, description, color_r, color_g, color_b, parent_id, shortcut)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    &code.id,
                    &code.name,
                    &code.description,
                    code.color.r,
                    code.color.g,
                    code.color.b,
                    &code.parent_id,
                    code.shortcut.map(|c| c.to_string()),
                ],
            )?;

            self.codes.insert(code.id.clone(), code);
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct CodeApplication {
    pub code_id: String,
    pub note_id: String,
    pub start_offset: usize,
    pub end_offset: usize,
    pub memo: Option<String>,
    pub created_at: String,
}

