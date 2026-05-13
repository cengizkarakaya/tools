pub mod epub;
pub mod ocr;
pub mod pdf;

use std::path::Path;

use crate::bookgrep::{
    error::{BookgrepError, Result},
    model::{DocumentFormat, ExtractedDocument},
};

// Her format kendi çıkarıcısını sağlar; çağıran kod sadece bu ortak trait'i bilir.
pub trait TextExtractor {
    fn extract_text(
        &self,
        path: &Path,
        include_sidecar_metadata: bool,
    ) -> Result<ExtractedDocument>;
}

pub fn extract_file(path: &Path, include_sidecar_metadata: bool) -> Result<ExtractedDocument> {
    // Format seçimi tek yerde kalır; yeni format eklenirse bu match genişletilir.
    match DocumentFormat::from_path(path) {
        Some(DocumentFormat::Pdf) => pdf::PdfExtractor.extract_text(path, include_sidecar_metadata),
        Some(DocumentFormat::Epub) => {
            epub::EpubExtractor.extract_text(path, include_sidecar_metadata)
        }
        Some(DocumentFormat::Opf) | None => {
            Err(BookgrepError::UnsupportedFormat(path.to_path_buf()))
        }
    }
}
