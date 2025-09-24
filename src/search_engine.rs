// Full-text search engine using Tantivy for fast note searching

use anyhow::Result;
use tantivy::{
    schema::*,
    Index, IndexWriter,
    directory::MmapDirectory,
    query::QueryParser,
    collector::TopDocs,
    IndexReader,
    TantivyDocument,
};
use std::path::Path;
use std::sync::{Arc, Mutex};
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

pub struct SearchEngine {
    #[allow(dead_code)]
    index: Index,
    writer: Arc<Mutex<IndexWriter>>,
    reader: IndexReader,
    query_parser: QueryParser,
    #[allow(dead_code)]
    schema: Schema,
    title_field: Field,
    content_field: Field,
    id_field: Field,
    tags_field: Field,
}

impl SearchEngine {
    pub fn new(notes_dir: &Path) -> Result<Self> {
        // Build schema
        let mut schema_builder = Schema::builder();

        let id_field = schema_builder.add_text_field("id", STORED);
        let title_field = schema_builder.add_text_field("title", TEXT | STORED);
        let content_field = schema_builder.add_text_field("content", TEXT | STORED);
        let tags_field = schema_builder.add_text_field("tags", TEXT | STORED);

        let schema = schema_builder.build();

        // Create/open index
        let index_path = notes_dir.join("search_index");
        std::fs::create_dir_all(&index_path)?;

        let directory = MmapDirectory::open(&index_path)?;
        let index = Index::open_or_create(directory, schema.clone())?;

        // Create writer and reader
        let writer = index.writer(50_000_000)?; // 50MB buffer
        let reader = index.reader()?;

        // Create query parser for multiple fields
        let query_parser = QueryParser::for_index(&index, vec![title_field, content_field, tags_field]);

        Ok(SearchEngine {
            index,
            writer: Arc::new(Mutex::new(writer)),
            reader,
            query_parser,
            schema,
            title_field,
            content_field,
            id_field,
            tags_field,
        })
    }

    pub fn index_note(&self, id: &str, title: &str, content: &str, tags: &[String]) -> Result<()> {
        let mut doc = tantivy::doc!();
        doc.add_text(self.id_field, id);
        doc.add_text(self.title_field, title);
        doc.add_text(self.content_field, content);
        doc.add_text(self.tags_field, &tags.join(" "));

        let mut writer = self.writer.lock().unwrap();

        // Delete existing document with same ID
        let id_term = Term::from_field_text(self.id_field, id);
        writer.delete_term(id_term);

        // Add new document
        writer.add_document(doc)?;
        writer.commit()?;

        Ok(())
    }

    pub fn search(&self, query_str: &str) -> Result<Vec<SearchResult>> {
        let searcher = self.reader.searcher();

        // Parse query - support both exact and fuzzy search
        let query = if query_str.starts_with("\"") && query_str.ends_with("\"") {
            // Exact phrase search
            self.query_parser.parse_query(&query_str)?
        } else {
            // Normal search with fuzzy matching fallback
            self.query_parser.parse_query(&query_str)
                .unwrap_or_else(|_| {
                    // If parsing fails, create a simple term query
                    let cleaned = query_str.replace(&['(', ')', '[', ']', '{', '}', '*', '?'][..], "");
                    self.query_parser.parse_query(&cleaned).unwrap_or_else(|_| {
                        Box::new(tantivy::query::EmptyQuery)
                    })
                })
        };

        // Search with top 100 results
        let top_docs = searcher.search(&query, &TopDocs::with_limit(100))?;

        let mut results = Vec::new();
        for (_score, doc_address) in top_docs {
            let retrieved_doc: tantivy::TantivyDocument = searcher.doc(doc_address)?;

            let id = retrieved_doc
                .get_first(self.id_field)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let title = retrieved_doc
                .get_first(self.title_field)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let content = retrieved_doc
                .get_first(self.content_field)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let tags_str = retrieved_doc
                .get_first(self.tags_field)
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let tags: Vec<String> = if tags_str.is_empty() {
                Vec::new()
            } else {
                tags_str.split_whitespace().map(|s| s.to_string()).collect()
            };

            results.push(SearchResult {
                id,
                title,
                content_preview: Self::create_preview(&content, query_str, 100),
                score: _score,
                tags,
            });
        }

        // If no results from Tantivy, try fuzzy search
        if results.is_empty() {
            results = self.fuzzy_search(query_str)?;
        }

        Ok(results)
    }

