// Snyfter3 - Fast note-taking and qualitative data analysis app
// NValt-like interface with QualCoder-style highlighting

use anyhow::Result;
use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind, MouseButton, EnableMouseCapture, DisableMouseCapture},
    execute, terminal,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use chrono;
use nucleo::{Matcher, Utf32Str, pattern::{Pattern, CaseMatching, Normalization}};

mod note_store;
mod search_engine;
mod ui;
mod qda_codes;  // Qualitative data analysis codes/tags
mod editor;
mod edit_renderer;
mod block_selection;
mod markdown;
mod templates;
mod syntax;

use note_store::{Note, NoteStore};
use search_engine::SearchEngine;
use ui::UI;
use qda_codes::CodeManager;
use editor::TextEditor;
use templates::TemplateManager;

#[derive(Parser, Debug)]
#[command(name = "snyfter3", author, version, about)]
struct Args {
    /// Directory to store notes (defaults to ~/Documents/Snyfter3)
    #[arg(long)]
    notes_dir: Option<PathBuf>,

    /// Open with a search query
    #[arg(short, long)]
    search: Option<String>,
}

// Single unified mode - no mode switching needed

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FocusArea {
    SearchBar,
    NoteList,
    Editor,
}

pub struct App {
    // Core components
    notes: NoteStore,
    search: SearchEngine,
    codes: CodeManager,
    ui: UI,
    editor: TextEditor,
    templates: TemplateManager,

    // All notes and filtering
    all_notes: Vec<Note>,  // All notes to search through

    // Current state
    selected_note: Option<Note>,
    selected_note_index: usize,
    search_query: String,
    filtered_notes: Vec<Note>,  // Notes matching current search
    focus_area: FocusArea,  // Which area currently has focus

    // Display state
    needs_redraw: bool,
    exit_requested: bool,
    status_message: String,

    // Split pane position (percentage of screen width for note list)
    split_ratio: f32,  // 0.2 = 20% width for list, 80% for editor
    dragging_divider: bool,  // Whether we're currently dragging the divider

    // Cursor acceleration
    last_arrow_key: Option<KeyCode>,
    arrow_key_count: usize,
    last_arrow_time: Option<Instant>,
}

// Arrow key acceleration helper
fn update_arrow_acceleration(app: &mut App, key: KeyCode) -> usize {
    let now = Instant::now();

    // Check if it's the same arrow key being held
    if let Some(last_key) = app.last_arrow_key {
        if last_key == key {
            // Check if it's within the acceleration window (300ms)
            if let Some(last_time) = app.last_arrow_time {
                if now.duration_since(last_time) < Duration::from_millis(300) {
                    app.arrow_key_count += 1;
                } else {
                    // Reset if too much time has passed
                    app.arrow_key_count = 1;
                }
            }
        } else {
            // Different arrow key, reset counter
            app.arrow_key_count = 1;
        }
    } else {
        // First arrow press
        app.arrow_key_count = 1;
    }

    app.last_arrow_key = Some(key);
    app.last_arrow_time = Some(now);

    // Calculate acceleration based on key count
    match app.arrow_key_count {
        1..=3 => 1,           // Normal speed for first few presses
        4..=8 => 3,           // 3x speed after holding briefly
        9..=15 => 6,          // 6x speed for sustained holding
        _ => 10,              // Max 10x speed for long holds
    }
}

