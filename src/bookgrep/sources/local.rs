use std::{
    fs,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use walkdir::WalkDir;

use super::DocumentSource;
use crate::bookgrep::{
    error::{BookgrepError, Result},
    model::{DocumentFormat, DocumentRef},
};

#[derive(Debug, Clone)]
pub struct LocalSource {
    root: PathBuf,
    recursive: bool,
    extensions: Option<Vec<DocumentFormat>>,
}

impl LocalSource {
    pub fn new(root: PathBuf, recursive: bool, extensions: Option<Vec<DocumentFormat>>) -> Self {
        Self {
            root,
            recursive,
            extensions,
        }
    }

    fn accepts(&self, path: &Path) -> bool {
        let Some(format) = DocumentFormat::from_path(path) else {
            return false;
        };
        format.is_searchable()
            && self
                .extensions
                .as_ref()
                .is_none_or(|extensions| extensions.contains(&format))
    }
}

impl DocumentSource for LocalSource {
    fn list_documents(&self) -> Result<Vec<DocumentRef>> {
        let max_depth = if self.recursive { usize::MAX } else { 1 };
        let mut documents = Vec::new();

        for entry in WalkDir::new(&self.root).max_depth(max_depth) {
            let entry = entry.map_err(|err| BookgrepError::Source(err.to_string()))?;
            if !entry.file_type().is_file() || !self.accepts(entry.path()) {
                continue;
            }
            let metadata =
                fs::metadata(entry.path()).map_err(|err| BookgrepError::Source(err.to_string()))?;
            let modified_unix = metadata
                .modified()
                .ok()
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .map(|duration| duration.as_secs());
            documents.push(DocumentRef {
                source_path: entry.path().to_path_buf(),
                format: DocumentFormat::from_path(entry.path())
                    .expect("accepted documents have a format"),
                size: Some(metadata.len()),
                modified_unix,
            });
        }

        Ok(documents)
    }

    fn fetch_document(&self, document: &DocumentRef) -> Result<PathBuf> {
        Ok(document.source_path.clone())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn lists_only_supported_documents() {
        let dir = tempdir().expect("tempdir");
        fs::write(dir.path().join("book.pdf"), b"%PDF").expect("pdf");
        fs::write(dir.path().join("book.epub"), b"epub").expect("epub");
        fs::write(dir.path().join("notes.txt"), b"text").expect("txt");

        let source = LocalSource::new(dir.path().to_path_buf(), false, None);
        let documents = source.list_documents().expect("documents");
        assert_eq!(documents.len(), 2);
        assert!(documents.iter().all(|doc| doc.format.is_searchable()));
    }
}