    fn fuzzy_search(&self, query_str: &str) -> Result<Vec<SearchResult>> {
        let matcher = SkimMatcherV2::default();
        let searcher = self.reader.searcher();
        let mut results = Vec::new();

        // Get all documents and fuzzy match
        for segment_reader in searcher.segment_readers() {
            let store_reader = segment_reader.get_store_reader(0)?; // 0 cache blocks

            for doc_id in 0..segment_reader.max_doc() {
                if let Ok(doc) = store_reader.get::<TantivyDocument>(doc_id) {
                    let title = doc
                        .get_first(self.title_field)
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    let content = doc
                        .get_first(self.content_field)
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    // Calculate fuzzy match scores
                    let title_score = matcher.fuzzy_match(&title, query_str).unwrap_or(0);
                    let content_score = matcher.fuzzy_match(&content, query_str).unwrap_or(0);

                    let combined_score = title_score * 2 + content_score; // Weight title higher

                    if combined_score > 50 { // Threshold for relevance
                        let id = doc
                            .get_first(self.id_field)
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();

                        let tags_str = doc
                            .get_first(self.tags_field)
                            .and_then(|v| v.as_str())
                            .unwrap_or("");

                        let tags: Vec<String> = if tags_str.is_empty() {
                            Vec::new()
                        } else {
                            tags_str.split_whitespace().map(|s| s.to_string()).collect()
                        };

                        results.push(SearchResult {
                            id,
                            title,
                            content_preview: Self::create_preview(&content, query_str, 100),
                            score: combined_score as f32,
                            tags,
                        });
                    }
                }
            }
        }

        // Sort by score
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        results.truncate(50); // Limit results

        Ok(results)
    }

    fn create_preview(content: &str, query: &str, max_len: usize) -> String {
        // Find the query in content and create a preview around it
        let lower_content = content.to_lowercase();
        let lower_query = query.to_lowercase();

        if let Some(pos) = lower_content.find(&lower_query) {
            let start = pos.saturating_sub(30);
            let end = (pos + lower_query.len() + 70).min(content.len());

            let mut preview = String::new();
            if start > 0 {
                preview.push_str("...");
            }
            preview.push_str(&content[start..end]);
            if end < content.len() {
                preview.push_str("...");
            }
            preview
        } else {
            // Just show beginning of content
            if content.len() <= max_len {
                content.to_string()
            } else {
                format!("{}...", &content[..max_len])
            }
        }
    }

    pub fn delete_note(&self, id: &str) -> Result<()> {
        let mut writer = self.writer.lock().unwrap();
        let id_term = Term::from_field_text(self.id_field, id);
        writer.delete_term(id_term);
        writer.commit()?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn reindex_all(&self, notes: &[crate::note_store::Note]) -> Result<()> {
        let mut writer = self.writer.lock().unwrap();

        // Clear index
        writer.delete_all_documents()?;

        // Re-add all notes
        for note in notes {
            let mut doc = tantivy::doc!();
            doc.add_text(self.id_field, &note.id);
            doc.add_text(self.title_field, &note.title);
            doc.add_text(self.content_field, &note.content);
            doc.add_text(self.tags_field, &note.tags.join(" "));
            writer.add_document(doc)?;
        }

        writer.commit()?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub id: String,
    pub title: String,
    #[allow(dead_code)]
    pub content_preview: String,
    pub score: f32,
    #[allow(dead_code)]
    pub tags: Vec<String>,
}