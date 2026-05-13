// `cfg(feature = ...)` kodun sadece ilgili Cargo özelliği açıksa derlenmesini sağlar.
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

use std::{
    env, fs,
    io::{self, Write},
    panic,
    path::{Path, PathBuf},
    process::{self, Command, Stdio},
    sync::{
        Once,
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, bail};
use clap::Parser;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use tracing_subscriber::EnvFilter;

use self::cli::{Cli, Commands};
use self::error::{BookgrepError, Result as BookgrepResult};
use self::extractors::{
    TextExtractor,
    epub::EpubExtractor,
    pdf::{self, PdfExtractor},
};
use self::model::{DocumentFormat, ExtractedDocument, SearchOptions};
use self::output::{OutputFormat, write_results};
use self::search::matcher::Matcher;
use self::sources::{DocumentSource, local::LocalSource};

static NEXT_EXTRACT_ID: AtomicU64 = AtomicU64::new(1);
static OCR_WARNING_SHOWN: AtomicBool = AtomicBool::new(false);
const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const GREEN: &str = "\x1b[32m";
const CYAN: &str = "\x1b[36m";
const YELLOW: &str = "\x1b[33m";

#[derive(Debug, Serialize, Deserialize)]
struct ExtractResponse {
    document: Option<ExtractedDocument>,
    error: Option<String>,
}

pub fn run() -> anyhow::Result<()> {
    // Komut satırı argümanlarını okuyup ilgili bookgrep komutunu çalıştırır.
    init_tracing();
    install_pdf_panic_hook();
    // `clap::Parser` derive makrosu, struct/enum tanımlarından CLI parser üretir.
    let cli = Cli::parse();

    match cli.command {
        Commands::Search(args) => {
            let options = SearchOptions::from(&args);
            // Arama kaynağı yerel dizin veya pCloud seçeneklerine göre seçilir.
            // `if let Some(...)` Option içindeki değeri güvenli şekilde açar.
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
                    // `?` hata varsa fonksiyondan hemen döner, varsa sonucu açar.
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
                OutputFormat::Terminal {
                    show_matches: args.matches,
                }
            };
            write_results(&results, format)?;
        }
        Commands::Info(args) => {
            // Tek dosya için metin/metadata çıkarır, eşleşme araması yapmaz.
            let doc = extractors::extract_file(&args.file, true)
                .with_context(|| format!("could not read {}", args.file.display()))?;
            write_results(
                &[search::result::SearchResult::from_document_info(doc)],
                OutputFormat::Terminal {
                    show_matches: false,
                },
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
        Commands::Extract(args) => {
            if let Some(output) = args.output {
                let response = match extractors::extract_file(&args.file, args.metadata) {
                    Ok(document) => ExtractResponse {
                        document: Some(document),
                        error: None,
                    },
                    Err(BookgrepError::Ocr(message)) => ExtractResponse {
                        document: None,
                        error: Some(message),
                    },
                    Err(err) => ExtractResponse {
                        document: None,
                        error: Some(err.to_string()),
                    },
                };
                fs::write(output, serde_json::to_vec(&response)?)?;
            } else {
                let document = extractors::extract_file(&args.file, args.metadata)?;
                let stdout = io::stdout();
                serde_json::to_writer(stdout.lock(), &document)?;
                writeln!(io::stdout())?;
            }
        }
    }

    Ok(())
}

fn init_tracing() {
    // RUST_LOG ayarlıysa tanılama loglarını etkinleştirir.
    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .without_time()
        .try_init();
}

fn install_pdf_panic_hook() {
    static INSTALL: Once = Once::new();
    INSTALL.call_once(|| {
        let default_hook = panic::take_hook();
        panic::set_hook(Box::new(move |info| {
            if pdf::is_pdf_extraction_active() {
                tracing::debug!("skipping PDF after extractor panic: {info}");
                return;
            }
            default_hook(info);
        }));
    });
}

fn search_source<S: DocumentSource + Sync>(
    source: &S,
    options: &SearchOptions,
    include_metadata: bool,
) -> anyhow::Result<Vec<search::result::SearchResult>> {
    // Generic `S`, hem yerel hem uzak kaynakların aynı fonksiyonla aranmasını sağlar.
    // `Sync` sınırı, Rayon paralel iterator'larının kaynağı thread'ler arasında paylaşabilmesi içindir.
    let matcher = Matcher::new(options)?;
    let documents = source.list_documents()?;
    let limit = options.limit.unwrap_or(usize::MAX);
    let processed = AtomicUsize::new(0);
    let total = documents.len();
    if total > 0 {
        eprintln!("{GREEN}{BOLD}bookgrep{RESET} {DIM}scanning {total} document(s){RESET}");
    }

    // Dosyalar paralel işlenir; okunamayan veya desteklenmeyenler atlanır.
    let mut results: Vec<_> = documents
        .par_iter()
        .filter_map(|doc_ref| {
            let current = processed.fetch_add(1, Ordering::Relaxed) + 1;
            if total > 0 {
                eprint!("\r{CYAN}{BOLD}progress{RESET} {YELLOW}{current}/{total}{RESET}");
                let _ = io::stderr().flush();
            }
            // Hatalı belgeyi tüm aramayı durdurmadan atlarız; debug log isteyen dosyayı görebilir.
            let local_path = match source.fetch_document(doc_ref) {
                Ok(path) => path,
                Err(err) => {
                    tracing::debug!(
                        error = %err,
                        path = %doc_ref.source_path.display(),
                        "skipping document fetch failure"
                    );
                    return None;
                }
            };
            let format = DocumentFormat::from_path(&local_path)?;
            let extracted = extract_document(
                &local_path,
                format,
                include_metadata,
                options.extract_timeout_secs,
            );
            let extracted = match extracted {
                Ok(document) => document,
                Err(err) => {
                    if let BookgrepError::Ocr(message) = &err
                        && !OCR_WARNING_SHOWN.swap(true, Ordering::Relaxed)
                    {
                        eprintln!(
                            "{YELLOW}{BOLD}OCR fallback unavailable{RESET} {DIM}{message}{RESET}"
                        );
                    }
                    tracing::debug!(
                        error = %err,
                        path = %local_path.display(),
                        "skipping document extraction failure"
                    );
                    return None;
                }
            };

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

    if total > 0 {
        eprintln!();
    }

    results.sort_by(|a, b| a.document.source_path.cmp(&b.document.source_path));
    // Çıktı deterministik kalır ve istenirse belge sayısı sınırlandırılır.
    if results.len() > limit {
        results.truncate(limit);
    }
    Ok(results)
}

fn extract_document(
    path: &Path,
    format: DocumentFormat,
    include_metadata: bool,
    timeout_secs: u64,
) -> BookgrepResult<ExtractedDocument> {
    match format {
        DocumentFormat::Pdf if timeout_secs > 0 => {
            extract_pdf_in_child(path, include_metadata, Duration::from_secs(timeout_secs))
        }
        DocumentFormat::Pdf => PdfExtractor.extract_text(path, include_metadata),
        DocumentFormat::Epub => EpubExtractor.extract_text(path, include_metadata),
        DocumentFormat::Opf => Err(BookgrepError::UnsupportedFormat(path.to_path_buf())),
    }
}

fn extract_pdf_in_child(
    path: &Path,
    include_metadata: bool,
    timeout: Duration,
) -> BookgrepResult<ExtractedDocument> {
    let output = extract_output_path();
    let mut command =
        Command::new(env::current_exe().map_err(|err| BookgrepError::Source(err.to_string()))?);
    command
        .arg("__extract")
        .arg("--file")
        .arg(path)
        .arg("--output")
        .arg(&output)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if include_metadata {
        command.arg("--metadata");
    }

    let mut child = command
        .spawn()
        .map_err(|_| BookgrepError::PdfExtraction(path.to_path_buf()))?;
    let started = Instant::now();
    loop {
        match child
            .try_wait()
            .map_err(|_| BookgrepError::PdfExtraction(path.to_path_buf()))?
        {
            Some(status) if status.success() => {
                let raw = fs::read(&output)
                    .map_err(|_| BookgrepError::PdfExtraction(path.to_path_buf()))?;
                let _ = fs::remove_file(&output);
                let response: ExtractResponse = serde_json::from_slice(&raw)
                    .map_err(|_| BookgrepError::PdfExtraction(path.to_path_buf()))?;
                return match (response.document, response.error) {
                    (Some(document), _) => Ok(document),
                    (None, Some(error)) => Err(BookgrepError::Ocr(error)),
                    (None, None) => Err(BookgrepError::PdfExtraction(path.to_path_buf())),
                };
            }
            Some(_) => {
                let _ = fs::remove_file(&output);
                return Err(BookgrepError::PdfExtraction(path.to_path_buf()));
            }
            None if started.elapsed() >= timeout => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = fs::remove_file(&output);
                tracing::debug!(
                    path = %path.display(),
                    timeout_secs = timeout.as_secs(),
                    "skipping PDF after extraction timeout"
                );
                return Err(BookgrepError::PdfExtraction(path.to_path_buf()));
            }
            None => thread::sleep(Duration::from_millis(25)),
        }
    }
}

fn extract_output_path() -> PathBuf {
    let id = NEXT_EXTRACT_ID.fetch_add(1, Ordering::Relaxed);
    env::temp_dir().join(format!("bookgrep-extract-{}-{id}.json", process::id()))
}
