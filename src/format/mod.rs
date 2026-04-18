//! Format dispatch: renders a list of resolved items into a string.

#![allow(dead_code)]

pub mod json;
pub mod markdown;
pub mod xml;

use crate::memory::Note;
use crate::resolve::ResolvedItem;
use crate::source::Source;

pub(crate) fn partition_docs(
    items: &[ResolvedItem],
) -> (Vec<&ResolvedItem>, Vec<&ResolvedItem>) {
    let mut docs = Vec::new();
    let mut other = Vec::new();
    for item in items {
        if matches!(item.item.source, Source::Docs(_)) {
            docs.push(item);
        } else {
            other.push(item);
        }
    }
    (docs, other)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Format {
    #[default]
    Markdown,
    Xml,
    Json,
}

impl Format {
    /// Parse a user-supplied `--format <name>` value. Accepts common
    /// aliases (`md`, `mkd` → markdown). Case-insensitive.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "markdown" | "md" | "mkd" => Some(Format::Markdown),
            "xml" => Some(Format::Xml),
            "json" => Some(Format::Json),
            _ => None,
        }
    }

    /// Short name suitable for status messages.
    pub fn name(self) -> &'static str {
        match self {
            Format::Markdown => "markdown",
            Format::Xml => "xml",
            Format::Json => "json",
        }
    }
}

pub fn render(
    format: Format,
    items: &[ResolvedItem],
    memory: &[Note],
    no_provenance: bool,
) -> String {
    match format {
        Format::Markdown => markdown::render(items, memory, no_provenance),
        Format::Xml => xml::render(items, memory, no_provenance),
        Format::Json => json::render(items, memory, no_provenance),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_accepts_canonical_names() {
        assert_eq!(Format::parse("markdown"), Some(Format::Markdown));
        assert_eq!(Format::parse("xml"), Some(Format::Xml));
        assert_eq!(Format::parse("json"), Some(Format::Json));
    }

    #[test]
    fn parse_accepts_markdown_aliases() {
        assert_eq!(Format::parse("md"), Some(Format::Markdown));
        assert_eq!(Format::parse("mkd"), Some(Format::Markdown));
    }

    #[test]
    fn parse_is_case_insensitive() {
        assert_eq!(Format::parse("XML"), Some(Format::Xml));
        assert_eq!(Format::parse("Json"), Some(Format::Json));
    }

    #[test]
    fn parse_rejects_unknown() {
        assert_eq!(Format::parse(""), None);
        assert_eq!(Format::parse("yaml"), None);
    }
}
