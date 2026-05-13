use std::{env, fs, path::PathBuf};

use anyhow::Context;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    pub pcloud_token: Option<String>,
    pub cache_dir: Option<PathBuf>,
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        let mut config = Self::default();
        if let Some(project_dirs) = ProjectDirs::from("com", "cengiz", "bookgrep") {
            let config_path = project_dirs.config_dir().join("config.json");
            if config_path.exists() {
                let raw = fs::read_to_string(&config_path)
                    .with_context(|| format!("could not read {}", config_path.display()))?;
                config = serde_json::from_str(&raw)
                    .with_context(|| format!("invalid JSON in {}", config_path.display()))?;
            }
        }

        if let Ok(token) = env::var("BOOKGREP_PCLOUD_TOKEN")
            && !token.trim().is_empty()
        {
            config.pcloud_token = Some(token);
        }

        Ok(config)
    }
}
