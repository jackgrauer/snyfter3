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

        // Calculate split positions
        let split_y = (height as f32 * app.split_ratio) as u16;

        // Render based on mode
        match app.mode {
            AppMode::Search => {
                self.render_search_box(app, width)?;
                self.render_note_list(app, width, 2, split_y)?;
                self.render_editor(app, width, split_y + 1, height - split_y - 2)?;
            }
            _ => {
                self.render_header(app, width)?;
                self.render_note_list(app, width, 1, split_y)?;
                self.render_editor(app, width, split_y + 1, height - split_y - 2)?;
            }
        }

        self.render_status_bar(app, width, height)?;

        // Position cursor based on mode
        match app.mode {
            AppMode::Search => {
                let search_len = app.search_query.width() as u16;
                execute!(io::stdout(), cursor::MoveTo(9 + search_len, 0))?;
            }
            AppMode::NoteEdit => {
                // Position cursor in editor
                // TODO: Calculate actual cursor position based on rope selection
                execute!(io::stdout(), cursor::MoveTo(2, split_y + 2))?;
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
        // Get notes to display
        let notes = if app.search_query.is_empty() {
            app.notes.get_all_notes()?
        } else {
            // Search results would go here
            app.notes.search_notes(&app.search_query)?
        };

        // Render list header
        execute!(
            io::stdout(),
            cursor::MoveTo(0, start_y),
            SetBackgroundColor(Color::Rgb { r: 30, g: 30, b: 30 }),
            SetForegroundColor(Color::Rgb { r: 150, g: 150, b: 150 }),
        )?;

        print!("{:width$}", " NOTES", width = width as usize);

        // Render notes
        let display_height = height - 1;
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
                format!("{}...", &note.title[..width as usize - 7])
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

        execute!(io::stdout(), style::ResetColor)?;
        Ok(())
    }

    fn render_editor(&self, app: &App, width: u16, start_y: u16, height: u16) -> Result<()> {
        // Render editor border
        execute!(
            io::stdout(),
            cursor::MoveTo(0, start_y),
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

        if let Some(ref note) = app.selected_note {
            // Render with syntax highlighting for coded segments
            let rope_string = app.rope.to_string();
            let lines: Vec<&str> = rope_string.lines().collect();

            for (i, line) in lines.iter().enumerate() {
                if i >= height as usize - 1 {
                    break;
                }

                let y = start_y + 1 + i as u16;
                execute!(io::stdout(), cursor::MoveTo(0, y))?;

                // Check if this line contains coded segments
                let line_start = app.rope.to_string()
                    .lines()
                    .take(i)
                    .map(|l| l.len() + 1) // +1 for newline
                    .sum::<usize>();

                let line_end = line_start + line.len();

                // Find codes that overlap with this line
                let mut highlighted_ranges = Vec::new();
                for code in &note.codes {
                    if code.start_offset < line_end && code.end_offset > line_start {
                        let start = code.start_offset.saturating_sub(line_start);
                        let end = (code.end_offset - line_start).min(line.len());
                        highlighted_ranges.push((start, end));
                    }
                }

                // Render line with highlights
                if highlighted_ranges.is_empty() {
                    print!(" {:<width$}", line, width = width as usize - 1);
                } else {
                    print!(" ");
                    let mut last_end = 0;

                    for (start, end) in highlighted_ranges {
                        // Print normal text before highlight
                        if start > last_end {
                            print!("{}", &line[last_end..start]);
                        }

                        // Print highlighted text
                        execute!(
                            io::stdout(),
                            SetBackgroundColor(Color::Rgb { r: 100, g: 100, b: 50 }),
                            SetForegroundColor(Color::Rgb { r: 255, g: 255, b: 200 }),
                        )?;
                        print!("{}", &line[start..end]);
                        execute!(
                            io::stdout(),
                            SetBackgroundColor(Color::Black),
                            SetForegroundColor(Color::Rgb { r: 200, g: 200, b: 200 }),
                        )?;

                        last_end = end;
                    }

                    // Print remaining normal text
                    if last_end < line.len() {
                        print!("{}", &line[last_end..]);
                    }

                    // Fill rest of line
                    let printed = 1 + line.width();
                    if printed < width as usize {
                        print!("{:width$}", "", width = width as usize - printed);
                    }
                }
            }

            // Clear remaining lines
            for i in lines.len()..height as usize - 1 {
                let y = start_y + 1 + i as u16;
                execute!(io::stdout(), cursor::MoveTo(0, y))?;
                print!("{:width$}", "", width = width as usize);
            }
        } else {
            // No note selected - show placeholder
            for i in 0..height - 1 {
                let y = start_y + 1 + i;
                execute!(io::stdout(), cursor::MoveTo(0, y))?;

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
            AppMode::NoteList => "^Q: Quit | ^N: New | ^F: Search | ^T: Tags | ↑↓: Navigate | Enter: Edit",
            AppMode::NoteEdit => "ESC: Back | ^S: Save | ^H: Highlight",
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