// Terminal UI rendering for NValt-like interface

use anyhow::Result;
use crossterm::{
    cursor,
    style::{self, Color, SetBackgroundColor, SetForegroundColor},
    terminal,
    execute,
};
use std::io;
use unicode_width::UnicodeWidthStr;

use crate::{App, FocusArea};
use crate::syntax::SyntaxHighlighter;
use crate::edit_renderer::EditPanelRenderer;

pub struct UI {
    syntax_highlighter: SyntaxHighlighter,
    edit_renderer: EditPanelRenderer,
}

impl UI {
    pub fn new() -> Result<Self> {
        Ok(UI {
            syntax_highlighter: SyntaxHighlighter::new()?,
            edit_renderer: EditPanelRenderer::new(80, 24),  // Default size, will be updated
        })
    }

    /// Handle mouse click in the editor area and convert to document position
    pub fn handle_editor_click(&self, app: &mut App, click_row: usize, click_col: usize) {
        // Get the current scroll offsets from the edit renderer
        let (scroll_x, scroll_y) = self.edit_renderer.get_scroll();

        // Convert screen position to document position by adding scroll offsets
        let doc_row = click_row + scroll_y as usize;
        let doc_col = click_col + scroll_x as usize;

        // Set the cursor position in the editor (allows virtual positioning anywhere on grid)
        app.editor.set_cursor_position(doc_row, doc_col);
    }

    pub fn render(&mut self, app: &App) -> Result<()> {
        let (width, height) = terminal::size()?;

        // Clear screen
        execute!(
            io::stdout(),
            terminal::Clear(terminal::ClearType::All),
            cursor::MoveTo(0, 0),
        )?;

        // Calculate split positions (left-right split)
        let split_x = (width as f32 * app.split_ratio) as u16;
        let editor_width = width.saturating_sub(split_x + 1);  // +1 for divider

        // Always render header and search bar
        self.render_header(app, width)?;
        self.render_search_bar(app, width)?;

        // Render note list on left (starting at line 3)
        self.render_note_list(app, split_x, 2, height - 3)?;  // -3 for header, search, and status

        // Render divider
        self.render_divider(split_x, 2, height - 3, app.dragging_divider)?;

        // Render editor on right
        self.render_editor(app, split_x + 1, editor_width, 2, height - 3)?;

        self.render_status_bar(app, width, height)?;

        // Position cursor based on focus area
        match app.focus_area {
            FocusArea::SearchBar => {
                let search_len = app.search_query.width() as u16;
                execute!(io::stdout(), cursor::Show, cursor::MoveTo(9 + search_len, 1))?;
            }
            _ => {
                // Hide the terminal cursor - we render our own block cursor in editor
                execute!(io::stdout(), cursor::Hide)?;
            }
        }

        Ok(())
    }

    fn render_header(&self, app: &App, width: u16) -> Result<()> {
        execute!(
            io::stdout(),
            cursor::MoveTo(0, 0),
            SetBackgroundColor(Color::Rgb { r: 40, g: 40, b: 40 }),
            SetForegroundColor(Color::Rgb { r: 200, g: 200, b: 200 }),
        )?;

        let header = format!(" Snyfter3 - {} notes ",
            app.notes.get_note_count()
        );

        print!("{:width$}", header, width = width as usize);

        execute!(
            io::stdout(),
            style::ResetColor,
        )?;

        Ok(())
    }

    fn render_divider(&self, x: u16, start_y: u16, height: u16, is_dragging: bool) -> Result<()> {
        let color = if is_dragging {
            Color::Rgb { r: 100, g: 150, b: 200 }
        } else {
            Color::Rgb { r: 60, g: 60, b: 60 }
        };

        for y in start_y..start_y + height {
            execute!(
                io::stdout(),
                cursor::MoveTo(x, y),
                SetForegroundColor(color),
            )?;
            print!("│");
        }

        execute!(io::stdout(), style::ResetColor)?;
        Ok(())
    }

    fn render_search_bar(&self, app: &App, width: u16) -> Result<()> {
        let is_focused = app.focus_area == FocusArea::SearchBar;

        execute!(
            io::stdout(),
            cursor::MoveTo(0, 1),
            SetBackgroundColor(if is_focused {
                Color::Rgb { r: 50, g: 70, b: 120 }  // Blue background when focused
            } else {
                Color::Rgb { r: 35, g: 35, b: 35 }
            }),
            SetForegroundColor(if is_focused {
                Color::White
            } else {
                Color::Rgb { r: 150, g: 150, b: 150 }
            }),
        )?;

        print!(" Search: {}", app.search_query);

        // Show match count
        let match_info = format!(" ({} notes) ", app.filtered_notes.len());

        // Clear rest of line
        let used = 9 + app.search_query.width() + match_info.width();
        if used < width as usize {
            print!("{}", match_info);
            print!("{:width$}", "", width = width as usize - used);
        }

        execute!(io::stdout(), style::ResetColor)?;
        Ok(())
    }

