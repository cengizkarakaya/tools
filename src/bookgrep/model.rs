use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::cli::SearchArgs;

// Ortak veri tipleri burada tutulur; extractor, source ve output katmanları aynı modeli paylaşır.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DocumentFormat {
    Pdf,
    Epub,
    Opf,
}

impl DocumentFormat {
    pub fn from_path(path: &Path) -> Option<Self> {
        // `?` burada fonksiyon `Option` döndürdüğü için uzantı yoksa `None` ile çıkar.
        match path
            .extension()?
            .to_string_lossy()
            .to_ascii_lowercase()
            .as_str()
        {
            "pdf" => Some(Self::Pdf),
            "epub" => Some(Self::Epub),
            "opf" => Some(Self::Opf),
            _ => None,
        }
    }

    pub fn is_searchable(self) -> bool {
        matches!(self, Self::Pdf | Self::Epub)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentRef {
    pub source_path: PathBuf,
    pub format: DocumentFormat,
    pub size: Option<u64>,
    pub modified_unix: Option<u64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct DocumentMetadata {
    pub title: Option<String>,
    pub authors: Vec<String>,
    pub language: Option<String>,
    pub publisher: Option<String>,
    pub date: Option<String>,
    pub identifiers: Vec<String>,
    pub subjects: Vec<String>,
    pub description: Option<String>,
    pub series: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextSection {
    pub label: Option<String>,
    pub ordinal: Option<usize>,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedDocument {
    pub source_path: PathBuf,
    pub format: DocumentFormat,
    pub metadata: DocumentMetadata,
    pub sections: Vec<TextSection>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SearchMatch {
    pub text: String,
    pub before: String,
    pub after: String,
    pub page: Option<usize>,
    pub chapter: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SearchOptions {
    pub query: String,
    pub case_sensitive: bool,
    pub regex: bool,
    pub context: usize,
    pub limit: Option<usize>,
    pub extract_timeout_secs: u64,
}

impl From<&SearchArgs> for SearchOptions {
    fn from(value: &SearchArgs) -> Self {
        Self {
            // `String` sahipli veri olduğu için CLI argümanından yeni bir kopya alınır.
            query: value.query.clone(),
            case_sensitive: value.case_sensitive,
            regex: value.regex,
            context: value.context,
            limit: value.limit,
            extract_timeout_secs: value.extract_timeout,
        }
    }
}
