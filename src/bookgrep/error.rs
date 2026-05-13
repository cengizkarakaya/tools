use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum BookgrepError {
    #[error("unsupported document format: {0}")]
    UnsupportedFormat(PathBuf),
    #[error(
        "PDF text could not be extracted. The file may be scanned, encrypted, or malformed: {0}"
    )]
    PdfExtraction(PathBuf),
    #[error("EPUB text could not be extracted: {0}")]
    EpubExtraction(PathBuf),
    #[error("metadata could not be read: {0}")]
    Metadata(String),
    #[error("source could not be read: {0}")]
    Source(String),
    #[error("invalid search expression: {0}")]
    InvalidSearch(String),
    #[cfg_attr(not(feature = "pcloud"), allow(dead_code))]
    #[error("pCloud API error: {0}")]
    PCloud(String),
}

pub type Result<T> = std::result::Result<T, BookgrepError>;
