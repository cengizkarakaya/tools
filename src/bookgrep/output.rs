use std::io::{self, Write};

use anyhow::Context;

use crate::bookgrep::search::result::{JsonSearchResult, SearchResult};

#[derive(Debug, Clone, Copy)]
pub enum OutputFormat {
    Terminal,
    Json,
}

pub fn write_results(results: &[SearchResult], format: OutputFormat) -> anyhow::Result<()> {
    match format {
        OutputFormat::Terminal => write_terminal(results),
        OutputFormat::Json => write_json(results),
    }
}

fn write_json(results: &[SearchResult]) -> anyhow::Result<()> {
    let json: Vec<_> = results.iter().map(JsonSearchResult::from).collect();
    let stdout = io::stdout();
    serde_json::to_writer_pretty(stdout.lock(), &json).context("could not write JSON output")?;
    println!();
    Ok(())
}

fn write_terminal(results: &[SearchResult]) -> anyhow::Result<()> {
    let mut stdout = io::stdout().lock();
    let count: usize = results.iter().map(|result| result.matches.len()).sum();
    if count == 0 && results.len() == 1 {
        write_document_info(&mut stdout, &results[0])?;
        return Ok(());
    }

    writeln!(stdout, "Found {count} matches")?;
    writeln!(stdout)?;
    for (index, result) in results.iter().enumerate() {
        writeln!(
            stdout,
            "[{}] {}",
            index + 1,
            result
                .document
                .source_path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("<unknown>")
        )?;
        write_metadata(&mut stdout, result)?;
        for hit in &result.matches {
            if let Some(page) = hit.page {
                writeln!(stdout, "Page: {page}")?;
            }
            if let Some(chapter) = &hit.chapter {
                writeln!(stdout, "Chapter: {chapter}")?;
            }
            writeln!(stdout, "Match:")?;
            writeln!(stdout, "... {}{}{} ...", hit.before, hit.text, hit.after)?;
        }
        writeln!(stdout)?;
    }
    Ok(())
}

fn write_document_info(mut writer: impl Write, result: &SearchResult) -> anyhow::Result<()> {
    writeln!(writer, "File: {}", result.document.source_path.display())?;
    writeln!(writer, "Format: {:?}", result.document.format)?;
    write_metadata(&mut writer, result)?;
    writeln!(writer, "Sections: {}", result.document.sections.len())?;
    Ok(())
}

fn write_metadata(mut writer: impl Write, result: &SearchResult) -> anyhow::Result<()> {
    let metadata = &result.document.metadata;
    if let Some(title) = &metadata.title {
        writeln!(writer, "Title: {title}")?;
    }
    if !metadata.authors.is_empty() {
        writeln!(writer, "Author: {}", metadata.authors.join(", "))?;
    }
    if let Some(language) = &metadata.language {
        writeln!(writer, "Language: {language}")?;
    }
    if let Some(publisher) = &metadata.publisher {
        writeln!(writer, "Publisher: {publisher}")?;
    }
    if let Some(date) = &metadata.date {
        writeln!(writer, "Date: {date}")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::bookgrep::{
        model::{DocumentFormat, DocumentMetadata, ExtractedDocument, SearchMatch},
        search::result::{JsonSearchResult, SearchResult},
    };

    #[test]
    fn serializes_json_shape() {
        let result = SearchResult {
            document: ExtractedDocument {
                source_path: PathBuf::from("book.pdf"),
                format: DocumentFormat::Pdf,
                metadata: DocumentMetadata {
                    title: Some("Book".into()),
                    authors: vec!["Author".into()],
                    ..DocumentMetadata::default()
                },
                sections: Vec::new(),
            },
            matches: vec![SearchMatch {
                text: "ownership".into(),
                before: "Rust ".into(),
                after: " model".into(),
                page: Some(1),
                chapter: None,
            }],
        };

        let json = serde_json::to_string(&JsonSearchResult::from(&result)).expect("json");
        assert!(json.contains("\"file\":\"book.pdf\""));
        assert!(json.contains("\"title\":\"Book\""));
        assert!(json.contains("\"page\":1"));
    }
}
