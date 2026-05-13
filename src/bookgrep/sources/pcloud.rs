#![cfg_attr(not(any(test, feature = "pcloud")), allow(dead_code))]

use serde::Deserialize;

use crate::bookgrep::{
    error::{BookgrepError, Result},
    model::{DocumentFormat, DocumentRef},
};

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct PCloudMetadata {
    pub name: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub fileid: Option<u64>,
    #[serde(default)]
    pub folderid: Option<u64>,
    #[serde(default)]
    pub isfolder: bool,
    #[serde(default)]
    pub size: Option<u64>,
    #[serde(default)]
    pub modified: Option<String>,
    #[serde(default)]
    pub contents: Vec<PCloudMetadata>,
}

#[derive(Debug, Deserialize)]
pub struct ListFolderResponse {
    #[serde(default)]
    pub result: u32,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub metadata: Option<PCloudMetadata>,
}

pub fn parse_listfolder_documents(
    raw: &str,
    extensions: Option<&[DocumentFormat]>,
) -> Result<Vec<DocumentRef>> {
    let response: ListFolderResponse =
        serde_json::from_str(raw).map_err(|err| BookgrepError::PCloud(err.to_string()))?;
    if response.result != 0 {
        return Err(BookgrepError::PCloud(
            response
                .error
                .unwrap_or_else(|| format!("result {}", response.result)),
        ));
    }
    let mut documents = Vec::new();
    if let Some(metadata) = response.metadata {
        collect_documents(&metadata, extensions, &mut documents);
    }
    Ok(documents)
}

fn collect_documents(
    metadata: &PCloudMetadata,
    extensions: Option<&[DocumentFormat]>,
    documents: &mut Vec<DocumentRef>,
) {
    if metadata.isfolder {
        for child in &metadata.contents {
            collect_documents(child, extensions, documents);
        }
        return;
    }

    let path = metadata.path.as_deref().unwrap_or(&metadata.name);
    let Some(format) = DocumentFormat::from_path(std::path::Path::new(path)) else {
        return;
    };
    if !format.is_searchable() || extensions.is_some_and(|exts| !exts.contains(&format)) {
        return;
    }

    documents.push(DocumentRef {
        source_path: path.into(),
        format,
        size: metadata.size,
        modified_unix: None,
    });
}

#[cfg(feature = "pcloud")]
mod live {
    use std::{fs, io::Write, path::PathBuf};

    use reqwest::blocking::Client;
    use serde::Deserialize;

    use super::*;
    use crate::bookgrep::{
        cache::Cache, config::Config, model::DocumentRef, sources::DocumentSource,
    };

    #[derive(Debug, Clone)]
    pub enum PCloudRoot {
        FolderId(u64),
        Path(String),
    }

    #[derive(Debug, Clone)]
    pub struct PCloudSource {
        client: Client,
        token: String,
        root: PCloudRoot,
        recursive: bool,
        extensions: Option<Vec<DocumentFormat>>,
        cache: Cache,
    }

    impl PCloudSource {
        pub fn from_search_args(
            folder_id: Option<u64>,
            path: Option<String>,
            recursive: bool,
            extensions: Option<Vec<DocumentFormat>>,
            config: &Config,
        ) -> Result<Self> {
            let token = config
                .pcloud_token
                .clone()
                .ok_or_else(|| BookgrepError::PCloud("missing BOOKGREP_PCLOUD_TOKEN".into()))?;
            let root = match (folder_id, path) {
                (Some(id), None) => PCloudRoot::FolderId(id),
                (None, Some(path)) => PCloudRoot::Path(path),
                _ => {
                    return Err(BookgrepError::PCloud(
                        "provide either pcloud folder id or pcloud path".into(),
                    ));
                }
            };
            Ok(Self {
                client: Client::new(),
                token,
                root,
                recursive,
                extensions,
                cache: Cache::new(config.cache_dir.clone())?,
            })
        }

