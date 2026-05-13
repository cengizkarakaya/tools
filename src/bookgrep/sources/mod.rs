pub mod local;
pub mod pcloud;

use std::path::PathBuf;

use crate::bookgrep::{error::Result, model::DocumentRef};

pub trait DocumentSource {
    fn list_documents(&self) -> Result<Vec<DocumentRef>>;
    fn fetch_document(&self, document: &DocumentRef) -> Result<PathBuf>;
}
