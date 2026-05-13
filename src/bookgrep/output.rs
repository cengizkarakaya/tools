use std::io::{self, Write};

use anyhow::Context;

use crate::bookgrep::search::result::{JsonSearchResult, SearchResult};

const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const BLUE: &str = "\x1b[34m";
const GREEN: &str = "\x1b[32m";
const CYAN: &str = "\x1b[36m";
const YELLOW: &str = "\x1b[33m";
const MAGENTA: &str = "\x1b[35m";

#[derive(Debug, Clone, Copy)]
pub enum OutputFormat {
    Terminal { show_matches: bool },
    Json,
}

pub fn write_results(results: &[SearchResult], format: OutputFormat) -> anyhow::Result<()> {
    match format {
        OutputFormat::Terminal { show_matches } => write_terminal(results, show_matches),
        OutputFormat::Json => write_json(results),
    }
}

fn write_json(results: &[SearchResult]) -> anyhow::Result<()> {
    // JSON çıktısı için sonuçları ödünç alan hafif bir görünüm tipine dönüştürüyoruz.
    let json: Vec<_> = results.iter().map(JsonSearchResult::from).collect();
    let stdout = io::stdout();
    serde_json::to_writer_pretty(stdout.lock(), &json).context("could not write JSON output")?;
    println!();
    Ok(())
}

fn write_terminal(results: &[SearchResult], show_matches: bool) -> anyhow::Result<()> {
    let mut stdout = io::stdout().lock();
    // Iterator zinciri tüm belgelerdeki eşleşme sayılarını tek toplamda birleştirir.
    let count: usize = results.iter().map(|result| result.matches.len()).sum();
    if count == 0 && results.len() == 1 {
        write_document_info(&mut stdout, &results[0])?;
        return Ok(());
    }

    if results.is_empty() {
        writeln!(stdout, "{YELLOW}{BOLD}No matching books found{RESET}")?;
        return Ok(());
    }

    writeln!(
        stdout,
        "{GREEN}{BOLD}Found {} matching books{RESET} {DIM}({count} total matches){RESET}",
        results.len()
    )?;
    writeln!(stdout)?;
    for (index, result) in results.iter().enumerate() {
        write_book_summary(&mut stdout, index + 1, result)?;
        if show_matches {
            write_matches(&mut stdout, result)?;
        }
        writeln!(stdout)?;
    }
    Ok(())
}

fn write_document_info(mut writer: impl Write, result: &SearchResult) -> anyhow::Result<()> {
    writeln!(
        writer,
        "{GREEN}{BOLD}Document info{RESET} {DIM}{} sections{RESET}",
        result.document.sections.len()
    )?;
    write_metadata(&mut writer, result)?;
    write_field(
        &mut writer,
        "File",
        &result.document.source_path.display().to_string(),
    )?;
    write_field(
        &mut writer,
        "Format",
        &format!("{:?}", result.document.format),
    )?;
    Ok(())
}

fn write_metadata(mut writer: impl Write, result: &SearchResult) -> anyhow::Result<()> {
    let metadata = &result.document.metadata;
    if let Some(title) = &metadata.title {
        write_field(&mut writer, "Title", title)?;
    }
    if !metadata.authors.is_empty() {
        write_field(&mut writer, "Author", &metadata.authors.join(", "))?;
    }
    if let Some(language) = &metadata.language {
        write_field(&mut writer, "Language", language)?;
    }
    if let Some(publisher) = &metadata.publisher {
        write_field(&mut writer, "Publisher", publisher)?;
    }
    if let Some(date) = &metadata.date {
        write_field(&mut writer, "Date", date)?;
    }
    Ok(())
}

fn write_book_summary(
    mut writer: impl Write,
    index: usize,
    result: &SearchResult,
) -> anyhow::Result<()> {
    let metadata = &result.document.metadata;
    let fallback_name = result
        .document
        .source_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("<unknown>");
    let title = metadata.title.as_deref().unwrap_or(fallback_name);

    writeln!(
        writer,
        "{CYAN}{BOLD}[{index}] {title}{RESET} {DIM}{} match(es){RESET}",
        result.matches.len()
    )?;
    write_metadata(&mut writer, result)?;
    if metadata.title.is_none() {
        write_field(&mut writer, "Title", fallback_name)?;
    }
    write_field(
        &mut writer,
        "File",
        &result.document.source_path.display().to_string(),
    )?;
    write_field(
        &mut writer,
        "Format",
        &format!("{:?}", result.document.format),
    )?;
    Ok(())
}

fn write_matches(mut writer: impl Write, result: &SearchResult) -> anyhow::Result<()> {
    if result.matches.is_empty() {
        return Ok(());
    }

    writeln!(writer, "  {MAGENTA}{BOLD}Matches{RESET}")?;
    for (index, hit) in result.matches.iter().enumerate() {
        write!(writer, "  {DIM}{}.{RESET} ", index + 1)?;
        if let Some(page) = hit.page {
            write!(writer, "{YELLOW}Page {page}{RESET} ")?;
        }
        if let Some(chapter) = &hit.chapter {
            write!(writer, "{YELLOW}{chapter}{RESET} ")?;
        }
        writeln!(
            writer,
            "... {}{BOLD}{GREEN}{}{RESET}{} ...",
            hit.before, hit.text, hit.after
        )?;
    }
    Ok(())
}

fn write_field(mut writer: impl Write, label: &str, value: &str) -> anyhow::Result<()> {
    writeln!(writer, "  {BLUE}{BOLD}{label:<9}{RESET} {value}")?;
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
