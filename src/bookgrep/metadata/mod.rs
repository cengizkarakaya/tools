pub mod opf;

use std::path::Path;

use crate::bookgrep::{error::Result, model::DocumentMetadata};

// Metadata okumak opsiyoneldir; yan dosya yoksa hata yerine `Ok(None)` döner.
pub trait MetadataReader {
    fn read_sidecar_metadata(&self, document_path: &Path) -> Result<Option<DocumentMetadata>>;
}