    fn render_note_list(&self, app: &App, width: u16, start_y: u16, height: u16) -> Result<()> {
        // Display search results if searching, otherwise all notes
        let display_height = height - 1;

        // Render list header with focus indication
        let is_focused = app.focus_area == FocusArea::NoteList;

        execute!(
            io::stdout(),
            cursor::MoveTo(0, start_y),
            SetBackgroundColor(if is_focused {
                Color::Rgb { r: 40, g: 50, b: 70 }  // Darker blue when focused
            } else {
                Color::Rgb { r: 30, g: 30, b: 30 }
            }),
            SetForegroundColor(if is_focused {
                Color::Rgb { r: 200, g: 200, b: 200 }
            } else {
                Color::Rgb { r: 150, g: 150, b: 150 }
            }),
        )?;

        print!("{:width$}", " NOTES", width = width as usize);

        // Render filtered notes
        for (i, note) in app.filtered_notes.iter().enumerate() {
            if i >= display_height as usize {
                break;
            }

            let y = start_y + 1 + i as u16;
            execute!(io::stdout(), cursor::MoveTo(0, y))?;

            // Highlight selected note
            if i == app.selected_note_index {
                execute!(
                    io::stdout(),
                    SetBackgroundColor(Color::Rgb { r: 60, g: 60, b: 100 }),
                    SetForegroundColor(Color::Rgb { r: 255, g: 255, b: 255 }),
                )?;
            } else {
                execute!(
                    io::stdout(),
                    SetBackgroundColor(Color::Black),
                    SetForegroundColor(Color::Rgb { r: 200, g: 200, b: 200 }),
                )?;
            }

            // Format note line
            let title = if note.title.width() > (width as usize - 4) {
                format!("{}...", &note.title.chars().take(width as usize - 7).collect::<String>())
            } else {
                note.title.clone()
            };

            print!(" {:<width$}", title, width = width as usize - 1);
        }

        // Clear remaining lines
        for i in app.filtered_notes.len()..display_height as usize {
            let y = start_y + 1 + i as u16;
            execute!(
                io::stdout(),
                cursor::MoveTo(0, y),
                SetBackgroundColor(Color::Black),
            )?;
            print!("{:width$}", "", width = width as usize);
        }


        execute!(io::stdout(), style::ResetColor)?;
        Ok(())
    }

    fn render_editor(&mut self, app: &App, start_x: u16, width: u16, start_y: u16, height: u16) -> Result<()> {
        // Render editor header with focus indication
        let is_focused = app.focus_area == FocusArea::Editor;

        execute!(
            io::stdout(),
            cursor::MoveTo(start_x, start_y),
            SetBackgroundColor(if is_focused {
                Color::Rgb { r: 40, g: 50, b: 70 }  // Darker blue when focused
            } else {
                Color::Rgb { r: 30, g: 30, b: 30 }
            }),
            SetForegroundColor(if is_focused {
                Color::Rgb { r: 200, g: 200, b: 200 }
            } else {
                Color::Rgb { r: 150, g: 150, b: 150 }
            }),
        )?;

        let editor_header = if let Some(ref note) = app.selected_note {
            format!(" EDITOR - {} ", note.title)
        } else {
            " EDITOR - No note selected ".to_string()
        };

        print!("{:width$}", editor_header, width = width as usize);

        // Use the EditPanelRenderer for exact chonker7 rendering
        if let Some(ref _note) = app.selected_note {
            // Update renderer size if needed
            self.edit_renderer.resize(width, height - 1);

            // Update content from the rope
            self.edit_renderer.update_from_rope(&app.editor.rope);

            // Get cursor position - use the virtual cursor position from the editor
            let cursor_line = app.editor.cursor_pos.row;
            let cursor_col = app.editor.cursor_pos.col;

            // Make cursor follow viewport
            self.edit_renderer.follow_cursor(cursor_col, cursor_line, 3);

            // Don't pass any selection bounds - we only want block selections and cursor
            let (sel_start, sel_end) = (None, None);

            // Render with cursor and selection using exact chonker7 colors (RGB 80,80,200)
            // Use block selection renderer if block selection is active
            self.edit_renderer.render_with_cursor_and_block_selection(
                start_x, start_y + 1, width, height - 1,
                (cursor_col, cursor_line),
                app.editor.block_selection.as_ref(),
                sel_start,
                sel_end
            )?;
        } else {
            // No note selected - clear the editor area
            execute!(
                io::stdout(),
                SetBackgroundColor(Color::Black),
                SetForegroundColor(Color::Rgb { r: 100, g: 100, b: 100 }),
            )?;

            for i in 0..height - 1 {
                execute!(io::stdout(), cursor::MoveTo(start_x, start_y + 1 + i))?;
                print!("{:width$}", "", width = width as usize);
            }
        }

        execute!(io::stdout(), style::ResetColor)?;
        Ok(())
    }
    fn render_status_bar(&self, app: &App, width: u16, height: u16) -> Result<()> {
        execute!(
            io::stdout(),
            cursor::MoveTo(0, height - 1),
            SetBackgroundColor(Color::Rgb { r: 40, g: 40, b: 40 }),
            SetForegroundColor(Color::Rgb { r: 180, g: 180, b: 180 }),
        )?;

        let left_status = format!(" {} ", app.status_message);

        let shortcuts = match app.focus_area {
            FocusArea::SearchBar => "ESC/Enter/↓: Exit Search | Type to filter notes",
            FocusArea::NoteList => "^Q: Quit | ^N: New | ^L/^F: Search | Enter/→: Edit | ^D: Delete | Tab: Switch Focus",
            FocusArea::Editor => "ESC/←: Back to List | ^X: Cut | ^C: Copy | ^V: Paste | ^A: Select All | Tab: Switch Focus",
        };

        let right_status = format!(" {} ", shortcuts);

        print!("{}", left_status);

        let padding = width as usize - left_status.width() - right_status.width();
        if padding > 0 {
            print!("{:width$}", "", width = padding);
        }

        print!("{}", right_status);

        execute!(io::stdout(), style::ResetColor)?;
        Ok(())
    }
}