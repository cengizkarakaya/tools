use std::{cell::Cell, panic, path::Path};

use crate::bookgrep::{
    error::{BookgrepError, Result},
    metadata::{MetadataReader, opf::OpfMetadataReader},
    model::{DocumentFormat, DocumentMetadata, ExtractedDocument, TextSection},
};

use super::{TextExtractor, ocr};

const OCR_MIN_TEXT_CHARS: usize = 64;

thread_local! {
    static PDF_EXTRACTION_ACTIVE: Cell<bool> = const { Cell::new(false) };
}

pub(crate) fn is_pdf_extraction_active() -> bool {
    PDF_EXTRACTION_ACTIVE.with(Cell::get)
}

struct PdfExtractionGuard;

impl PdfExtractionGuard {
    fn new() -> Self {
        PDF_EXTRACTION_ACTIVE.with(|active| active.set(true));
        Self
    }
}

impl Drop for PdfExtractionGuard {
    fn drop(&mut self) {
        PDF_EXTRACTION_ACTIVE.with(|active| active.set(false));
    }
}

pub struct PdfExtractor;

impl TextExtractor for PdfExtractor {
    fn extract_text(
        &self,
        path: &Path,
        include_sidecar_metadata: bool,
    ) -> Result<ExtractedDocument> {
        // Bazı PDF font/encoding hatalarında `pdf-extract` hata döndürmek yerine panic atabiliyor.
        let text_result = panic::catch_unwind(|| {
            let _guard = PdfExtractionGuard::new();
            pdf_extract::extract_text(path)
        });
        let mut text = match text_result {
            Ok(Ok(text)) => text,
            Ok(Err(_)) | Err(_) => String::new(),
        };
        if should_use_ocr_fallback(&text) {
            text = ocr::extract_pdf_text(path)?;
        }
        if text.trim().is_empty() {
            return Err(BookgrepError::PdfExtraction(path.to_path_buf()));
        }

        // PDF metni gelir; metadata gerekiyorsa aynı adlı `.opf` yan dosyasından okunur.
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

fn should_use_ocr_fallback(text: &str) -> bool {
    text.split_whitespace().collect::<String>().chars().count() < OCR_MIN_TEXT_CHARS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_ocr_for_empty_or_tiny_text() {
        assert!(should_use_ocr_fallback(""));
        assert!(should_use_ocr_fallback("page 1"));
    }

    #[test]
    fn keeps_normal_pdf_text_without_ocr() {
        let text = "Rust ownership and borrowing ".repeat(10);
        assert!(!should_use_ocr_fallback(&text));
    }
}

fn split_pdf_pages(text: &str) -> Vec<TextSection> {
    // `pdf_extract` sayfaları form-feed (`\x0C`) karakteriyle ayırabilir.
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
