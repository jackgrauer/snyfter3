# Snyfter3 - Note-Taking and Qualitative Data Analysis

A fast, keyboard-driven note-taking application inspired by NValt with QualCoder-style qualitative data analysis features. Built entirely in Rust for maximum performance.

## Features

- **NValt-like Interface**: Split-pane design with instant search and note editing
- **Live Search**: Search-as-you-type with full-text search using Tantivy
- **Fuzzy Matching**: Find notes even with typos or partial matches
- **Helix Editor Integration**: Advanced text editing with modal operations
- **Qualitative Coding**: Apply codes/tags to text segments for analysis
- **SQLite Storage**: Persistent, reliable note storage
- **Keyboard-Driven**: Complete keyboard control for maximum efficiency

## Installation

```bash
# Clone the repository
git clone https://github.com/yourusername/snyfter3
cd snyfter3

# Build release version
cargo build --release

# Run the application
./target/release/snyfter3
```

## Usage

### Starting Snyfter3

```bash
# Run with default settings (stores notes in ~/Documents/Snyfter3)
snyfter3

# Specify custom notes directory
snyfter3 --notes-dir ~/MyNotes

# Start with search query
snyfter3 --search "important"
```

### Keyboard Shortcuts

#### Note List Mode
- `↑/↓` or `j/k` - Navigate notes
- `Enter` or `Tab` - Edit selected note
- `Ctrl+N` - Create new note
- `Ctrl+F` or `/` - Start search
- `Ctrl+T` - Open code/tag manager
- `Ctrl+Q` - Quit

#### Edit Mode
- `Esc` - Save and return to note list
- `Ctrl+S` - Save note
- `Ctrl+H` - Start highlighting for coding
- Standard text editing keys (arrows, Home, End, etc.)
- `Ctrl+←/→` - Move by word
- `Ctrl+A` - Select all

#### Search Mode
- Type to search live (instant results)
- `Enter` - Execute search
- `Esc` - Cancel search

#### Code Manager Mode
- `n` - Create new code
- `Esc` or `q` - Return to note list

## Architecture

### Core Modules

- **`main.rs`**: Application entry point and state management
- **`note_store.rs`**: SQLite database for note persistence
- **`search_engine.rs`**: Tantivy full-text search integration
- **`editor.rs`**: Helix-based text editor implementation
- **`ui.rs`**: Terminal UI rendering with split-pane layout
- **`qda_codes.rs`**: Qualitative data analysis coding system

### Data Storage

Notes are stored in SQLite with the following structure:
- Unique ID (SHA256 hash)
- Title
- Content
- Creation/modification timestamps
- Tags
- Coded segments

Search indices are maintained separately using Tantivy for lightning-fast full-text search.

## Qualitative Coding

Snyfter3 includes 8 default qualitative codes:
- **Theme** - Major themes or patterns
- **Concept** - Key concepts or ideas
- **Question** - Research questions
- **Insight** - Important findings
- **To Do** - Action items
- **Quote** - Notable quotations
- **Reference** - Citations
- **Method** - Methodological notes

Codes can be applied to any text segment and include optional memos for additional context.

## Technology Stack

- **Rust** - Core language for performance and reliability
- **Helix-core** - Advanced text editing capabilities
- **Tantivy** - Full-text search engine (like Lucene)
- **SQLite** - Persistent storage with FTS5
- **Crossterm** - Terminal UI rendering
- **Fuzzy-matcher** - Fuzzy search implementation

## Development

### Building from Source

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Run tests
cargo test
```

### Project Structure

```
snyfter3/
├── src/
│   ├── main.rs         # Application entry point
│   ├── note_store.rs   # Note storage layer
│   ├── search_engine.rs # Search functionality
│   ├── editor.rs       # Text editor
│   ├── ui.rs          # Terminal UI
│   └── qda_codes.rs   # Coding system
├── Cargo.toml         # Dependencies
└── README.md         # This file
```

## Future Enhancements

- [ ] File watcher for auto-sync
- [ ] Export to various formats (Markdown, HTML, PDF)
- [ ] Cloud sync support
- [ ] Multi-window support
- [ ] Plugin system for extensions
- [ ] Collaborative editing
- [ ] Advanced code visualization
- [ ] Statistical analysis of codes

## License

MIT License - See LICENSE file for details

## Contributing

Contributions are welcome! Please feel free to submit pull requests or open issues for bugs and feature requests.

## Acknowledgments

- Inspired by [NValt](http://brettterpstra.com/projects/nvalt/) for the interface design
- [QualCoder](https://github.com/ccbogel/QualCoder) for qualitative analysis concepts
- [Helix Editor](https://helix-editor.com/) for text editing capabilities