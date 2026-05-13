use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

use super::model::DocumentFormat;

#[derive(Debug, Parser)]
#[command(name = "bookgrep", version, about = "Search PDF and EPUB books")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Search(SearchArgs),
    Info(InfoArgs),
    Index(IndexArgs),
    SearchIndex(SearchIndexArgs),
}

#[derive(Debug, Args)]
pub struct SearchArgs {
    #[arg(long)]
    pub path: Option<PathBuf>,
    #[arg(long)]
    pub pcloud_folder_id: Option<u64>,
    #[arg(long)]
    pub pcloud_path: Option<String>,
    #[arg(long)]
    pub query: String,
    #[arg(long)]
    pub recursive: bool,
    #[arg(long)]
    pub case_sensitive: bool,
    #[arg(long)]
    pub regex: bool,
    #[arg(long, default_value_t = 80)]
    pub context: usize,
    #[arg(long)]
    pub limit: Option<usize>,
    #[arg(long)]
    pub json: bool,
    #[arg(long = "extension", value_enum)]
    extension: Vec<ExtensionArg>,
    #[arg(long)]
    pub metadata: bool,
}

impl SearchArgs {
    pub fn extensions(&self) -> Option<Vec<DocumentFormat>> {
        if self.extension.is_empty() {
            return None;
        }

        Some(
            self.extension
                .iter()
                .map(|ext| match ext {
                    ExtensionArg::Pdf => DocumentFormat::Pdf,
                    ExtensionArg::Epub => DocumentFormat::Epub,
                })
                .collect(),
        )
    }
}

#[derive(Debug, Clone, ValueEnum)]
enum ExtensionArg {
    Pdf,
    Epub,
}

#[derive(Debug, Args)]
pub struct InfoArgs {
    #[arg(long)]
    pub file: PathBuf,
}

#[derive(Debug, Args)]
pub struct IndexArgs {
    #[arg(long)]
    pub path: PathBuf,
}

#[derive(Debug, Args)]
pub struct SearchIndexArgs {
    #[arg(long)]
    pub index: PathBuf,
    #[arg(long)]
    pub query: String,
}
