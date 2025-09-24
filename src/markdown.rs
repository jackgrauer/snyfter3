// Markdown parsing and rendering for notes

use std::collections::HashMap;
use regex::Regex;
use crossterm::style::Color;

#[derive(Debug, Clone)]
pub struct MarkdownRenderer {
    // Color scheme for different elements
    colors: HashMap<String, Color>,
}

#[derive(Debug, Clone)]
pub struct FormattedLine {
    #[allow(dead_code)]
    pub text: String,
    pub segments: Vec<TextSegment>,
}

#[derive(Debug, Clone)]
pub struct TextSegment {
    pub start: usize,
    pub end: usize,
    pub style: TextStyle,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextStyle {
    pub color: Color,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub is_link: bool,
    pub link_target: Option<String>,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            color: Color::Rgb { r: 200, g: 200, b: 200 },
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
            is_link: false,
            link_target: None,
        }
    }
}

impl MarkdownRenderer {
    pub fn new() -> Self {
        let mut colors = HashMap::new();

        // Define color scheme
        colors.insert("heading1".to_string(), Color::Rgb { r: 255, g: 180, b: 100 });
        colors.insert("heading2".to_string(), Color::Rgb { r: 255, g: 160, b: 120 });
        colors.insert("heading3".to_string(), Color::Rgb { r: 255, g: 140, b: 140 });
        colors.insert("bold".to_string(), Color::Rgb { r: 255, g: 255, b: 255 });
        colors.insert("italic".to_string(), Color::Rgb { r: 200, g: 200, b: 255 });
        colors.insert("code".to_string(), Color::Rgb { r: 150, g: 255, b: 150 });
        colors.insert("code_block".to_string(), Color::Rgb { r: 100, g: 200, b: 100 });
        colors.insert("link".to_string(), Color::Rgb { r: 100, g: 150, b: 255 });
        colors.insert("wiki_link".to_string(), Color::Rgb { r: 150, g: 200, b: 255 });
        colors.insert("tag".to_string(), Color::Rgb { r: 255, g: 200, b: 100 });
        colors.insert("task_pending".to_string(), Color::Rgb { r: 255, g: 150, b: 150 });
        colors.insert("task_done".to_string(), Color::Rgb { r: 150, g: 255, b: 150 });
        colors.insert("blockquote".to_string(), Color::Rgb { r: 150, g: 150, b: 200 });
        colors.insert("list_marker".to_string(), Color::Rgb { r: 200, g: 150, b: 100 });

        Self { colors }
    }

    pub fn render_line(&self, line: &str) -> FormattedLine {
        let mut segments = Vec::new();

        // Check for special patterns first

        // Headers
        if let Some(level) = Self::detect_header(line) {
            let color = match level {
                1 => self.colors.get("heading1"),
                2 => self.colors.get("heading2"),
                _ => self.colors.get("heading3"),
            }.cloned().unwrap_or(Color::Rgb { r: 200, g: 200, b: 200 });

            segments.push(TextSegment {
                start: 0,
                end: line.len(),
                style: TextStyle {
                    color,
                    bold: true,
                    ..Default::default()
                },
            });
        }
        // Task lists
        else if line.starts_with("- [ ]") || line.starts_with("* [ ]") {
            segments.push(TextSegment {
                start: 0,
                end: 5,
                style: TextStyle {
                    color: self.colors.get("task_pending").cloned().unwrap_or(Color::Rgb { r: 200, g: 200, b: 200 }),
                    ..Default::default()
                },
            });
        }
        else if line.starts_with("- [x]") || line.starts_with("* [x]") || line.starts_with("- [X]") || line.starts_with("* [X]") {
            segments.push(TextSegment {
                start: 0,
                end: 5,
                style: TextStyle {
                    color: self.colors.get("task_done").cloned().unwrap_or(Color::Rgb { r: 200, g: 200, b: 200 }),
                    strikethrough: true,
                    ..Default::default()
                },
            });
        }
        // Lists
        else if line.starts_with("- ") || line.starts_with("* ") || line.starts_with("+ ") {
            segments.push(TextSegment {
                start: 0,
                end: 2,
                style: TextStyle {
                    color: self.colors.get("list_marker").cloned().unwrap_or(Color::Rgb { r: 200, g: 200, b: 200 }),
                    bold: true,
                    ..Default::default()
                },
            });
        }
        // Blockquotes
        else if line.starts_with("> ") {
            segments.push(TextSegment {
                start: 0,
                end: line.len(),
                style: TextStyle {
                    color: self.colors.get("blockquote").cloned().unwrap_or(Color::Rgb { r: 200, g: 200, b: 200 }),
                    italic: true,
                    ..Default::default()
                },
            });
        }

        // Find inline patterns
        segments.extend(self.find_inline_patterns(line));

        // Sort segments by start position
        segments.sort_by_key(|s| s.start);

        FormattedLine {
            text: line.to_string(),
            segments,
        }
    }

