use std::{
    fs,
    path::{Path, PathBuf},
};

use directories::ProjectDirs;
use sha2::{Digest, Sha256};

use super::error::{BookgrepError, Result};

#[derive(Debug, Clone)]
pub struct Cache {
    root: PathBuf,
}

impl Cache {
    pub fn new(root: Option<PathBuf>) -> Result<Self> {
        let root = match root {
            Some(root) => root,
            None => ProjectDirs::from("com", "cengiz", "bookgrep")
                .ok_or_else(|| BookgrepError::Source("could not resolve cache directory".into()))?
                .cache_dir()
                .to_path_buf(),
        };
        fs::create_dir_all(&root).map_err(|err| BookgrepError::Source(err.to_string()))?;
        Ok(Self { root })
    }

    pub fn path_for_key(&self, key: &str, extension: &str) -> PathBuf {
        let mut hasher = Sha256::new();
        hasher.update(key.as_bytes());
        let digest = format!("{:x}", hasher.finalize());
        self.root.join(format!("{digest}.{extension}"))
    }

    pub fn is_valid(path: &Path, expected_size: Option<u64>) -> bool {
        let Ok(metadata) = fs::metadata(path) else {
            return false;
        };
        expected_size.is_none_or(|size| metadata.len() == size)
    }
}
