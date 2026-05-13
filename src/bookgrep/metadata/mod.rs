pub mod opf;

use std::path::Path;

use crate::bookgrep::{error::Result, model::DocumentMetadata};

pub trait MetadataReader {
    fn read_sidecar_metadata(&self, document_path: &Path) -> Result<Option<DocumentMetadata>>;
}