impl App {
    pub fn new(notes_dir: PathBuf) -> Result<Self> {
        let notes = NoteStore::new(&notes_dir)?;
        let search = SearchEngine::new(&notes_dir)?;
        let codes = CodeManager::new(&notes_dir)?;
        let ui = UI::new()?;

        // Load initial notes
        let all_notes = notes.get_all_notes()?;

        // Index all notes in search engine
        for note in &all_notes {
            search.index_note(&note.id, &note.title, &note.content, &note.tags)?;
        }

        let filtered_notes = all_notes.clone();

        Ok(App {
            notes,
            search,
            codes,
            ui,
            editor: TextEditor::new(),
            templates: TemplateManager::new(),
            all_notes,
            selected_note: None,
            selected_note_index: 0,
            search_query: String::new(),
            filtered_notes,
            focus_area: FocusArea::NoteList,  // Start with note list focused
            needs_redraw: true,
            exit_requested: false,
            status_message: String::from("Welcome to Snyfter3!"),
            split_ratio: 0.2,  // Start with narrower notes list
            dragging_divider: false,
            last_arrow_key: None,
            arrow_key_count: 0,
            last_arrow_time: None,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

        // Main event loop
        while !self.exit_requested {
            // Render
            if self.needs_redraw {
                self.render()?;
                self.needs_redraw = false;
            }

            // Handle input
            if event::poll(std::time::Duration::from_millis(50))? {
                match event::read()? {
                    Event::Key(key) => self.handle_key(key).await?,
                    Event::Mouse(mouse) => self.handle_mouse(mouse)?,
                    Event::Resize(_, _) => self.needs_redraw = true,
                    _ => {}
                }
            }
        }

        // Cleanup
        execute!(stdout, DisableMouseCapture, LeaveAlternateScreen)?;
        disable_raw_mode()?;

        Ok(())
    }

    fn render(&mut self) -> Result<()> {
        // Extract necessary references before calling render
        let mut ui = std::mem::replace(&mut self.ui, UI::new()?);
        ui.render(self)?;
        self.ui = ui;
        io::stdout().flush()?;
        Ok(())
    }

    async fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        // Handle Ctrl+Q to quit from anywhere
        if key.code == KeyCode::Char('q') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.exit_requested = true;
            return Ok(());
        }

        // Handle Ctrl+L or Cmd+L to focus search bar (NValt style)
        if key.code == KeyCode::Char('l') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.focus_area = FocusArea::SearchBar;
            self.needs_redraw = true;
            return Ok(());
        }

        // Tab cycles through focus areas
        if key.code == KeyCode::Tab {
            self.focus_area = match self.focus_area {
                FocusArea::SearchBar => FocusArea::NoteList,
                FocusArea::NoteList => FocusArea::Editor,
                FocusArea::Editor => FocusArea::SearchBar,
            };
            self.needs_redraw = true;
            return Ok(());
        }

        // Handle input based on focus area
        match self.focus_area {
            FocusArea::SearchBar => self.handle_search_key(key).await?,
            FocusArea::NoteList => self.handle_list_key(key).await?,
            FocusArea::Editor => self.handle_editor_key(key).await?,
        }