    fn detect_header(line: &str) -> Option<usize> {
        if line.starts_with("### ") {
            Some(3)
        } else if line.starts_with("## ") {
            Some(2)
        } else if line.starts_with("# ") {
            Some(1)
        } else {
            None
        }
    }

    fn find_inline_patterns(&self, line: &str) -> Vec<TextSegment> {
        let mut segments = Vec::new();

        // Wiki-style links [[Note Title]]
        let wiki_link_re = Regex::new(r"\[\[([^\]]+)\]\]").unwrap();
        for cap in wiki_link_re.captures_iter(line) {
            if let Some(m) = cap.get(0) {
                segments.push(TextSegment {
                    start: m.start(),
                    end: m.end(),
                    style: TextStyle {
                        color: self.colors.get("wiki_link").cloned().unwrap_or(Color::Rgb { r: 200, g: 200, b: 200 }),
                        underline: true,
                        is_link: true,
                        link_target: cap.get(1).map(|s| s.as_str().to_string()),
                        ..Default::default()
                    },
                });
            }
        }

        // Tags #tag
        let tag_re = Regex::new(r"#([a-zA-Z0-9_-]+)").unwrap();
        for cap in tag_re.captures_iter(line) {
            if let Some(m) = cap.get(0) {
                segments.push(TextSegment {
                    start: m.start(),
                    end: m.end(),
                    style: TextStyle {
                        color: self.colors.get("tag").cloned().unwrap_or(Color::Rgb { r: 200, g: 200, b: 200 }),
                        bold: true,
                        ..Default::default()
                    },
                });
            }
        }

        // Bold **text**
        let bold_re = Regex::new(r"\*\*([^\*]+)\*\*").unwrap();
        for cap in bold_re.captures_iter(line) {
            if let Some(m) = cap.get(0) {
                segments.push(TextSegment {
                    start: m.start(),
                    end: m.end(),
                    style: TextStyle {
                        color: self.colors.get("bold").cloned().unwrap_or(Color::Rgb { r: 200, g: 200, b: 200 }),
                        bold: true,
                        ..Default::default()
                    },
                });
            }
        }

        // Italic *text* or _text_
        let italic_re = Regex::new(r"\*([^\*]+)\*|_([^_]+)_").unwrap();
        for cap in italic_re.captures_iter(line) {
            if let Some(m) = cap.get(0) {
                segments.push(TextSegment {
                    start: m.start(),
                    end: m.end(),
                    style: TextStyle {
                        color: self.colors.get("italic").cloned().unwrap_or(Color::Rgb { r: 200, g: 200, b: 200 }),
                        italic: true,
                        ..Default::default()
                    },
                });
            }
        }

        // Inline code `code`
        let code_re = Regex::new(r"`([^`]+)`").unwrap();
        for cap in code_re.captures_iter(line) {
            if let Some(m) = cap.get(0) {
                segments.push(TextSegment {
                    start: m.start(),
                    end: m.end(),
                    style: TextStyle {
                        color: self.colors.get("code").cloned().unwrap_or(Color::Rgb { r: 200, g: 200, b: 200 }),
                        ..Default::default()
                    },
                });
            }
        }

        // Markdown links [text](url)
        let link_re = Regex::new(r"\[([^\]]+)\]\(([^)]+)\)").unwrap();
        for cap in link_re.captures_iter(line) {
            if let Some(m) = cap.get(0) {
                segments.push(TextSegment {
                    start: m.start(),
                    end: m.end(),
                    style: TextStyle {
                        color: self.colors.get("link").cloned().unwrap_or(Color::Rgb { r: 200, g: 200, b: 200 }),
                        underline: true,
                        is_link: true,
                        link_target: cap.get(2).map(|s| s.as_str().to_string()),
                        ..Default::default()
                    },
                });
            }
        }

        segments
    }

    pub fn extract_tags(text: &str) -> Vec<String> {
        let tag_re = Regex::new(r"#([a-zA-Z0-9_-]+)").unwrap();
        tag_re.captures_iter(text)
            .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
            .collect()
    }

    pub fn extract_wiki_links(text: &str) -> Vec<String> {
        let wiki_link_re = Regex::new(r"\[\[([^\]]+)\]\]").unwrap();
        wiki_link_re.captures_iter(text)
            .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
            .collect()
    }

    #[allow(dead_code)]
    pub fn parse_front_matter(text: &str) -> Option<HashMap<String, String>> {
        if !text.starts_with("---\n") {
            return None;
        }

        let end = text[4..].find("\n---\n")?;
        let front_matter = &text[4..4 + end];

        let mut metadata = HashMap::new();
        for line in front_matter.lines() {
            if let Some(colon) = line.find(':') {
                let key = line[..colon].trim().to_string();
                let value = line[colon + 1..].trim().to_string();
                metadata.insert(key, value);
            }
        }

        Some(metadata)
    }
}