        fn listfolder_raw(&self) -> Result<String> {
            let mut request = self
                .client
                .get("https://api.pcloud.com/listfolder")
                .bearer_auth(&self.token)
                .query(&[("recursive", if self.recursive { "1" } else { "0" })]);

            request = match &self.root {
                PCloudRoot::FolderId(id) => request.query(&[("folderid", id.to_string())]),
                PCloudRoot::Path(path) => request.query(&[("path", path.clone())]),
            };

            request
                .send()
                .and_then(|response| response.error_for_status())
                .map_err(|err| BookgrepError::PCloud(err.to_string()))?
                .text()
                .map_err(|err| BookgrepError::PCloud(err.to_string()))
        }

        fn download_to_cache(&self, document: &DocumentRef) -> Result<PathBuf> {
            let extension = document
                .source_path
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("bin");
            let key = format!(
                "{}:{:?}:{:?}",
                document.source_path.display(),
                document.size,
                document.modified_unix
            );
            let cache_path = self.cache.path_for_key(&key, extension);
            if Cache::is_valid(&cache_path, document.size) {
                return Ok(cache_path);
            }

            let link = self.get_file_link(document)?;
            let bytes = self
                .client
                .get(link)
                .send()
                .and_then(|response| response.error_for_status())
                .map_err(|err| BookgrepError::PCloud(err.to_string()))?
                .bytes()
                .map_err(|err| BookgrepError::PCloud(err.to_string()))?;
            let mut file = fs::File::create(&cache_path)
                .map_err(|err| BookgrepError::PCloud(err.to_string()))?;
            file.write_all(&bytes)
                .map_err(|err| BookgrepError::PCloud(err.to_string()))?;
            Ok(cache_path)
        }

        fn get_file_link(&self, document: &DocumentRef) -> Result<String> {
            #[derive(Deserialize)]
            struct LinkResponse {
                result: u32,
                #[serde(default)]
                error: Option<String>,
                #[serde(default)]
                hosts: Vec<String>,
                #[serde(default)]
                path: Option<String>,
            }

            let response: LinkResponse = self
                .client
                .get("https://api.pcloud.com/getfilelink")
                .bearer_auth(&self.token)
                .query(&[("path", document.source_path.display().to_string())])
                .send()
                .and_then(|response| response.error_for_status())
                .map_err(|err| BookgrepError::PCloud(err.to_string()))?
                .json()
                .map_err(|err| BookgrepError::PCloud(err.to_string()))?;
            if response.result != 0 {
                return Err(BookgrepError::PCloud(
                    response
                        .error
                        .unwrap_or_else(|| format!("result {}", response.result)),
                ));
            }
            let host = response
                .hosts
                .first()
                .ok_or_else(|| BookgrepError::PCloud("getfilelink returned no host".into()))?;
            let path = response
                .path
                .ok_or_else(|| BookgrepError::PCloud("getfilelink returned no path".into()))?;
            Ok(format!("https://{host}{path}"))
        }
    }

    impl DocumentSource for PCloudSource {
        fn list_documents(&self) -> Result<Vec<DocumentRef>> {
            let raw = self.listfolder_raw()?;
            parse_listfolder_documents(&raw, self.extensions.as_deref())
        }

        fn fetch_document(&self, document: &DocumentRef) -> Result<PathBuf> {
            self.download_to_cache(document)
        }
    }
}

#[cfg(feature = "pcloud")]
pub use live::PCloudSource;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_listfolder_response_without_real_api() {
        let raw = r#"
        {
          "result": 0,
          "metadata": {
            "isfolder": true,
            "name": "Books",
            "contents": [
              {"isfolder": false, "name": "rust.pdf", "path": "/Books/rust.pdf", "size": 10},
              {"isfolder": false, "name": "notes.txt", "path": "/Books/notes.txt", "size": 5},
              {"isfolder": false, "name": "memory.epub", "path": "/Books/memory.epub", "size": 20}
            ]
          }
        }
        "#;

        let docs = parse_listfolder_documents(raw, None).expect("documents");
        assert_eq!(docs.len(), 2);
        assert!(docs.iter().any(|doc| doc.format == DocumentFormat::Pdf));
        assert!(docs.iter().any(|doc| doc.format == DocumentFormat::Epub));
    }
}