        self.needs_redraw = true;
        Ok(())
    }

    async fn handle_search_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Down => {
                // Exit search focus, move to notes list
                self.focus_area = FocusArea::NoteList;
                if !self.filtered_notes.is_empty() && key.code == KeyCode::Enter {
                    self.selected_note_index = 0;
                    self.load_selected_note()?;
                }
            }
            KeyCode::Backspace => {
                self.search_query.pop();
                self.update_search()?;
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.search_query.push(c);
                self.update_search()?;
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_list_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('/') | KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Focus search bar
                self.focus_area = FocusArea::SearchBar;
                // Don't clear - allow incremental search
            }
            KeyCode::Enter | KeyCode::Right => {
                // Move focus to editor
                if self.selected_note.is_some() {
                    self.focus_area = FocusArea::Editor;
                }
            }
            KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Create new note
                self.create_new_note()?;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected_note_index > 0 {
                    self.selected_note_index -= 1;
                    self.load_selected_note()?;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected_note_index < self.filtered_notes.len().saturating_sub(1) {
                    self.selected_note_index += 1;
                    self.load_selected_note()?;
                }
            }
            KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Follow wiki link under cursor
                self.follow_wiki_link()?;
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Delete selected note
                self.delete_selected_note()?;
            }
            // Resize panes with keyboard
            KeyCode::Char(',') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Make notes pane smaller
                self.split_ratio = (self.split_ratio - 0.05).max(0.1);  // Allow down to 10%
                self.needs_redraw = true;
            }
            KeyCode::Char('.') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Make notes pane larger
                self.split_ratio = (self.split_ratio + 0.05).min(0.7);
                self.needs_redraw = true;
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_editor_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                // Go back to note list
                self.focus_area = FocusArea::NoteList;
            }
            // Arrow keys with acceleration
            KeyCode::Left | KeyCode::Right | KeyCode::Up | KeyCode::Down
                if key.modifiers.is_empty() => {
                // Calculate acceleration for arrow keys
                let acceleration = update_arrow_acceleration(self, key.code);

                if self.selected_note.is_some() {
                    if self.editor.handle_key_with_acceleration(key.code, key.modifiers, acceleration)? {
                        // Auto-save immediately after any modification
                        self.auto_save_current_note()?;
                    }
                }
            }
            // All other keys are passed to the editor for text editing
            _ => {
                if self.selected_note.is_some() {
                    if self.editor.handle_key(key.code, key.modifiers)? {
                        // Auto-save immediately after any modification
                        self.auto_save_current_note()?;
                    }
                }
            }
        }
        Ok(())
    }


    fn apply_template(&mut self, template_name: &str) -> Result<()> {
        // Create new note with template
        // Auto-save handles saving

        let title = format!("Note {}", chrono::Utc::now().format("%Y-%m-%d %H:%M"));

        // Get template content
        let mut vars = std::collections::HashMap::new();
        vars.insert("TITLE".to_string(), title.clone());
        vars.insert("PROJECT_NAME".to_string(), "My Project".to_string());
        vars.insert("TOPIC".to_string(), "Research Topic".to_string());
        vars.insert("AUTHOR".to_string(), "Author Name".to_string());
        vars.insert("LANGUAGE".to_string(), "rust".to_string());
        vars.insert("FIELD".to_string(), "field".to_string());
        vars.insert("GENRE".to_string(), "genre".to_string());

        let content = self.templates.apply_template(template_name, vars)?;

        let note = self.notes.create_note(&title, &content)?;

        // Index in search engine
        self.search.index_note(&note.id, &note.title, &note.content, &note.tags)?;

        self.selected_note = Some(note);
        self.editor.set_text(&content);
        self.status_message = format!("Created new note from {} template", template_name);
        Ok(())
    }

    fn create_new_note(&mut self) -> Result<()> {
        // Auto-save handles saving

        let title = format!("Note {}", chrono::Utc::now().format("%Y-%m-%d %H:%M"));
        let note = self.notes.create_note(&title, "")?;

        // Index in search engine
        self.search.index_note(&note.id, &note.title, &note.content, &note.tags)?;

        // Add to all_notes and update filtered
        self.all_notes.push(note.clone());
        self.update_search()?;

        // Select the new note
        self.selected_note = Some(note);
        self.editor.set_text("");
        self.status_message = "New note created".to_string();
        Ok(())
    }

    fn load_selected_note(&mut self) -> Result<()> {
        // Auto-save handles saving

        // Get note from filtered results
        if self.selected_note_index < self.filtered_notes.len() {
            let note = self.filtered_notes[self.selected_note_index].clone();
            self.selected_note = Some(note.clone());
            self.editor.set_text(&note.content);
        }
        Ok(())
    }

    fn update_search(&mut self) -> Result<()> {
        if self.search_query.is_empty() {
            // Show all notes when search is empty
            self.filtered_notes = self.all_notes.clone();
        } else {
            // Use nucleo for fuzzy search
            let pattern = Pattern::parse(
                &self.search_query,
                CaseMatching::Ignore,
                Normalization::Smart,
            );

            let mut matcher = Matcher::default();
            let mut matches = Vec::new();
            let mut buf = Vec::new();

            for note in &self.all_notes {
                let haystack = format!("{} {} {}", note.title, note.content, note.tags.join(" "));
                buf.clear();
                let score = pattern.score(Utf32Str::new(&haystack, &mut buf), &mut matcher);
                if let Some(score) = score {
                    matches.push((score, note.clone()));
                }
            }

            // Sort by score (highest first)
            matches.sort_by(|a, b| b.0.cmp(&a.0));
            self.filtered_notes = matches.into_iter().map(|(_, note)| note).collect();
        }

        // Reset selection if needed
        if self.selected_note_index >= self.filtered_notes.len() {
            self.selected_note_index = 0;
        }

        // Load the first result if any
        if !self.filtered_notes.is_empty() {
            self.load_selected_note()?;
        }

        self.status_message = format!("{} notes", self.filtered_notes.len());
        Ok(())
    }

    fn auto_save_current_note(&mut self) -> Result<()> {
        if let Some(mut note) = self.selected_note.take() {
            note.content = self.editor.get_text();

            // Extract tags from content
            use crate::markdown::MarkdownRenderer;
            note.tags = MarkdownRenderer::extract_tags(&note.content);

            self.notes.update_note(&note)?;

            // Update search index
            self.search.index_note(&note.id, &note.title, &note.content, &note.tags)?;

            self.selected_note = Some(note);
        }
        Ok(())
    }

    fn follow_wiki_link(&mut self) -> Result<()> {
        if let Some(ref _note) = self.selected_note {
            // Get current cursor position and find wiki link under cursor
            let text = self.editor.get_text();
            let _cursor_pos = self.editor.get_cursor_position();

            // Simple approach: find all wiki links and check if cursor is within one
            use crate::markdown::MarkdownRenderer;
            let links = MarkdownRenderer::extract_wiki_links(&text);

            // Find if cursor is within a wiki link (simplified for now)

            // Search for the link at cursor position (simplified for now)
            for link_title in links {
                // Search for a note with this title
                let all_notes = self.notes.get_all_notes()?;
                for (idx, note) in all_notes.iter().enumerate() {
                    if note.title == link_title {
                        self.selected_note_index = idx;
                        self.load_selected_note()?;
                        self.status_message = format!("Navigated to: {}", link_title);
                        return Ok(());
                    }
                }

                // If not found, create a new note with this title
                let new_note = self.notes.create_note(&link_title, "")?;
                self.search.index_note(&new_note.id, &new_note.title, &new_note.content, &new_note.tags)?;
                self.selected_note = Some(new_note);
                self.editor.set_text("");
                self.status_message = format!("Created new note: {}", link_title);
                return Ok(());
            }
        }
        Ok(())
    }

    fn handle_mouse(&mut self, mouse: MouseEvent) -> Result<()> {
        let (term_width, _term_height) = terminal::size()?;
        let divider_x = (term_width as f32 * self.split_ratio) as u16;

        // Check if Alt is being held for block selection
        let is_alt_held = mouse.modifiers.contains(crossterm::event::KeyModifiers::ALT);

        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                // Check if clicking on search bar (line 2)
                if mouse.row == 1 {
                    self.focus_area = FocusArea::SearchBar;
                    self.needs_redraw = true;
                }
                // Check if clicking on divider (within 2 pixels)
                else if mouse.column >= divider_x.saturating_sub(1) && mouse.column <= divider_x + 1 {
                    self.dragging_divider = true;
                } else if mouse.column < divider_x {
                    // Clicking in notes list area
                    if mouse.row == 2 {
                        // Clicking on notes header
                        self.focus_area = FocusArea::NoteList;
                        self.needs_redraw = true;
                    } else if mouse.row > 2 {  // Skip header and search bar
                        let index = (mouse.row - 3) as usize;
                        if index < self.filtered_notes.len() {
                            self.selected_note_index = index;
                            self.load_selected_note()?;
                            self.focus_area = FocusArea::NoteList;
                            self.needs_redraw = true;
                        }
                    }
                } else {
                    // Clicking in editor area
                    if self.selected_note.is_some() {
                        self.focus_area = FocusArea::Editor;

                        // Calculate the click position relative to the editor panel
                        let editor_start_x = divider_x + 1;
                        let editor_start_y = 3; // After search bar and editor header

                        if mouse.column >= editor_start_x && mouse.row >= editor_start_y {
                            // Convert screen coordinates to editor coordinates
                            let click_col = (mouse.column - editor_start_x) as usize;
                            let click_row = (mouse.row - editor_start_y) as usize;

                            // Clear any existing block selection on new click
                            self.editor.block_selection = None;

                            // Set cursor position
                            self.editor.set_cursor_position(click_row, click_col);

                            // Store the click position for potential block selection on drag
                            self.editor.potential_block_start = Some((click_row, click_col));
                        }

                        self.needs_redraw = true;
                    }
                }
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                if self.dragging_divider {
                    // Update split ratio based on mouse position
                    self.split_ratio = (mouse.column as f32 / term_width as f32)
                        .max(0.1)  // Allow down to 10%
                        .min(0.7);
                    self.needs_redraw = true;
                } else if mouse.column > divider_x {
                    // Handle dragging in editor area
                    let editor_start_x = divider_x + 1;
                    let editor_start_y = 3;

                    if mouse.column >= editor_start_x && mouse.row >= editor_start_y {
                        let drag_col = (mouse.column - editor_start_x) as usize;
                        let drag_row = (mouse.row - editor_start_y) as usize;

                        // Create block selection on first drag if we have a start position
                        if self.editor.block_selection.is_none() {
                            if let Some((start_row, start_col)) = self.editor.potential_block_start {
                                // Only create block selection if we've actually moved
                                if drag_row != start_row || drag_col != start_col {
                                    self.editor.block_selection = Some(crate::block_selection::BlockSelection::new(start_row, start_col));
                                    if let Some(block_sel) = &mut self.editor.block_selection {
                                        block_sel.anchor_visual_col = start_col;
                                        block_sel.cursor_visual_col = start_col;
                                    }
                                }
                            }
                        }

                        // Extend existing block selection
                        if let Some(block_sel) = &mut self.editor.block_selection {
                            block_sel.extend_to(drag_row, drag_col, drag_col);
                        }

                        self.needs_redraw = true;
                    }
                }
            }
            MouseEventKind::Up(MouseButton::Left) => {
                self.dragging_divider = false;

                // Clear the potential block start since mouse is released
                self.editor.potential_block_start = None;

                // Check if block selection exists and hasn't been extended (just a click, no drag)
                if let Some(ref block_sel) = self.editor.block_selection {
                    // If anchor and cursor are at the same position, it was just a click
                    if block_sel.anchor.line == block_sel.cursor.line
                        && block_sel.anchor.column == block_sel.cursor.column {
                        // Clear the block selection since no drag occurred
                        self.editor.block_selection = None;
                    }
                }
            }
            MouseEventKind::ScrollDown => {
                // Scroll notes list down
                if self.selected_note_index < self.filtered_notes.len().saturating_sub(1) {
                    self.selected_note_index += 1;
                    self.load_selected_note()?;
                    self.needs_redraw = true;
                }
            }
            MouseEventKind::ScrollUp => {
                // Scroll notes list up
                if self.selected_note_index > 0 {
                    self.selected_note_index -= 1;
                    self.load_selected_note()?;
                    self.needs_redraw = true;
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn delete_selected_note(&mut self) -> Result<()> {
        if self.selected_note_index < self.filtered_notes.len() {
            let note = &self.filtered_notes[self.selected_note_index];
            let id = note.id.clone();

            // Delete from storage
            self.notes.delete_note(&id)?;
            // Delete from search index
            self.search.delete_note(&id)?;

            // Remove from all_notes and filtered_notes
            self.all_notes.retain(|n| n.id != id);
            self.filtered_notes.retain(|n| n.id != id);

            // Adjust selected index if needed
            if self.selected_note_index >= self.filtered_notes.len() && self.selected_note_index > 0 {
                self.selected_note_index -= 1;
            }

            // Clear selected note if it was the deleted one
            if let Some(ref selected) = self.selected_note {
                if selected.id == id {
                    self.selected_note = None;
                    self.editor.set_text("");
                }
            }

            // Update search results
            self.update_search()?;

            self.status_message = "Note deleted".to_string();
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Set up notes directory
    let notes_dir = args.notes_dir.unwrap_or_else(|| {
        dirs::document_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Snyfter3")
    });

    // Create directory if it doesn't exist
    std::fs::create_dir_all(&notes_dir)?;

    // Initialize and run app
    let mut app = App::new(notes_dir)?;

    // If search query provided, start with search
    if let Some(query) = args.search {
        app.search_query = query;
        app.focus_area = FocusArea::SearchBar;
        app.update_search()?;
    }

    // Load first note if any
    if !app.filtered_notes.is_empty() {
        app.load_selected_note()?;
    }

    app.run().await?;

    Ok(())
}