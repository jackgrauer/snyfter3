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

use crate::{App, AppMode};

pub struct UI {}

impl UI {
    pub fn new() -> Result<Self> {
        Ok(UI {})
    }

    pub fn render(&self, app: &App) -> Result<()> {
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

        // Render header across top
        if app.mode == AppMode::Search {
            self.render_search_box(app, width)?;
        } else {
            self.render_header(app, width)?;
        }

        // Render note list on left
        self.render_note_list(app, split_x, 1, height - 2)?;  // -2 for header and status

        // Render divider
        self.render_divider(split_x, 1, height - 2, app.dragging_divider)?;

        // Render editor on right
        self.render_editor(app, split_x + 1, editor_width, 1, height - 2)?;

        self.render_status_bar(app, width, height)?;

        // Position cursor based on mode
        match app.mode {
            AppMode::Search => {
                let search_len = app.search_query.width() as u16;
                execute!(io::stdout(), cursor::Show, cursor::MoveTo(9 + search_len, 0))?;
            }
            AppMode::NoteEdit => {
                // Show cursor in editor at correct position (right pane)
                let split_x = (width as f32 * app.split_ratio) as u16;
                let (cursor_row, cursor_col) = app.editor.get_cursor_screen_position();
                let actual_row = 2 + cursor_row as u16;  // 2 for header and border
                let actual_col = split_x + 1 + cursor_col as u16;  // +1 for divider
                execute!(io::stdout(), cursor::Show, cursor::MoveTo(actual_col, actual_row))?;
            }
            _ => {
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

        let header = format!(" Snyfter3 - {} notes | Mode: {:?} ",
            app.notes.get_note_count(),
            app.mode
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

    fn render_search_box(&self, app: &App, width: u16) -> Result<()> {
        execute!(
            io::stdout(),
            cursor::MoveTo(0, 0),
            SetBackgroundColor(Color::Rgb { r: 50, g: 50, b: 50 }),
            SetForegroundColor(Color::Rgb { r: 255, g: 255, b: 255 }),
        )?;

        print!(" Search: {}", app.search_query);

        // Clear rest of line
        let used = 9 + app.search_query.width();
        if used < width as usize {
            print!("{:width$}", "", width = width as usize - used);
        }

        execute!(io::stdout(), style::ResetColor)?;
        Ok(())
    }

    fn render_note_list(&self, app: &App, width: u16, start_y: u16, height: u16) -> Result<()> {
        // Display search results if searching, otherwise all notes
        let display_height = height - 1;

        // Render list header
        execute!(
            io::stdout(),
            cursor::MoveTo(0, start_y),
            SetBackgroundColor(Color::Rgb { r: 30, g: 30, b: 30 }),
            SetForegroundColor(Color::Rgb { r: 150, g: 150, b: 150 }),
        )?;

        print!("{:width$}", " NOTES", width = width as usize);

        // Render notes or search results
        if !app.search_query.is_empty() && !app.search_results.is_empty() {
            // Render search results
            for (i, result) in app.search_results.iter().enumerate() {
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

                // Format search result line with preview
                let display_text = if result.score > 0.0 {
                    format!("{} [{}]", result.title, (result.score * 100.0) as u32)
                } else {
                    result.title.clone()
                };

                let display = if display_text.width() > (width as usize - 4) {
                    format!("{}...", &display_text.chars().take(width as usize - 7).collect::<String>())
                } else {
                    display_text
                };

                print!(" {:<width$}", display, width = width as usize - 1);
            }

            // Clear remaining lines
            for i in app.search_results.len()..display_height as usize {
                let y = start_y + 1 + i as u16;
                execute!(
                    io::stdout(),
                    cursor::MoveTo(0, y),
                    SetBackgroundColor(Color::Black),
                )?;
                print!("{:width$}", "", width = width as usize);
            }
        } else {
            // Render all notes
            let notes = app.notes.get_all_notes()?;
            for (i, note) in notes.iter().enumerate() {
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
            for i in notes.len()..display_height as usize {
                let y = start_y + 1 + i as u16;
                execute!(
                    io::stdout(),
                    cursor::MoveTo(0, y),
                    SetBackgroundColor(Color::Black),
                )?;
                print!("{:width$}", "", width = width as usize);
            }
        }


        execute!(io::stdout(), style::ResetColor)?;
        Ok(())
    }

    fn render_editor(&self, app: &App, start_x: u16, width: u16, start_y: u16, height: u16) -> Result<()> {
        // Render editor header
        execute!(
            io::stdout(),
            cursor::MoveTo(start_x, start_y),
            SetBackgroundColor(Color::Rgb { r: 30, g: 30, b: 30 }),
            SetForegroundColor(Color::Rgb { r: 150, g: 150, b: 150 }),
        )?;

        let editor_header = if let Some(ref note) = app.selected_note {
            format!(" EDITOR - {} ", note.title)
        } else {
            " EDITOR - No note selected ".to_string()
        };

        print!("{:width$}", editor_header, width = width as usize);

        // Render editor content
        execute!(
            io::stdout(),
            SetBackgroundColor(Color::Black),
            SetForegroundColor(Color::Rgb { r: 200, g: 200, b: 200 }),
        )?;

        if let Some(ref _note) = app.selected_note {
            // Get visible lines from editor
            let visible_lines = app.editor.get_visible_lines(height as usize - 1);
            let (cursor_row, cursor_col) = app.editor.get_cursor_screen_position();

            for (i, line) in visible_lines.iter().enumerate() {
                if i >= height as usize - 1 {
                    break;
                }

                let y = start_y + 1 + i as u16;
                execute!(io::stdout(), cursor::MoveTo(start_x, y))?;

                // For now, render without code highlights (will add later)
                // Show cursor position
                if app.mode == AppMode::NoteEdit && i == cursor_row {
                    // Render line with cursor
                    print!(" ");
                    for (j, ch) in line.chars().enumerate() {
                        if j == cursor_col {
                            execute!(
                                io::stdout(),
                                SetBackgroundColor(Color::Rgb { r: 100, g: 100, b: 100 }),
                                SetForegroundColor(Color::Rgb { r: 255, g: 255, b: 255 }),
                            )?;
                            print!("{}", ch);
                            execute!(
                                io::stdout(),
                                SetBackgroundColor(Color::Black),
                                SetForegroundColor(Color::Rgb { r: 200, g: 200, b: 200 }),
                            )?;
                        } else {
                            print!("{}", ch);
                        }
                    }
                    // Show cursor at end of line if needed
                    if cursor_col >= line.len() {
                        execute!(
                            io::stdout(),
                            SetBackgroundColor(Color::Rgb { r: 100, g: 100, b: 100 }),
                        )?;
                        print!(" ");
                        execute!(
                            io::stdout(),
                            SetBackgroundColor(Color::Black),
                        )?;
                    }
                    // Fill rest of line
                    let printed = 1 + line.width() + if cursor_col >= line.len() { 1 } else { 0 };
                    if printed < width as usize {
                        print!("{:width$}", "", width = width as usize - printed);
                    }
                } else {
                    // Regular line rendering
                    print!(" {:<width$}", line, width = width as usize - 1);
                }

            }

            // Clear remaining lines
            for i in visible_lines.len()..height as usize - 1 {
                let y = start_y + 1 + i as u16;
                execute!(io::stdout(), cursor::MoveTo(start_x, y))?;
                print!("{:width$}", "", width = width as usize);
            }
        } else {
            // No note selected - show placeholder
            for i in 0..height - 1 {
                let y = start_y + 1 + i;
                execute!(io::stdout(), cursor::MoveTo(start_x, y))?;

                if i == height / 2 - 1 {
                    let msg = "Select or create a note to begin editing";
                    let padding = (width as usize - msg.len()) / 2;
                    print!("{:padding$}{}{:padding$}", "", msg, "",
                           padding = padding);
                } else {
                    print!("{:width$}", "", width = width as usize);
                }
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

        let shortcuts = match app.mode {
            AppMode::Search => "ESC: Cancel | Enter: Search",
            AppMode::NoteList => "^Q: Quit | ^N: New | ^F: Search | ^D: Delete | ^</>: Resize | Enter: Edit",
            AppMode::NoteEdit => "ESC: Back | ^S: Save | ^H: Highlight | ^</>: Resize",
            AppMode::CodeManager => "ESC: Back | N: New Code",
            AppMode::Highlighting => "ESC: Cancel | Enter: Apply Code",
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