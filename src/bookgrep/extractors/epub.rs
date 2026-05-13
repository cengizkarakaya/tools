use std::{
    collections::HashMap,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

use roxmltree::Document;
use zip::ZipArchive;

use crate::bookgrep::{
    error::{BookgrepError, Result},
    metadata::opf::parse_opf_metadata,
    model::{DocumentFormat, ExtractedDocument, TextSection},
};

use super::TextExtractor;

pub struct EpubExtractor;

impl TextExtractor for EpubExtractor {
    fn extract_text(
        &self,
        path: &Path,
        _include_sidecar_metadata: bool,
    ) -> Result<ExtractedDocument> {
        let file = File::open(path)
            .map_err(|err| BookgrepError::EpubExtraction(err.to_string().into()))?;
        let mut archive = ZipArchive::new(file)
            .map_err(|err| BookgrepError::EpubExtraction(err.to_string().into()))?;
        // EPUB aslında ZIP'tir; önce container.xml içinden ana OPF dosyasının yolu bulunur.
        let opf_path = read_container_opf_path(&mut archive)?;
        let opf_raw = read_zip_text(&mut archive, &opf_path)?;
        let metadata = parse_opf_metadata(&opf_raw)?;
        let sections = extract_sections(&mut archive, &opf_path, &opf_raw)?;

        if sections
            .iter()
            .all(|section| section.text.trim().is_empty())
        {
            return Err(BookgrepError::EpubExtraction(path.to_path_buf()));
        }

        Ok(ExtractedDocument {
            source_path: path.to_path_buf(),
            format: DocumentFormat::Epub,
            metadata,
            sections,
        })
    }
}

fn read_container_opf_path<R: std::io::Read + std::io::Seek>(
    archive: &mut ZipArchive<R>,
) -> Result<String> {
    let raw = read_zip_text(archive, "META-INF/container.xml")?;
    let doc = Document::parse(&raw)
        .map_err(|err| BookgrepError::EpubExtraction(err.to_string().into()))?;
    doc.descendants()
        .find(|node| node.has_tag_name("rootfile"))
        .and_then(|node| node.attribute("full-path"))
        .map(ToOwned::to_owned)
        .ok_or_else(|| BookgrepError::EpubExtraction("EPUB container has no rootfile".into()))
}

fn extract_sections<R: std::io::Read + std::io::Seek>(
    archive: &mut ZipArchive<R>,
    opf_path: &str,
    opf_raw: &str,
) -> Result<Vec<TextSection>> {
    let doc = Document::parse(opf_raw)
        .map_err(|err| BookgrepError::EpubExtraction(err.to_string().into()))?;
    let manifest = manifest_items(&doc);
    let base = Path::new(opf_path)
        .parent()
        .unwrap_or_else(|| Path::new(""));
    let spine_ids: Vec<_> = doc
        .descendants()
        .filter(|node| node.has_tag_name("itemref"))
        .filter_map(|node| node.attribute("idref"))
        .collect();

    let mut sections = Vec::new();
    for (index, idref) in spine_ids.iter().enumerate() {
        let Some(href) = manifest.get(*idref) else {
            continue;
        };
        let zip_path = normalize_zip_path(base.join(href));
        let raw = read_zip_text(archive, &zip_path)?;
        let text = xhtml_to_text(&raw);
        if text.trim().is_empty() {
            continue;
        }
        sections.push(TextSection {
            label: chapter_title(&raw).or_else(|| Some(href.to_string())),
            ordinal: Some(index + 1),
            text,
        });
    }

    Ok(sections)
}

fn manifest_items<'a>(doc: &'a Document<'a>) -> HashMap<&'a str, &'a str> {
    // Dönen `&str` değerler XML dokümanının içini ödünç alır; bu yüzden lifetime `'a` görünür.
    doc.descendants()
        .filter(|node| node.has_tag_name("item"))
        .filter_map(|node| Some((node.attribute("id")?, node.attribute("href")?)))
        .collect()
}

fn read_zip_text<R: std::io::Read + std::io::Seek>(
    archive: &mut ZipArchive<R>,
    path: &str,
) -> Result<String> {
    let mut file = archive
        .by_name(path)
        .map_err(|err| BookgrepError::EpubExtraction(err.to_string().into()))?;
    let mut raw = String::new();
    file.read_to_string(&mut raw)
        .map_err(|err| BookgrepError::EpubExtraction(err.to_string().into()))?;
    Ok(raw)
}

fn normalize_zip_path(path: PathBuf) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn chapter_title(raw: &str) -> Option<String> {
    let doc = Document::parse(raw).ok()?;
    doc.descendants()
        .find(|node| matches!(node.tag_name().name(), "title" | "h1" | "h2"))
        .and_then(|node| node.text())
        .map(clean_whitespace)
        .filter(|text| !text.is_empty())
}

fn xhtml_to_text(raw: &str) -> String {
    if let Ok(doc) = Document::parse(raw) {
        let mut parts = Vec::new();
        for node in doc.descendants().filter(|node| node.is_text()) {
            // script/style içeriği kitap metni sayılmadığı için atlanır.
            if node
                .ancestors()
                .any(|parent| matches!(parent.tag_name().name(), "script" | "style"))
            {
                continue;
            }
            if let Some(text) = node
                .text()
                .map(clean_whitespace)
                .filter(|text| !text.is_empty())
            {
                parts.push(text);
            }
        }
        return parts.join(" ");
    }

    // XML bozuksa kaba bir etiket temizleme ile yine de metin çıkarmayı deneriz.
    let mut text = String::with_capacity(raw.len());
    let mut inside_tag = false;
    for ch in raw.chars() {
        match ch {
            '<' => inside_tag = true,
            '>' => {
                inside_tag = false;
                text.push(' ');
            }
            _ if !inside_tag => text.push(ch),
            _ => {}
        }
    }
    clean_whitespace(&text)
}

fn clean_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_plain_text_from_xhtml() {
        let text = xhtml_to_text(
            "<html><body><h1>Memory</h1><p>ownership and borrowing</p></body></html>",
        );
        assert_eq!(text, "Memory ownership and borrowing");
    }
}
