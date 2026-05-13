use std::path::Path;

use crate::bookgrep::{
    error::{BookgrepError, Result},
    metadata::{MetadataReader, opf::OpfMetadataReader},
    model::{DocumentFormat, DocumentMetadata, ExtractedDocument, TextSection},
};

use super::TextExtractor;

pub struct PdfExtractor;

impl TextExtractor for PdfExtractor {
    fn extract_text(
        &self,
        path: &Path,
        include_sidecar_metadata: bool,
    ) -> Result<ExtractedDocument> {
        let text = pdf_extract::extract_text(path)
            .map_err(|_| BookgrepError::PdfExtraction(path.to_path_buf()))?;
        if text.trim().is_empty() {
            return Err(BookgrepError::PdfExtraction(path.to_path_buf()));
        }

        let metadata = if include_sidecar_metadata {
            OpfMetadataReader
                .read_sidecar_metadata(path)?
                .unwrap_or_default()
        } else {
            DocumentMetadata::default()
        };

        Ok(ExtractedDocument {
            source_path: path.to_path_buf(),
            format: DocumentFormat::Pdf,
            metadata,
            sections: split_pdf_pages(&text),
        })
    }
}

fn split_pdf_pages(text: &str) -> Vec<TextSection> {
    let parts: Vec<_> = text.split('\x0C').collect();
    if parts.len() > 1 {
        parts
            .into_iter()
            .enumerate()
            .filter_map(|(index, part)| {
                let text = part.trim().to_owned();
                (!text.is_empty()).then_some(TextSection {
                    label: Some(format!("Page {}", index + 1)),
                    ordinal: Some(index + 1),
                    text,
                })
            })
            .collect()
    } else {
        vec![TextSection {
            label: None,
            ordinal: None,
            text: text.to_owned(),
        }]
    }
}
