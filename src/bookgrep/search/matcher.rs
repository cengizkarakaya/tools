use regex::{Regex, RegexBuilder};

use crate::bookgrep::{
    error::{BookgrepError, Result},
    model::{ExtractedDocument, SearchMatch, SearchOptions},
};

#[derive(Debug)]
pub struct Matcher {
    regex: Regex,
    context: usize,
}

impl Matcher {
    pub fn new(options: &SearchOptions) -> Result<Self> {
        let pattern = if options.regex {
            options.query.clone()
        } else {
            regex::escape(&options.query)
        };
        let regex = RegexBuilder::new(&pattern)
            .case_insensitive(!options.case_sensitive)
            .build()
            .map_err(|err| BookgrepError::InvalidSearch(err.to_string()))?;
        Ok(Self {
            regex,
            context: options.context,
        })
    }

    pub fn find_matches(&self, document: &ExtractedDocument) -> Vec<SearchMatch> {
        let mut matches = Vec::new();
        for section in &document.sections {
            for hit in self.regex.find_iter(&section.text) {
                let before_start =
                    floor_char_boundary(&section.text, hit.start().saturating_sub(self.context));
                let after_end = ceil_char_boundary(
                    &section.text,
                    (hit.end() + self.context).min(section.text.len()),
                );
                matches.push(SearchMatch {
                    before: section.text[before_start..hit.start()].trim().to_owned(),
                    text: hit.as_str().to_owned(),
                    after: section.text[hit.end()..after_end].trim().to_owned(),
                    page: section.ordinal.filter(|_| {
                        section
                            .label
                            .as_deref()
                            .is_some_and(|label| label.starts_with("Page "))
                    }),
                    chapter: section
                        .label
                        .as_ref()
                        .filter(|label| !label.starts_with("Page "))
                        .cloned(),
                });
            }
        }
        matches
    }
}

fn floor_char_boundary(text: &str, mut index: usize) -> usize {
    while !text.is_char_boundary(index) {
        index -= 1;
    }
    index
}

fn ceil_char_boundary(text: &str, mut index: usize) -> usize {
    while !text.is_char_boundary(index) {
        index += 1;
    }
    index
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::bookgrep::model::{DocumentFormat, DocumentMetadata, TextSection};

    use super::*;

    fn doc(text: &str) -> ExtractedDocument {
        ExtractedDocument {
            source_path: PathBuf::from("book.epub"),
            format: DocumentFormat::Epub,
            metadata: DocumentMetadata::default(),
            sections: vec![TextSection {
                label: Some("Memory".into()),
                ordinal: Some(1),
                text: text.into(),
            }],
        }
    }

    #[test]
    fn finds_case_insensitive_phrase_with_context() {
        let options = SearchOptions {
            query: "ownership model".into(),
            case_sensitive: false,
            regex: false,
            context: 11,
            limit: None,
        };
        let matcher = Matcher::new(&options).expect("matcher");
        let matches = matcher.find_matches(&doc("Rust's Ownership Model guarantees safety."));
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].text, "Ownership Model");
        assert_eq!(matches[0].before, "Rust's");
        assert_eq!(matches[0].after, "guarantees");
    }

    #[test]
    fn honors_case_sensitive_search() {
        let options = SearchOptions {
            query: "ownership".into(),
            case_sensitive: true,
            regex: false,
            context: 5,
            limit: None,
        };
        let matcher = Matcher::new(&options).expect("matcher");
        assert!(matcher.find_matches(&doc("Ownership ownership")).len() == 1);
    }

    #[test]
    fn supports_regex_search() {
        let options = SearchOptions {
            query: "own[a-z]+".into(),
            case_sensitive: false,
            regex: true,
            context: 0,
            limit: None,
        };
        let matcher = Matcher::new(&options).expect("matcher");
        assert_eq!(matcher.find_matches(&doc("ownership")).len(), 1);
    }
}
