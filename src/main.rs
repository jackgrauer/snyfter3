// Snyfter3 - Fast note-taking and qualitative data analysis app
// NValt-like interface with QualCoder-style highlighting

use anyhow::Result;
use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io::{self, Write};
use std::path::PathBuf;
use chrono;

mod note_store;
mod search_engine;
mod ui;
mod qda_codes;  // Qualitative data analysis codes/tags
mod editor;

use note_store::{Note, NoteStore};
use search_engine::{SearchEngine, SearchResult};
use ui::UI;
use qda_codes::CodeManager;
use editor::TextEditor;

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

#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    NoteList,      // Browsing/searching notes (top pane)
    NoteEdit,      // Editing selected note (bottom pane)
    Search,        // Typing in search box
    CodeManager,   // Managing qualitative codes/tags
    Highlighting,  // Selecting text to code
}

pub struct App {
    // Core components
    notes: NoteStore,
    search: SearchEngine,
    codes: CodeManager,
    ui: UI,
    editor: TextEditor,

    // Current state
    mode: AppMode,
    selected_note: Option<Note>,
    selected_note_index: usize,
    search_query: String,
    search_results: Vec<SearchResult>,

    // Display state
    needs_redraw: bool,
    exit_requested: bool,
    status_message: String,
    unsaved_changes: bool,

    // Split pane position (percentage of screen for note list)
    split_ratio: f32,  // 0.3 = 30% for list, 70% for editor
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

        Ok(App {
            notes,
            search,
            codes,
            ui,
            editor: TextEditor::new(),
            mode: AppMode::NoteList,
            selected_note: None,
            selected_note_index: 0,
            search_query: String::new(),
            search_results: Vec::new(),
            needs_redraw: true,
            exit_requested: false,
            status_message: String::from("Welcome to Snyfter3!"),
            unsaved_changes: false,
            split_ratio: 0.3,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;

        // Main event loop
        while !self.exit_requested {
            // Render
            if self.needs_redraw {
                self.render()?;
                self.needs_redraw = false;
            }

            // Handle input
            if event::poll(std::time::Duration::from_millis(50))? {
                if let Event::Key(key) = event::read()? {
                    self.handle_key(key).await?;
                }
            }
        }

        // Cleanup
        disable_raw_mode()?;
        execute!(stdout, LeaveAlternateScreen)?;

        Ok(())
    }

    fn render(&mut self) -> Result<()> {
        self.ui.render(self)?;
        io::stdout().flush()?;
        Ok(())
    }

    async fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        match self.mode {
            AppMode::Search => self.handle_search_key(key).await?,
            AppMode::NoteList => self.handle_list_key(key).await?,
            AppMode::NoteEdit => self.handle_edit_key(key).await?,
            AppMode::CodeManager => self.handle_code_key(key).await?,
            AppMode::Highlighting => self.handle_highlight_key(key).await?,
        }

        self.needs_redraw = true;
        Ok(())
    }

