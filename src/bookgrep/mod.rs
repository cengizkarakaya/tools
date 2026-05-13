#[cfg(feature = "pcloud")]
pub mod cache;
pub mod cli;
#[cfg(feature = "pcloud")]
pub mod config;
pub mod error;
pub mod extractors;
pub mod metadata;
pub mod model;
pub mod output;
pub mod search;
pub mod sources;

use anyhow::{Context, bail};
use clap::Parser;
use rayon::prelude::*;
use tracing_subscriber::EnvFilter;

use self::cli::{Cli, Commands};
use self::extractors::{TextExtractor, epub::EpubExtractor, pdf::PdfExtractor};
use self::model::{DocumentFormat, SearchOptions};
use self::output::{OutputFormat, write_results};
use self::search::matcher::Matcher;
use self::sources::{DocumentSource, local::LocalSource};

pub fn run() -> anyhow::Result<()> {
    init_tracing();
    let cli = Cli::parse();

    match cli.command {
        Commands::Search(args) => {
            let options = SearchOptions::from(&args);
            let results = if let Some(path) = &args.path {
                let source = LocalSource::new(path.clone(), args.recursive, args.extensions());
                search_source(&source, &options, args.metadata)?
            } else if args.pcloud_folder_id.is_some() || args.pcloud_path.is_some() {
                #[cfg(feature = "pcloud")]
                {
                    let config = config::Config::load()?;
                    let source = sources::pcloud::PCloudSource::from_search_args(
                        args.pcloud_folder_id,
                        args.pcloud_path.clone(),
                        args.recursive,
                        args.extensions(),
                        &config,
                    )?;
                    search_source(&source, &options, args.metadata)?
                }
                #[cfg(not(feature = "pcloud"))]
                {
                    bail!("pCloud API support is disabled. Rebuild with `--features pcloud`.");
                }
            } else {
                bail!("Provide `--path`, `--pcloud-folder-id`, or `--pcloud-path`.");
            };

            let format = if args.json {
                OutputFormat::Json
            } else {
                OutputFormat::Terminal
            };
            write_results(&results, format)?;
        }
        Commands::Info(args) => {
            let doc = extractors::extract_file(&args.file, true)
                .with_context(|| format!("could not read {}", args.file.display()))?;
            write_results(
                &[search::result::SearchResult::from_document_info(doc)],
                OutputFormat::Terminal,
            )?;
        }
        Commands::Index(args) => {
            bail!(
                "Indexing is planned for a later version. Direct search is available now for {}.",
                args.path.display()
            );
        }
        Commands::SearchIndex(args) => {
            bail!(
                "Indexed search is planned for a later version. Requested index: {}",
                args.index.display()
            );
        }
    }

    Ok(())
}

fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .without_time()
        .try_init();
}

fn search_source<S: DocumentSource + Sync>(
    source: &S,
    options: &SearchOptions,
    include_metadata: bool,
) -> anyhow::Result<Vec<search::result::SearchResult>> {
    let matcher = Matcher::new(options)?;
    let documents = source.list_documents()?;
    let limit = options.limit.unwrap_or(usize::MAX);

    let mut results: Vec<_> = documents
        .par_iter()
        .filter_map(|doc_ref| {
            let local_path = source.fetch_document(doc_ref).ok()?;
            let format = DocumentFormat::from_path(&local_path)?;
            let extracted = match format {
                DocumentFormat::Pdf => PdfExtractor.extract_text(&local_path, include_metadata),
                DocumentFormat::Epub => EpubExtractor.extract_text(&local_path, include_metadata),
                DocumentFormat::Opf => return None,
            }
            .ok()?;

            let matches = matcher.find_matches(&extracted);
            if matches.is_empty() {
                None
            } else {
                Some(search::result::SearchResult {
                    document: extracted,
                    matches,
                })
            }
        })
        .collect();

    results.sort_by(|a, b| a.document.source_path.cmp(&b.document.source_path));
    if results.len() > limit {
        results.truncate(limit);
    }
    Ok(results)
}
