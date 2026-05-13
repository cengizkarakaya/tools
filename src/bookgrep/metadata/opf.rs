use std::{fs, path::Path};

use roxmltree::Document;

use super::MetadataReader;
use crate::bookgrep::{
    error::{BookgrepError, Result},
    model::DocumentMetadata,
};

pub struct OpfMetadataReader;

impl MetadataReader for OpfMetadataReader {
    fn read_sidecar_metadata(&self, document_path: &Path) -> Result<Option<DocumentMetadata>> {
        let Some(path) = find_sidecar_opf(document_path) else {
            return Ok(None);
        };
        let raw =
            fs::read_to_string(&path).map_err(|err| BookgrepError::Metadata(err.to_string()))?;
        parse_opf_metadata(&raw).map(Some)
    }
}

pub fn find_sidecar_opf(document_path: &Path) -> Option<std::path::PathBuf> {
    let same_stem = document_path.with_extension("opf");
    if same_stem.exists() {
        return Some(same_stem);
    }

    let parent = document_path.parent()?;
    let mut opfs = fs::read_dir(parent)
        .ok()?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("opf"))
        });

    let first = opfs.next()?;
    if opfs.next().is_none() {
        Some(first)
    } else {
        None
    }
}

pub fn parse_opf_metadata(raw: &str) -> Result<DocumentMetadata> {
    let doc = Document::parse(raw).map_err(|err| BookgrepError::Metadata(err.to_string()))?;
    let mut metadata = DocumentMetadata::default();

    for node in doc.descendants().filter(|node| node.is_element()) {
        let name = node.tag_name().name();
        let text = node.text().map(str::trim).filter(|text| !text.is_empty());
        match name {
            "title" => metadata.title = text.map(ToOwned::to_owned),
            "creator" => {
                if let Some(text) = text {
                    metadata.authors.push(text.to_owned());
                }
            }
            "language" => metadata.language = text.map(ToOwned::to_owned),
            "publisher" => metadata.publisher = text.map(ToOwned::to_owned),
            "date" => metadata.date = text.map(ToOwned::to_owned),
            "identifier" => {
                if let Some(text) = text {
                    metadata.identifiers.push(text.to_owned());
                }
            }
            "subject" => {
                if let Some(text) = text {
                    metadata.subjects.push(text.to_owned());
                }
            }
            "description" => metadata.description = text.map(ToOwned::to_owned),
            "meta" => parse_meta_node(node, &mut metadata),
            _ => {}
        }
    }

    metadata.authors.dedup();
    metadata.subjects.dedup();
    metadata.identifiers.dedup();
    Ok(metadata)
}

fn parse_meta_node(node: roxmltree::Node<'_, '_>, metadata: &mut DocumentMetadata) {
    let name = node
        .attribute("name")
        .or_else(|| node.attribute("property"));
    let content = node
        .attribute("content")
        .or_else(|| node.text())
        .map(str::trim);
    match (name, content) {
        (Some("calibre:series"), Some(value)) if !value.is_empty() => {
            metadata.series = Some(value.to_owned());
        }
        (Some("calibre:title_sort"), Some(value))
            if metadata.title.is_none() && !value.is_empty() =>
        {
            metadata.title = Some(value.to_owned());
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_calibre_opf_metadata() {
        let raw = r#"
            <package xmlns:dc="http://purl.org/dc/elements/1.1/">
              <metadata>
                <dc:title>Rust Book</dc:title>
                <dc:creator>Ferris Author</dc:creator>
                <dc:language>en</dc:language>
                <dc:subject>rust</dc:subject>
                <dc:subject>systems</dc:subject>
                <dc:identifier>isbn:123</dc:identifier>
                <meta name="calibre:series" content="Learning Rust"/>
              </metadata>
            </package>
        "#;

        let metadata = parse_opf_metadata(raw).expect("metadata parses");
        assert_eq!(metadata.title.as_deref(), Some("Rust Book"));
        assert_eq!(metadata.authors, vec!["Ferris Author"]);
        assert_eq!(metadata.language.as_deref(), Some("en"));
        assert_eq!(metadata.subjects, vec!["rust", "systems"]);
        assert_eq!(metadata.series.as_deref(), Some("Learning Rust"));
    }
}
