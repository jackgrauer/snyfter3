// Text editor functionality using Helix-core

use anyhow::Result;
use helix_core::{
    Rope, Selection,
    Position,
    graphemes::{next_grapheme_boundary, prev_grapheme_boundary},
};
use crossterm::event::{KeyCode, KeyModifiers};

pub struct TextEditor {
    pub rope: Rope,
    pub selection: Selection,
    pub cursor_pos: Position,
    pub viewport_offset: usize,  // First visible line
}

impl TextEditor {
    pub fn new() -> Self {
        Self {
            rope: Rope::new(),
            selection: Selection::single(0, 0),
            cursor_pos: Position::new(0, 0),
            viewport_offset: 0,
        }
    }

    pub fn from_text(text: &str) -> Self {
        let rope = Rope::from_str(text);
        Self {
            rope,
            selection: Selection::single(0, 0),
            cursor_pos: Position::new(0, 0),
            viewport_offset: 0,
        }
    }

    pub fn handle_key(&mut self, code: KeyCode, modifiers: KeyModifiers) -> Result<bool> {
        let mut modified = false;

        match (code, modifiers) {
            // Basic movement
            (KeyCode::Left, KeyModifiers::NONE) => self.move_cursor_left(),
            (KeyCode::Right, KeyModifiers::NONE) => self.move_cursor_right(),
            (KeyCode::Up, KeyModifiers::NONE) => self.move_cursor_up(),
            (KeyCode::Down, KeyModifiers::NONE) => self.move_cursor_down(),

            // Word movement
            (KeyCode::Left, mods) if mods.contains(KeyModifiers::CONTROL) => self.move_word_left(),
            (KeyCode::Right, mods) if mods.contains(KeyModifiers::CONTROL) => self.move_word_right(),

            // Line movement
            (KeyCode::Home, _) => self.move_to_line_start(),
            (KeyCode::End, _) => self.move_to_line_end(),

            // Page movement
            (KeyCode::PageUp, _) => self.page_up(),
            (KeyCode::PageDown, _) => self.page_down(),

            // Text insertion
            (KeyCode::Char(c), KeyModifiers::NONE) |
            (KeyCode::Char(c), KeyModifiers::SHIFT) => {
                self.insert_char(c);
                modified = true;
            }

            // Special characters
            (KeyCode::Enter, _) => {
                self.insert_newline();
                modified = true;
            }
            (KeyCode::Tab, _) => {
                self.insert_char('\t');
                modified = true;
            }

            // Deletion
            (KeyCode::Backspace, _) => {
                if self.delete_char_backward() {
                    modified = true;
                }
            }
            (KeyCode::Delete, _) => {
                if self.delete_char_forward() {
                    modified = true;
                }
            }

            // Undo/Redo (simplified - would need history tracking)
            (KeyCode::Char('z'), mods) if mods.contains(KeyModifiers::CONTROL) => {
                // TODO: Implement undo
            }
            (KeyCode::Char('y'), mods) if mods.contains(KeyModifiers::CONTROL) => {
                // TODO: Implement redo
            }

            // Selection (simplified)
            (KeyCode::Char('a'), mods) if mods.contains(KeyModifiers::CONTROL) => {
                self.select_all();
            }

            _ => {}
        }

        Ok(modified)
    }

    fn move_cursor_left(&mut self) {
        let text = self.rope.slice(..);
        let range = self.selection.primary();
            let pos = range.cursor(text);
        if pos > 0 {
            let new_pos = prev_grapheme_boundary(text, pos);
            self.selection = Selection::single(new_pos, new_pos);
            self.update_cursor_position();
        }
    }

    fn move_cursor_right(&mut self) {
        let text = self.rope.slice(..);
        let range = self.selection.primary();
            let pos = range.cursor(text);
        let new_pos = next_grapheme_boundary(text, pos);
        if new_pos < text.len_chars() {
            self.selection = Selection::single(new_pos, new_pos);
            self.update_cursor_position();
        }
    }

    fn move_cursor_up(&mut self) {
        let text = self.rope.slice(..);
        let range = self.selection.primary();
            let pos = range.cursor(text);
        let (row, col) = self.pos_to_coords(pos);
        if row > 0 {
            let new_row = row - 1;
            let new_pos = self.coords_to_pos(new_row, col);
            self.selection = Selection::single(new_pos, new_pos);
            self.update_cursor_position();
        }
    }

    fn move_cursor_down(&mut self) {
        let text = self.rope.slice(..);
        let range = self.selection.primary();
            let pos = range.cursor(text);
        let (row, col) = self.pos_to_coords(pos);
        let line_count = self.rope.len_lines();
        if row < line_count - 1 {
            let new_row = row + 1;
            let new_pos = self.coords_to_pos(new_row, col);
            self.selection = Selection::single(new_pos, new_pos);
            self.update_cursor_position();
        }
    }

    fn move_word_left(&mut self) {
        let text = self.rope.slice(..);
        let range = self.selection.primary();
            let pos = range.cursor(text);
        let new_pos = self.prev_word_boundary(pos);
        self.selection = Selection::single(new_pos, new_pos);
        self.update_cursor_position();
    }

    fn move_word_right(&mut self) {
        let text = self.rope.slice(..);
        let range = self.selection.primary();
            let pos = range.cursor(text);
        let new_pos = self.next_word_boundary(pos);
        self.selection = Selection::single(new_pos, new_pos);
        self.update_cursor_position();
    }

    fn move_to_line_start(&mut self) {
        let text = self.rope.slice(..);
        let range = self.selection.primary();
            let pos = range.cursor(text);
        let line_start = text.line_to_char(text.char_to_line(pos));
        self.selection = Selection::single(line_start, line_start);
        self.update_cursor_position();
    }

