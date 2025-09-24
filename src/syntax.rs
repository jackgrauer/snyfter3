// Syntax highlighting using syntect directly (bat wraps syntect)

use syntect::highlighting::{Style, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::easy::HighlightLines;
use anyhow::Result;

pub struct SyntaxHighlighter {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
}

impl SyntaxHighlighter {
    pub fn new() -> Result<Self> {
        Ok(Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
        })
    }

    /// Get syntax highlighted lines for display in the editor
    pub fn highlight_lines(&self, text: &str, file_extension: &str) -> Vec<Vec<(Style, String)>> {
        let syntax = self.syntax_set
            .find_syntax_by_extension(file_extension)
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());

        let theme = &self.theme_set.themes["Monokai"];

        let mut highlighter = HighlightLines::new(syntax, theme);
        let mut highlighted_lines = Vec::new();

        for line in text.lines() {
            if let Ok(highlighted) = highlighter.highlight_line(line, &self.syntax_set) {
                // Convert &str to String
                let owned_highlighted: Vec<(Style, String)> = highlighted
                    .into_iter()
                    .map(|(style, text)| (style, text.to_string()))
                    .collect();
                highlighted_lines.push(owned_highlighted);
            } else {
                // Fallback to plain text
                highlighted_lines.push(vec![(Style::default(), line.to_string())]);
            }
        }

        highlighted_lines
    }

    /// Get a simple highlighted version for terminal display
    pub fn get_highlighted_text(&self, text: &str, _file_extension: &str) -> String {
        // For now, just return the text as-is
        text.to_string()
    }
}

impl Default for SyntaxHighlighter {
    fn default() -> Self {
        Self::new().expect("Failed to initialize syntax highlighter")
    }
}