    async fn handle_search_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.mode = AppMode::NoteList;
            }
            KeyCode::Enter => {
                // Perform search
                self.search_results = self.search.search(&self.search_query)?;
                self.status_message = format!("Found {} notes", self.search_results.len());
                self.mode = AppMode::NoteList;
            }
            KeyCode::Backspace => {
                self.search_query.pop();
            }
            KeyCode::Char(c) => {
                self.search_query.push(c);
                // Live search as you type (like NValt)
                self.search_results = self.search.search(&self.search_query)?;
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_list_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.exit_requested = true;
            }
            KeyCode::Char('/') | KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.mode = AppMode::Search;
                self.search_query.clear();
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
                let note_count = self.notes.get_note_count();
                if self.selected_note_index < note_count.saturating_sub(1) {
                    self.selected_note_index += 1;
                    self.load_selected_note()?;
                }
            }
            KeyCode::Enter | KeyCode::Tab => {
                // Switch to editor
                self.mode = AppMode::NoteEdit;
            }
            KeyCode::Char('t') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Open code/tag manager
                self.mode = AppMode::CodeManager;
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Delete selected note
                self.delete_selected_note()?;
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_edit_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                // Save and go back to list
                self.save_current_note()?;
                self.mode = AppMode::NoteList;
            }
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Save
                self.save_current_note()?;
                self.status_message = "Note saved".to_string();
            }
            KeyCode::Char('h') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Start highlighting for coding
                self.mode = AppMode::Highlighting;
            }
            // Text editing keys handled by editor module
            _ => {
                if self.editor.handle_key(key.code, key.modifiers)? {
                    self.unsaved_changes = true;
                    self.status_message = "*modified*".to_string();
                }
            }
        }
        Ok(())
    }

    async fn handle_code_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.mode = AppMode::NoteList;
            }
            KeyCode::Char('n') => {
                // Create new code
                self.codes.create_code_interactive()?;
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_highlight_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.mode = AppMode::NoteEdit;
            }
            KeyCode::Enter => {
                // Apply selected code to highlighted text
                // ...
                self.mode = AppMode::NoteEdit;
            }
            _ => {}
        }
        Ok(())
    }

    fn create_new_note(&mut self) -> Result<()> {
        // Save current note if there are unsaved changes
        if self.unsaved_changes {
            self.save_current_note()?;
        }

        let title = format!("Note {}", chrono::Utc::now().format("%Y-%m-%d %H:%M"));
        let note = self.notes.create_note(&title, "")?;

        // Index in search engine
        self.search.index_note(&note.id, &note.title, &note.content, &note.tags)?;

        self.selected_note = Some(note);
        self.editor.set_text("");
        self.mode = AppMode::NoteEdit;
        self.unsaved_changes = false;
        self.status_message = "New note created".to_string();
        Ok(())
    }

    fn load_selected_note(&mut self) -> Result<()> {
        // Save current note if there are unsaved changes
        if self.unsaved_changes {
            self.save_current_note()?;
        }

        // Get note from search results if searching, otherwise from all notes
        let note = if !self.search_query.is_empty() && !self.search_results.is_empty() {
            if self.selected_note_index < self.search_results.len() {
                let result = &self.search_results[self.selected_note_index];
                self.notes.get_note(&result.id)?
            } else {
                None
            }
        } else {
            self.notes.get_note_by_index(self.selected_note_index)?
        };

        if let Some(note) = note {
            self.selected_note = Some(note.clone());
            self.editor.set_text(&note.content);
            self.unsaved_changes = false;
        }
        Ok(())
    }

    fn save_current_note(&mut self) -> Result<()> {
        if let Some(mut note) = self.selected_note.take() {
            note.content = self.editor.get_text();
            self.notes.update_note(&note)?;

            // Update search index
            self.search.index_note(&note.id, &note.title, &note.content, &note.tags)?;

            self.selected_note = Some(note);
            self.unsaved_changes = false;
            self.status_message = "Note saved".to_string();
        }
        Ok(())
    }

    fn delete_selected_note(&mut self) -> Result<()> {
        if !self.search_query.is_empty() && !self.search_results.is_empty() {
            if self.selected_note_index < self.search_results.len() {
                let result = &self.search_results[self.selected_note_index];
                let id = result.id.clone();

                // Delete from storage
                self.notes.delete_note(&id)?;
                // Delete from search index
                self.search.delete_note(&id)?;

                // Remove from search results
                self.search_results.remove(self.selected_note_index);

                // Adjust selected index if needed
                if self.selected_note_index >= self.search_results.len() && self.selected_note_index > 0 {
                    self.selected_note_index -= 1;
                }

                self.status_message = "Note deleted".to_string();
            }
        } else {
            // Delete from all notes
            let all_notes = self.notes.get_all_notes()?;
            if self.selected_note_index < all_notes.len() {
                let note = &all_notes[self.selected_note_index];
                let id = note.id.clone();

                // Delete from storage
                self.notes.delete_note(&id)?;
                // Delete from search index
                self.search.delete_note(&id)?;

                // Adjust selected index if needed
                let new_count = self.notes.get_note_count();
                if self.selected_note_index >= new_count && self.selected_note_index > 0 {
                    self.selected_note_index -= 1;
                }

                // Clear selected note if it was the deleted one
                if let Some(ref selected) = self.selected_note {
                    if selected.id == id {
                        self.selected_note = None;
                        self.editor.set_text("");
                    }
                }

                self.status_message = "Note deleted".to_string();
            }
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
        app.mode = AppMode::Search;
    }

    app.run().await?;

    Ok(())
}