    fn move_to_line_end(&mut self) {
        let text = self.rope.slice(..);
        let range = self.selection.primary();
            let pos = range.cursor(text);
        let line = text.char_to_line(pos);
        let line_end = if line < text.len_lines() - 1 {
            text.line_to_char(line + 1) - 1
        } else {
            text.len_chars()
        };
        self.selection = Selection::single(line_end, line_end);
        self.update_cursor_position();
    }

    fn page_up(&mut self) {
        // Move up by viewport height
        for _ in 0..20 {
            self.move_cursor_up();
        }
    }

    fn page_down(&mut self) {
        // Move down by viewport height
        for _ in 0..20 {
            self.move_cursor_down();
        }
    }

    fn insert_char(&mut self, ch: char) {
        let text = self.rope.slice(..);
        let range = self.selection.primary();
        let pos = range.cursor(text);

        // Simple insertion without Transaction API
        let mut new_text = self.rope.to_string();
        new_text.insert(pos, ch);
        self.rope = Rope::from_str(&new_text);

        // Move cursor forward
        let new_pos = pos + 1;
        self.selection = Selection::single(new_pos, new_pos);
        self.update_cursor_position();
    }

    fn insert_newline(&mut self) {
        self.insert_char('\n');
    }

    fn delete_char_backward(&mut self) -> bool {
        let text = self.rope.slice(..);
        let range = self.selection.primary();
            let pos = range.cursor(text);
        if pos > 0 {
            let start = prev_grapheme_boundary(text, pos);
            // Simple deletion without Transaction API
            let mut new_text = self.rope.to_string();
            new_text.drain(start..pos);
            self.rope = Rope::from_str(&new_text);
            self.selection = Selection::single(start, start);
            self.update_cursor_position();
            return true;
        }
        false
    }

    fn delete_char_forward(&mut self) -> bool {
        let text = self.rope.slice(..);
        let range = self.selection.primary();
            let pos = range.cursor(text);
        if pos < text.len_chars() {
            let end = next_grapheme_boundary(text, pos);
            // Simple deletion without Transaction API
            let mut new_text = self.rope.to_string();
            new_text.drain(pos..end);
            self.rope = Rope::from_str(&new_text);
            self.selection = Selection::single(pos, pos);
            self.update_cursor_position();
            return true;
        }
        false
    }

    fn select_all(&mut self) {
        let len = self.rope.len_chars();
        self.selection = Selection::single(0, len);
        self.update_cursor_position();
    }

    fn update_cursor_position(&mut self) {
        let text = self.rope.slice(..);
        let range = self.selection.primary();
        let pos = range.cursor(text);
        let (row, col) = self.pos_to_coords(pos);
        self.cursor_pos = Position::new(row, col);

        // Update viewport if cursor moved outside
        if row < self.viewport_offset {
            self.viewport_offset = row;
        } else if row >= self.viewport_offset + 20 {
            self.viewport_offset = row.saturating_sub(19);
        }
    }

    fn pos_to_coords(&self, pos: usize) -> (usize, usize) {
        let text = self.rope.slice(..);
        let line = text.char_to_line(pos);
        let line_start = text.line_to_char(line);
        let col = pos - line_start;
        (line, col)
    }

    fn coords_to_pos(&self, row: usize, col: usize) -> usize {
        let text = self.rope.slice(..);
        let line_start = text.line_to_char(row);
        let line = text.line(row);
        let max_col = line.len_chars().saturating_sub(1);
        line_start + col.min(max_col)
    }

    fn prev_word_boundary(&self, pos: usize) -> usize {
        let text = self.rope.slice(..);
        let mut new_pos = pos;

        // Skip whitespace backwards
        while new_pos > 0 {
            let prev_pos = prev_grapheme_boundary(text, new_pos);
            let ch = text.char(prev_pos);
            if !ch.is_whitespace() {
                break;
            }
            new_pos = prev_pos;
        }

        // Skip word characters backwards
        while new_pos > 0 {
            let prev_pos = prev_grapheme_boundary(text, new_pos);
            let ch = text.char(prev_pos);
            if !ch.is_alphanumeric() && ch != '_' {
                break;
            }
            new_pos = prev_pos;
        }

        new_pos
    }

    fn next_word_boundary(&self, pos: usize) -> usize {
        let text = self.rope.slice(..);
        let mut new_pos = pos;
        let len = text.len_chars();

        // Skip current word
        while new_pos < len {
            let ch = text.char(new_pos);
            if !ch.is_alphanumeric() && ch != '_' {
                break;
            }
            new_pos = next_grapheme_boundary(text, new_pos);
        }

        // Skip whitespace
        while new_pos < len {
            let ch = text.char(new_pos);
            if !ch.is_whitespace() {
                break;
            }
            new_pos = next_grapheme_boundary(text, new_pos);
        }

        new_pos
    }

    pub fn get_text(&self) -> String {
        self.rope.to_string()
    }

    pub fn set_text(&mut self, text: &str) {
        self.rope = Rope::from_str(text);
        self.selection = Selection::single(0, 0);
        self.cursor_pos = Position::new(0, 0);
        self.viewport_offset = 0;
    }

    pub fn get_visible_lines(&self, height: usize) -> Vec<String> {
        let mut lines = Vec::new();
        let text = self.rope.slice(..);
        let total_lines = text.len_lines();

        for i in self.viewport_offset..self.viewport_offset + height {
            if i >= total_lines {
                break;
            }
            let line = text.line(i);
            lines.push(line.to_string());
        }

        lines
    }

    pub fn get_cursor_screen_position(&self) -> (usize, usize) {
        let screen_row = self.cursor_pos.row.saturating_sub(self.viewport_offset);
        (screen_row, self.cursor_pos.col)
    }
}