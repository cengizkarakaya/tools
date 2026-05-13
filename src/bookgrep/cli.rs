use std::path::PathBuf;

use clap::{
    Args, ColorChoice, Parser, Subcommand, ValueEnum,
    builder::styling::{AnsiColor, Effects, Styles},
};

use super::model::DocumentFormat;

// `derive(Parser)` ve `derive(Subcommand)`, clap'in bu tiplerden CLI şeması üretmesini sağlar.
#[derive(Debug, Parser)]
#[command(
    name = "bookgrep",
    version,
    about = "Search PDF and EPUB books",
    long_about = "Search PDF and EPUB books from a local folder or pCloud source.",
    color = ColorChoice::Always,
    styles = help_styles(),
    subcommand_required = true,
    arg_required_else_help = true,
    after_help = "Examples:\n  bookgrep search --path ./books --query ownership\n  bookgrep search --path ./books --query \"borrow checker\" --recursive --metadata\n  bookgrep info --file ./books/rust.epub"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    #[command(about = "Search PDF and EPUB files")]
    Search(SearchArgs),
    #[command(about = "Show metadata and section info for one file")]
    Info(InfoArgs),
    #[command(about = "Create a search index (planned)")]
    Index(IndexArgs),
    #[command(about = "Search an existing index (planned)")]
    SearchIndex(SearchIndexArgs),
    #[command(name = "__extract", hide = true)]
    Extract(ExtractArgs),
}

#[derive(Debug, Args)]
#[command(
    after_help = "Examples:\n  bookgrep search --path ./books --query rust\n  bookgrep search --path ./books --query \"trait object\" --extension epub --recursive\n  bookgrep search --pcloud-path /Books --query ownership --metadata"
)]
pub struct SearchArgs {
    // `Option<T>`, argüman verilmezse `None`, verilirse `Some(değer)` taşır.
    #[arg(long, value_name = "DIR", help = "Search books under a local folder")]
    pub path: Option<PathBuf>,
    #[arg(long, value_name = "ID", help = "Search a pCloud folder by folder id")]
    pub pcloud_folder_id: Option<u64>,
    #[arg(long, value_name = "PATH", help = "Search a pCloud folder by path")]
    pub pcloud_path: Option<String>,
    #[arg(long, value_name = "TEXT", help = "Text or regex pattern to search")]
    pub query: String,
    #[arg(long, help = "Search folders recursively")]
    pub recursive: bool,
    #[arg(long, help = "Match uppercase and lowercase exactly")]
    pub case_sensitive: bool,
    #[arg(long, help = "Treat --query as a regular expression")]
    pub regex: bool,
    #[arg(
        long,
        default_value_t = 80,
        value_name = "CHARS",
        help = "Characters to show before and after each match"
    )]
    pub context: usize,
    #[arg(
        long,
        value_name = "COUNT",
        help = "Maximum number of documents to print"
    )]
    pub limit: Option<usize>,
    #[arg(long, help = "Write machine-readable JSON output")]
    pub json: bool,
    #[arg(
        long,
        help = "Show every matched passage, not only matching book metadata"
    )]
    pub matches: bool,
    #[arg(
        long = "extension",
        value_enum,
        value_name = "FORMAT",
        help = "Limit search to a format; can be repeated"
    )]
    extension: Vec<ExtensionArg>,
    #[arg(long, help = "Include OPF metadata when available")]
    pub metadata: bool,
    #[arg(
        long,
        default_value_t = 10,
        value_name = "SECONDS",
        help = "Seconds to allow each PDF extraction; 0 disables the timeout"
    )]
    pub extract_timeout: u64,
}

impl SearchArgs {
    pub fn extensions(&self) -> Option<Vec<DocumentFormat>> {
        if self.extension.is_empty() {
            return None;
        }

        // CLI'ye özel enum'u uygulamanın kullandığı enum'a çeviriyoruz.
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
    #[arg(long, value_name = "FILE", help = "PDF or EPUB file to inspect")]
    pub file: PathBuf,
}

#[derive(Debug, Args)]
pub struct IndexArgs {
    #[arg(long, value_name = "DIR", help = "Folder to index")]
    pub path: PathBuf,
}

#[derive(Debug, Args)]
pub struct SearchIndexArgs {
    #[arg(long, value_name = "INDEX", help = "Index file or folder to search")]
    pub index: PathBuf,
    #[arg(long, value_name = "TEXT", help = "Text to search in the index")]
    pub query: String,
}

#[derive(Debug, Args)]
pub struct ExtractArgs {
    #[arg(long)]
    pub file: PathBuf,
    #[arg(long)]
    pub metadata: bool,
    #[arg(long)]
    pub output: Option<PathBuf>,
}

fn help_styles() -> Styles {
    Styles::styled()
        .header(AnsiColor::Green.on_default() | Effects::BOLD)
        .usage(AnsiColor::Green.on_default() | Effects::BOLD)
        .literal(AnsiColor::Cyan.on_default() | Effects::BOLD)
        .placeholder(AnsiColor::Yellow.on_default())
        .error(AnsiColor::Red.on_default() | Effects::BOLD)
        .valid(AnsiColor::Cyan.on_default() | Effects::BOLD)
        .invalid(AnsiColor::Red.on_default() | Effects::BOLD)
}
