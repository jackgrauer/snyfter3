// Text editor functionality using Helix-core

use anyhow::Result;
use helix_core::{
    Rope, Selection,
    Position,
    graphemes::{next_grapheme_boundary, prev_grapheme_boundary},
    movement,
};
use crossterm::event::{KeyCode, KeyModifiers};
use std::process::Command;
use crate::block_selection::BlockSelection;

pub struct TextEditor {
    pub rope: Rope,
    pub selection: Selection,
    pub cursor_pos: Position,
    pub scroll_x: usize,  // Horizontal scroll offset
    pub scroll_y: usize,  // Vertical scroll offset (replaces viewport_offset)
    pub selection_anchor: Option<usize>,  // For shift-selection
    pub virtual_cursor_col: Option<usize>,  // Virtual column for up/down movement (like chonker7)
    pub block_selection: Option<BlockSelection>,  // For rectangular selection
    pub potential_block_start: Option<(usize, usize)>,  // For tracking mouse drag start
}

impl TextEditor {
    pub fn new() -> Self {
        Self {
            rope: Rope::new(),
            selection: Selection::single(0, 0),
            cursor_pos: Position::new(0, 0),
            scroll_x: 0,
            scroll_y: 0,
            selection_anchor: None,
            virtual_cursor_col: None,
            block_selection: None,
            potential_block_start: None,
        }
    }

    #[allow(dead_code)]
    pub fn from_text(text: &str) -> Self {
        let rope = Rope::from_str(text);
        Self {
            rope,
            selection: Selection::single(0, 0),
            cursor_pos: Position::new(0, 0),
            scroll_x: 0,
            scroll_y: 0,
            selection_anchor: None,
            virtual_cursor_col: None,
            block_selection: None,
            potential_block_start: None,
        }
    }

    pub fn handle_key(&mut self, code: KeyCode, modifiers: KeyModifiers) -> Result<bool> {
        self.handle_key_with_acceleration(code, modifiers, 1)
    }

