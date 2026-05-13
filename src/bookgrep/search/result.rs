use serde::{Deserialize, Serialize};

use crate::bookgrep::model::{DocumentFormat, DocumentMetadata, ExtractedDocument, SearchMatch};

// İç akışta kullanılan zengin sonuç modeli: belge bilgisi ve eşleşmeler birlikte taşınır.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub document: ExtractedDocument,
    pub matches: Vec<SearchMatch>,
}

impl SearchResult {
    pub fn from_document_info(document: ExtractedDocument) -> Self {
        Self {
            document,
            matches: Vec::new(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct JsonSearchResult<'a> {
    // `'a`, JSON görünümünün veriyi kopyalamadan `SearchResult` içinden ödünç aldığını gösterir.
    pub file: String,
    pub format: DocumentFormat,
    pub title: Option<&'a str>,
    pub authors: &'a [String],
    pub metadata: &'a DocumentMetadata,
    pub matches: &'a [SearchMatch],
}

impl<'a> From<&'a SearchResult> for JsonSearchResult<'a> {
    fn from(value: &'a SearchResult) -> Self {
        Self {
            file: value.document.source_path.display().to_string(),
            format: value.document.format,
            title: value.document.metadata.title.as_deref(),
            authors: &value.document.metadata.authors,
            metadata: &value.document.metadata,
            matches: &value.matches,
        }
    }
}