    pub fn handle_key_with_acceleration(&mut self, code: KeyCode, modifiers: KeyModifiers, acceleration: usize) -> Result<bool> {
        let mut modified = false;

        match (code, modifiers) {
            // Basic movement with acceleration
            (KeyCode::Left, KeyModifiers::NONE) => {
                for _ in 0..acceleration {
                    self.move_cursor_left();
                }
                self.selection_anchor = None;
                self.block_selection = None;  // Clear block selection on regular movement
                self.potential_block_start = None;  // Clear potential block start
            }
            (KeyCode::Right, KeyModifiers::NONE) => {
                for _ in 0..acceleration {
                    self.move_cursor_right();
                }
                self.selection_anchor = None;
                self.block_selection = None;  // Clear block selection on regular movement
            }
            (KeyCode::Up, KeyModifiers::NONE) => {
                for _ in 0..acceleration {
                    self.move_cursor_up();
                }
                self.selection_anchor = None;
                self.block_selection = None;  // Clear block selection on regular movement
            }
            (KeyCode::Down, KeyModifiers::NONE) => {
                for _ in 0..acceleration {
                    self.move_cursor_down();
                }
                self.selection_anchor = None;
                self.block_selection = None;  // Clear block selection on regular movement
            }

            // Shift selection
            (KeyCode::Left, KeyModifiers::SHIFT) => {
                self.extend_selection_left();
                self.block_selection = None;  // Clear block selection
            }
            (KeyCode::Right, KeyModifiers::SHIFT) => {
                self.extend_selection_right();
                self.block_selection = None;  // Clear block selection
            }
            (KeyCode::Up, KeyModifiers::SHIFT) => {
                self.extend_selection_up();
                self.block_selection = None;  // Clear block selection
            }
            (KeyCode::Down, KeyModifiers::SHIFT) => {
                self.extend_selection_down();
                self.block_selection = None;  // Clear block selection
            }

            // Alt+Shift for block selection
            (KeyCode::Left, mods) if mods.contains(KeyModifiers::ALT) && mods.contains(KeyModifiers::SHIFT) => {
                self.extend_block_selection_left();
            }
            (KeyCode::Right, mods) if mods.contains(KeyModifiers::ALT) && mods.contains(KeyModifiers::SHIFT) => {
                self.extend_block_selection_right();
            }
            (KeyCode::Up, mods) if mods.contains(KeyModifiers::ALT) && mods.contains(KeyModifiers::SHIFT) => {
                self.extend_block_selection_up();
            }
            (KeyCode::Down, mods) if mods.contains(KeyModifiers::ALT) && mods.contains(KeyModifiers::SHIFT) => {
                self.extend_block_selection_down();
            }

            // Word movement
            (KeyCode::Left, mods) if mods.contains(KeyModifiers::CONTROL) => {
                self.move_word_left();
                if !mods.contains(KeyModifiers::SHIFT) {
                    self.selection_anchor = None;
                }
            }
            (KeyCode::Right, mods) if mods.contains(KeyModifiers::CONTROL) => {
                self.move_word_right();
                if !mods.contains(KeyModifiers::SHIFT) {
                    self.selection_anchor = None;
                }
            }

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

            // Select all
            (KeyCode::Char('a'), mods) if mods.contains(KeyModifiers::CONTROL) => {
                self.select_all();
            }

            // Cut
            (KeyCode::Char('x'), mods) if mods.contains(KeyModifiers::CONTROL) => {
                if self.cut_selection()? {
                    modified = true;
                }
            }

            // Copy
            (KeyCode::Char('c'), mods) if mods.contains(KeyModifiers::CONTROL) => {
                self.copy_selection()?;
            }

            // Paste
            (KeyCode::Char('v'), mods) if mods.contains(KeyModifiers::CONTROL) => {
                if self.paste()? {
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

            _ => {}
        }

        Ok(modified)
    }

    fn move_cursor_left(&mut self) {
        // Check if we have a virtual cursor position
        if let Some(virtual_col) = self.virtual_cursor_col {
            if virtual_col > 0 {
                // Just move the virtual cursor left
                self.virtual_cursor_col = Some(virtual_col - 1);

                // Update the cursor position to reflect the virtual position
                let text = self.rope.slice(..);
                let pos = self.selection.primary().head;
                let line = text.char_to_line(pos);
                self.cursor_pos = Position::new(line, virtual_col - 1);

                // Check if we're now within the actual line
                let line_start = text.line_to_char(line);
                let line_slice = text.line(line);
                let line_len = line_slice.len_chars();
                let effective_len = if line_len > 0 && line_slice.char(line_len - 1) == '\n' {
                    line_len.saturating_sub(1)
                } else {
                    line_len
                };

                if virtual_col - 1 <= effective_len {
                    // We're back in the actual text, update selection
                    let new_pos = line_start + (virtual_col - 1).min(effective_len);
                    self.selection = Selection::point(new_pos);
                }
                return;
            }
        }

        // Normal movement within text - stop at line boundary
        let text = self.rope.slice(..);
        let pos = self.selection.primary().head;
        let line = text.char_to_line(pos);
        let line_start = text.line_to_char(line);

        // Only move left if we're not at the beginning of the line
        if pos > line_start {
            let new_pos = prev_grapheme_boundary(text, pos);
            // Make sure we don't go past the line start
            let new_pos = new_pos.max(line_start);
            self.selection = Selection::point(new_pos);
            self.update_cursor_position();
            // Clear virtual column when moving within actual text
            self.virtual_cursor_col = None;
        }
    }

    fn move_cursor_right(&mut self) {
        let text = self.rope.slice(..);
        let pos = self.selection.primary().head;
        let line = text.char_to_line(pos);
        let line_start = text.line_to_char(line);
        let line_slice = text.line(line);
        let line_len = line_slice.len_chars();

        // Get effective line length (excluding newline)
        let effective_len = if line_len > 0 && line_slice.char(line_len - 1) == '\n' {
            line_len.saturating_sub(1)
        } else {
            line_len
        };

        // Current column position
        let current_col = if let Some(vc) = self.virtual_cursor_col {
            vc
        } else {
            pos - line_start
        };

        // Always allow moving right, even into virtual space
        let new_col = current_col + 1;
        self.virtual_cursor_col = Some(new_col);

        // Update cursor position for rendering
        self.cursor_pos = Position::new(line, new_col);

        // If we're still within the actual text, update selection
        if new_col <= effective_len {
            let new_pos = line_start + new_col;
            if new_pos <= text.len_chars() {
                self.selection = Selection::point(new_pos);
            }
        }
        // If we're in virtual space, keep selection at end of line
        else {
            let line_end = line_start + effective_len;
            self.selection = Selection::point(line_end);
        }
    }

    fn move_cursor_up(&mut self) {
        let text = self.rope.slice(..);
        let pos = self.selection.primary().head;
        let line = text.char_to_line(pos);

        if line > 0 {
            // Preserve virtual column if set, otherwise calculate from current position
            let virtual_col = if let Some(vc) = self.virtual_cursor_col {
                vc
            } else {
                let line_start = text.line_to_char(line);
                pos - line_start
            };

            // Save the virtual column for future up/down movements
            self.virtual_cursor_col = Some(virtual_col);

            let new_line = line - 1;
            let new_line_start = text.line_to_char(new_line);
            let new_line_slice = text.line(new_line);
            let new_line_len = new_line_slice.len_chars();

            // Clamp to line length (excluding newline if present)
            let effective_len = if new_line_len > 0 && new_line_slice.char(new_line_len - 1) == '\n' {
                new_line_len.saturating_sub(1)
            } else {
                new_line_len
            };

            // Position cursor at virtual column, but clamp to actual line length for the selection
            let new_col = virtual_col.min(effective_len);
            let new_pos = new_line_start + new_col;

            self.selection = Selection::point(new_pos);
            self.update_cursor_position();
        }
    }

    fn move_cursor_down(&mut self) {
        let text = self.rope.slice(..);
        let pos = self.selection.primary().head;
        let line = text.char_to_line(pos);
        let max_line = text.len_lines().saturating_sub(1);

        if line < max_line {
            // Preserve virtual column if set, otherwise calculate from current position
            let virtual_col = if let Some(vc) = self.virtual_cursor_col {
                vc
            } else {
                let line_start = text.line_to_char(line);
                pos - line_start
            };

            // Save the virtual column for future up/down movements
            self.virtual_cursor_col = Some(virtual_col);

            let new_line = line + 1;
            let new_line_start = text.line_to_char(new_line);
            let new_line_slice = text.line(new_line);
            let new_line_len = new_line_slice.len_chars();

            // Clamp to line length (excluding newline if present)
            let effective_len = if new_line_len > 0 && new_line_slice.char(new_line_len - 1) == '\n' {
                new_line_len.saturating_sub(1)
            } else {
                new_line_len
            };

            // Position cursor at virtual column, but clamp to actual line length for the selection
            let new_col = virtual_col.min(effective_len);
            let new_pos = new_line_start + new_col;

            self.selection = Selection::point(new_pos);
            self.update_cursor_position();
        }
    }

    fn move_word_left(&mut self) {
        let text = self.rope.slice(..);
        let range = self.selection.primary();
        let pos = range.head;
        let line = text.char_to_line(pos);
        let line_start = text.line_to_char(line);

        // Use helix-core's movement function but clamp to line start
        let new_range = movement::move_prev_word_start(text, range, 1);
        let new_pos = new_range.head.max(line_start);

        self.selection = Selection::point(new_pos);
        self.update_cursor_position();
    }

    fn move_word_right(&mut self) {
        let text = self.rope.slice(..);
        let range = self.selection.primary();

        // Use helix-core's movement function
        let new_range = movement::move_next_word_end(text, range, 1);

        self.selection = Selection::single(new_range.anchor, new_range.head);
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
        // Clear block selection when typing
        self.block_selection = None;

        // Check if we're in virtual space
        if let Some(virtual_col) = self.virtual_cursor_col {
            let text = self.rope.slice(..);
            let pos = self.selection.primary().head;
            let line = text.char_to_line(pos);
            let line_start = text.line_to_char(line);
            let line_slice = text.line(line);
            let line_len = line_slice.len_chars();

            // Get effective line length (excluding newline)
            let effective_len = if line_len > 0 && line_slice.char(line_len - 1) == '\n' {
                line_len.saturating_sub(1)
            } else {
                line_len
            };

            // If cursor is past end of line, insert spaces to reach it
            if virtual_col > effective_len {
                let spaces_needed = virtual_col - effective_len;
                let insert_pos = line_start + effective_len;

                // Insert spaces to reach the virtual cursor position
                let mut new_text = self.rope.to_string();
                for _ in 0..spaces_needed {
                    new_text.insert(insert_pos, ' ');
                }
                // Then insert the actual character
                new_text.insert(insert_pos + spaces_needed, ch);
                self.rope = Rope::from_str(&new_text);

                // Update selection to be after the inserted character
                let new_pos = insert_pos + spaces_needed + 1;
                self.selection = Selection::single(new_pos, new_pos);

                // Clear virtual column now that we've filled the gap
                self.virtual_cursor_col = None;
                self.update_cursor_position();
                return;
            }
        }

        // Normal insertion
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
        // Clear virtual column when editing
        self.virtual_cursor_col = None;
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
            // Clear virtual column when editing
            self.virtual_cursor_col = None;
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
            // Clear virtual column when editing
            self.virtual_cursor_col = None;
            return true;
        }
        false
    }


    fn update_cursor_position(&mut self) {
        let text = self.rope.slice(..);
        let range = self.selection.primary();
        let pos = range.cursor(text);
        let (row, col) = self.pos_to_coords(pos);

        // If we have a virtual cursor column, use it for display
        // This allows the cursor to be rendered past the end of lines
        let display_col = self.virtual_cursor_col.unwrap_or(col);

        // Update cursor position with the virtual column for rendering
        self.cursor_pos = Position::new(row, display_col);

        // Follow cursor - use virtual column for scrolling
        self.follow_cursor(display_col, row, 2);
    }

    fn follow_cursor(&mut self, cursor_x: usize, cursor_y: usize, padding: usize) {
        const VIEWPORT_HEIGHT: usize = 20;
        const VIEWPORT_WIDTH: usize = 80;

        // Vertical scrolling - keep cursor visible with padding
        if cursor_y < self.scroll_y + padding {
            self.scroll_y = cursor_y.saturating_sub(padding);
        } else if cursor_y >= self.scroll_y + VIEWPORT_HEIGHT - padding {
            self.scroll_y = cursor_y + padding + 1 - VIEWPORT_HEIGHT;
        }

        // Horizontal scrolling - keep cursor visible with padding
        if cursor_x < self.scroll_x + padding {
            self.scroll_x = cursor_x.saturating_sub(padding);
        } else if cursor_x >= self.scroll_x + VIEWPORT_WIDTH - padding {
            self.scroll_x = cursor_x + padding + 1 - VIEWPORT_WIDTH;
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
        let line_len = line.len_chars();

        // Handle newline at end of line properly
        let effective_len = if line_len > 0 && line.char(line_len - 1) == '\n' {
            line_len.saturating_sub(1)
        } else {
            line_len
        };

        // Clamp column to effective line length
        line_start + col.min(effective_len)
    }


    pub fn get_text(&self) -> String {
        self.rope.to_string()
    }

    pub fn set_text(&mut self, text: &str) {
        self.rope = Rope::from_str(text);
        self.selection = Selection::single(0, 0);
        self.cursor_pos = Position::new(0, 0);
        self.scroll_x = 0;
        self.scroll_y = 0;
    }

    pub fn get_visible_lines(&self, height: usize) -> Vec<String> {
        let mut lines = Vec::new();
        let text = self.rope.slice(..);
        let total_lines = text.len_lines();

        for i in self.scroll_y..self.scroll_y + height {
            if i >= total_lines {
                // Add empty lines if we're past the end of the document
                lines.push(String::new());
            } else {
                let line = text.line(i);
                // Handle horizontal scrolling if needed
                let line_str = line.to_string();
                if self.scroll_x > 0 && line_str.len() > self.scroll_x {
                    lines.push(line_str.chars().skip(self.scroll_x).collect());
                } else if self.scroll_x > 0 {
                    lines.push(String::new());
                } else {
                    lines.push(line_str);
                }
            }
        }

        lines
    }

    pub fn get_cursor_screen_position(&self) -> (usize, usize) {
        let screen_row = self.cursor_pos.row.saturating_sub(self.scroll_y);
        // Use virtual cursor column if set (for rendering cursor in virtual space)
        let display_col = self.virtual_cursor_col.unwrap_or(self.cursor_pos.col);
        let screen_col = display_col.saturating_sub(self.scroll_x);
        (screen_row, screen_col)
    }

    pub fn get_cursor_position(&self) -> usize {
        // Return the absolute cursor position in the text
        let text = self.rope.slice(..);
        let range = self.selection.primary();
        range.cursor(text)
    }

    /// Set cursor position to specific row and column, allowing positioning anywhere on the grid
    pub fn set_cursor_position(&mut self, row: usize, col: usize) {
        let line_count = self.rope.len_lines();

        // Allow cursor to be placed on any row, even past the end of the document
        let target_row = row;

        if target_row >= line_count {
            // If clicking beyond the last line, extend the document with empty lines
            let lines_to_add = (target_row + 1).saturating_sub(line_count);
            for _ in 0..lines_to_add {
                let pos = self.rope.len_chars();
                self.rope.insert_char(pos, '\n');
            }
        }

        // Now set the cursor to the target position
        let line = target_row.min(self.rope.len_lines().saturating_sub(1));
        let line_start = self.rope.line_to_char(line);
        let _line_end = if line < self.rope.len_lines() - 1 {
            self.rope.line_to_char(line + 1).saturating_sub(1) // Exclude the newline
        } else {
            self.rope.len_chars()
        };

        // Allow cursor to be placed anywhere on the line, even past its end (virtual space)
        // We don't actually insert spaces, just set the cursor position virtually
        let char_pos = line_start + col.min(self.rope.line(line).len_chars());
        self.selection = Selection::single(char_pos, char_pos);

        // Update cursor position to support virtual columns
        self.cursor_pos = Position::new(line, col);

        // Set virtual cursor column for vertical movement
        self.virtual_cursor_col = Some(col);
    }

    // Selection extension methods
    fn extend_selection_left(&mut self) {
        let text = self.rope.slice(..);
        let range = self.selection.primary();
        let cursor = range.cursor(text);

        if self.selection_anchor.is_none() {
            self.selection_anchor = Some(cursor);
        }

        let new_cursor = prev_grapheme_boundary(text, cursor);
        if let Some(anchor) = self.selection_anchor {
            self.selection = Selection::single(anchor, new_cursor);
        }
    }

    fn extend_selection_right(&mut self) {
        let text = self.rope.slice(..);
        let range = self.selection.primary();
        let cursor = range.cursor(text);

        if self.selection_anchor.is_none() {
            self.selection_anchor = Some(cursor);
        }

        let new_cursor = next_grapheme_boundary(text, cursor);
        if let Some(anchor) = self.selection_anchor {
            self.selection = Selection::single(anchor, new_cursor);
        }
    }

    fn extend_selection_up(&mut self) {
        let text = self.rope.slice(..);
        let range = self.selection.primary();
        let cursor = range.cursor(text);

        if self.selection_anchor.is_none() {
            self.selection_anchor = Some(cursor);
        }

        let line = text.char_to_line(cursor);
        if line > 0 {
            let new_line = line - 1;
            let line_start = text.line_to_char(line);
            let col = cursor - line_start;
            let new_cursor = self.coords_to_pos(new_line, col);

            if let Some(anchor) = self.selection_anchor {
                self.selection = Selection::single(anchor, new_cursor);
            }
        }
    }

    fn extend_selection_down(&mut self) {
        let text = self.rope.slice(..);
        let range = self.selection.primary();
        let cursor = range.cursor(text);

        if self.selection_anchor.is_none() {
            self.selection_anchor = Some(cursor);
        }

        let line = text.char_to_line(cursor);
        let total_lines = text.len_lines();
        if line < total_lines - 1 {
            let new_line = line + 1;
            let line_start = text.line_to_char(line);
            let col = cursor - line_start;
            let new_cursor = self.coords_to_pos(new_line, col);

            if let Some(anchor) = self.selection_anchor {
                self.selection = Selection::single(anchor, new_cursor);
            }
        }
    }

    fn select_all(&mut self) {
        let text = self.rope.slice(..);
        let len = text.len_chars();
        self.selection = Selection::single(0, len);
        self.selection_anchor = Some(0);
    }

    // Block selection extensions
    fn extend_block_selection_left(&mut self) {
        let pos = self.selection.primary().head;
        let line = self.rope.char_to_line(pos);
        let line_start = self.rope.line_to_char(line);
        let col = pos - line_start;

        // Initialize block selection if needed
        if self.block_selection.is_none() {
            self.block_selection = Some(crate::block_selection::BlockSelection::new(line, col));
        }

        // Move cursor left
        if col > 0 {
            let new_col = col - 1;
            let line_slice = self.rope.slice(..).line(line);
            let visual_col = crate::block_selection::char_idx_to_visual_col(line_slice, new_col);

            if let Some(block_sel) = &mut self.block_selection {
                block_sel.extend_to(line, new_col, visual_col);
            }
        }
    }

    fn extend_block_selection_right(&mut self) {
        let pos = self.selection.primary().head;
        let line = self.rope.char_to_line(pos);
        let line_start = self.rope.line_to_char(line);
        let col = pos - line_start;
        let line_len = self.rope.line(line).len_chars();

        // Initialize block selection if needed
        if self.block_selection.is_none() {
            self.block_selection = Some(crate::block_selection::BlockSelection::new(line, col));
        }

        // Move cursor right
        if col < line_len {
            let new_col = col + 1;
            let line_slice = self.rope.slice(..).line(line);
            let visual_col = crate::block_selection::char_idx_to_visual_col(line_slice, new_col);

            if let Some(block_sel) = &mut self.block_selection {
                block_sel.extend_to(line, new_col, visual_col);
            }
        }
    }

    fn extend_block_selection_up(&mut self) {
        let pos = self.selection.primary().head;
        let line = self.rope.char_to_line(pos);
        let line_start = self.rope.line_to_char(line);
        let col = pos - line_start;

        // Initialize block selection if needed
        if self.block_selection.is_none() {
            self.block_selection = Some(crate::block_selection::BlockSelection::new(line, col));
        }

        // Move cursor up
        if line > 0 {
            let new_line = line - 1;
            let line_slice = self.rope.slice(..).line(new_line);
            let visual_col = crate::block_selection::char_idx_to_visual_col(line_slice, col);

            if let Some(block_sel) = &mut self.block_selection {
                block_sel.extend_to(new_line, col, visual_col);
            }
        }
    }

    fn extend_block_selection_down(&mut self) {
        let pos = self.selection.primary().head;
        let line = self.rope.char_to_line(pos);
        let line_start = self.rope.line_to_char(line);
        let col = pos - line_start;
        let max_line = self.rope.len_lines() - 1;

        // Initialize block selection if needed
        if self.block_selection.is_none() {
            self.block_selection = Some(crate::block_selection::BlockSelection::new(line, col));
        }

        // Move cursor down
        if line < max_line {
            let new_line = line + 1;
            let line_slice = self.rope.slice(..).line(new_line);
            let visual_col = crate::block_selection::char_idx_to_visual_col(line_slice, col);

            if let Some(block_sel) = &mut self.block_selection {
                block_sel.extend_to(new_line, col, visual_col);
            }
        }
    }

    // Clipboard operations
    fn copy_selection(&self) -> Result<()> {
        // Handle block selection first
        if let Some(ref block_sel) = self.block_selection {
            let mut lines = Vec::new();

            for (line_idx, start_col, end_col) in block_sel.iter_lines() {
                // Allow selecting beyond the actual number of lines
                let line_text = if line_idx < self.rope.len_lines() {
                    let line = self.rope.slice(..).line(line_idx);
                    // Get the line as string without the newline
                    let mut text = line.to_string();
                    if text.ends_with('\n') {
                        text.pop();
                    }
                    text
                } else {
                    // Line doesn't exist - treat as empty
                    String::new()
                };

                // Get the visual width of the line (handling tabs)
                let mut visual_width = 0;
                let mut char_to_visual = Vec::new();
                for ch in line_text.chars() {
                    char_to_visual.push(visual_width);
                    visual_width += match ch {
                        '\t' => 4 - (visual_width % 4),
                        _ => unicode_width::UnicodeWidthChar::width(ch).unwrap_or(1),
                    };
                }
                char_to_visual.push(visual_width); // Position after last char

                // Build the selected text with proper padding
                let mut selected = String::new();

                // Pad beginning if selection starts beyond line content
                if start_col > visual_width {
                    for _ in visual_width..start_col {
                        selected.push(' ');
                    }
                }

                // Extract the actual text content
                let mut current_visual = 0;
                for ch in line_text.chars() {
                    let ch_width = match ch {
                        '\t' => 4 - (current_visual % 4),
                        _ => unicode_width::UnicodeWidthChar::width(ch).unwrap_or(1),
                    };

                    if current_visual >= end_col {
                        break;
                    }

                    if current_visual + ch_width > start_col {
                        if current_visual < start_col {
                            // Partially selected tab - replace with spaces
                            for _ in start_col..((current_visual + ch_width).min(end_col)) {
                                selected.push(' ');
                            }
                        } else {
                            selected.push(ch);
                        }
                    }

                    current_visual += ch_width;
                }

                // Pad end if selection extends beyond line content
                if end_col > current_visual && current_visual >= start_col {
                    for _ in current_visual..end_col {
                        selected.push(' ');
                    }
                }

                lines.push(selected);
            }

            let block_text = lines.join("\n");
            self.copy_to_clipboard(&block_text)?;
        } else {
            // Regular selection
            let text = self.rope.slice(..);
            let range = self.selection.primary();

            if range.len() > 0 {
                let selected_text = text.slice(range.from()..range.to()).to_string();
                self.copy_to_clipboard(&selected_text)?;
            }
        }

        Ok(())
    }

    fn cut_selection(&mut self) -> Result<bool> {
        // Handle block selection first
        if let Some(ref block_sel) = self.block_selection.clone() {
            // First copy the block selection
            self.copy_selection()?;

            // Delete the block selection from bottom to top to maintain line indices
            let mut rope_str = self.rope.to_string();
            let mut lines: Vec<String> = rope_str.lines().map(|s| s.to_string()).collect();

            // If the last line doesn't end with a newline, the lines() iterator won't include an empty final line
            if rope_str.ends_with('\n') && !rope_str.ends_with("\n\n") {
                // Nothing to do - lines() handled it correctly
            } else if !rope_str.is_empty() && !rope_str.ends_with('\n') {
                // No trailing newline - lines() handled it correctly
            }

            for (line_idx, start_col, end_col) in block_sel.iter_lines().rev() {
                if line_idx >= lines.len() {
                    continue;
                }

                let line = &lines[line_idx];
                let line_slice = helix_core::RopeSlice::from(line.as_str());

                // Convert visual columns to char indices
                let start_char = crate::block_selection::visual_col_to_char_idx(line_slice, start_col);
                let end_char = crate::block_selection::visual_col_to_char_idx(line_slice, end_col);

                // Clamp to line length
                let start_char = start_char.min(line.len());
                let end_char = end_char.min(line.len());

                if start_char < end_char {
                    let mut new_line = String::new();
                    new_line.push_str(&line[..start_char]);
                    new_line.push_str(&line[end_char..]);
                    lines[line_idx] = new_line;
                }
            }

            // Reconstruct the rope
            let new_text = lines.join("\n");
            self.rope = Rope::from_str(&new_text);

            // Clear block selection
            self.block_selection = None;

            // Update selection to cursor position
            let (_, (max_line, max_col)) = block_sel.visual_bounds();
            let cursor_line = max_line.min(self.rope.len_lines().saturating_sub(1));
            let line_start = self.rope.line_to_char(cursor_line);
            let cursor_pos = line_start + max_col.min(self.rope.line(cursor_line).len_chars());
            self.selection = Selection::point(cursor_pos);
            self.selection_anchor = None;

            return Ok(true);
        }

        // Regular selection
        let text = self.rope.slice(..);
        let range = self.selection.primary();

        if range.len() > 0 {
            // Copy to clipboard first
            let selected_text = text.slice(range.from()..range.to()).to_string();
            self.copy_to_clipboard(&selected_text)?;

            // Delete the selection
            let mut new_text = self.rope.to_string();
            new_text.drain(range.from()..range.to());
            self.rope = Rope::from_str(&new_text);

            // Update selection
            self.selection = Selection::point(range.from());
            self.selection_anchor = None;

            return Ok(true);
        }

        Ok(false)
    }

    fn paste(&mut self) -> Result<bool> {
        if let Ok(clipboard_text) = self.paste_from_clipboard() {
            let range = self.selection.primary();
            let mut new_text = self.rope.to_string();

            // Calculate new position before modifying the rope
            let new_pos = if range.len() > 0 {
                // If there's a selection, replace it
                new_text.drain(range.from()..range.to());
                new_text.insert_str(range.from(), &clipboard_text);
                range.from() + clipboard_text.len()
            } else {
                // Otherwise insert at cursor
                let text = self.rope.slice(..);
                let pos = range.cursor(text);
                new_text.insert_str(pos, &clipboard_text);
                pos + clipboard_text.len()
            };

            self.rope = Rope::from_str(&new_text);
            self.selection = Selection::point(new_pos);
            self.selection_anchor = None;

            return Ok(true);
        }

        Ok(false)
    }

    fn copy_to_clipboard(&self, text: &str) -> Result<()> {
        #[cfg(target_os = "macos")]
        {
            let mut child = Command::new("pbcopy")
                .stdin(std::process::Stdio::piped())
                .spawn()?;

            if let Some(mut stdin) = child.stdin.take() {
                use std::io::Write;
                stdin.write_all(text.as_bytes())?;
            }

            child.wait()?;
        }

        #[cfg(not(target_os = "macos"))]
        {
            // For Linux, use xclip or xsel
            let mut child = Command::new("xclip")
                .arg("-selection")
                .arg("clipboard")
                .stdin(std::process::Stdio::piped())
                .spawn()?;

            if let Some(mut stdin) = child.stdin.take() {
                use std::io::Write;
                stdin.write_all(text.as_bytes())?;
            }

            child.wait()?;
        }

        Ok(())
    }

    fn paste_from_clipboard(&self) -> Result<String> {
        #[cfg(target_os = "macos")]
        {
            let output = Command::new("pbpaste").output()?;
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        }

        #[cfg(not(target_os = "macos"))]
        {
            let output = Command::new("xclip")
                .arg("-selection")
                .arg("clipboard")
                .arg("-o")
                .output()?;
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        }
    }

    pub fn has_selection(&self) -> bool {
        self.selection.primary().len() > 0
    }

    pub fn get_selection(&self) -> Option<String> {
        let range = self.selection.primary();
        if range.len() > 0 {
            let text = self.rope.slice(..);
            Some(text.slice(range.from()..range.to()).to_string())
        } else {
            None
        }
    }
}