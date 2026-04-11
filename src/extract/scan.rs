//! Scan an entire project for function and type names.
//!
//! Powers the TUI's `f` and `t` pickers: walks every file the project's
//! gitignore-aware walker exposes, runs the relevant tree-sitter query,
//! and collects every captured `@name` along with its file path.

use crate::lang;
use crate::walk;
use std::path::{Path, PathBuf};
use tree_sitter::{Language, Parser, Query, QueryCursor, StreamingIterator};

/// A discovered symbol: name + file path (relative to project root).
pub type Symbol = (String, PathBuf);

/// Scan all files in the project for function definitions.
pub fn scan_functions(project_root: &Path) -> Vec<Symbol> {
    scan_symbols(project_root, super::resolve_function_query)
}

/// Scan all files in the project for type definitions.
pub fn scan_types(project_root: &Path) -> Vec<Symbol> {
    scan_symbols(project_root, super::resolve_type_query)
}

fn scan_symbols(
    project_root: &Path,
    resolver: fn(&str) -> Result<(Language, &'static str), String>,
) -> Vec<Symbol> {
    // Use the project's existing gitignore-aware walker.
    let files = walk::expand("**/*", project_root, &[]).unwrap_or_default();
    let mut results = Vec::new();

    for rel in files {
        let language_name = lang::detect(&rel);
        if language_name == "text" {
            continue;
        }

        let Ok((lang, query_src)) = resolver(language_name) else {
            continue;
        };

        let abs = project_root.join(&rel);
        let Ok(source) = std::fs::read_to_string(&abs) else {
            continue;
        };

        let mut parser = Parser::new();
        if parser.set_language(&lang).is_err() {
            continue;
        }
        let Some(tree) = parser.parse(&source, None) else {
            continue;
        };
        let Ok(query) = Query::new(&lang, query_src) else {
            continue;
        };

        let Some(name_idx) = query.capture_names().iter().position(|n| *n == "name") else {
            continue;
        };

        let mut cursor = QueryCursor::new();
        let source_bytes = source.as_bytes();
        let mut matches = cursor.matches(&query, tree.root_node(), source_bytes);
        while let Some(m) = matches.next() {
            if let Some(cap) = m.captures.iter().find(|c| c.index as usize == name_idx) {
                if let Ok(name) = cap.node.utf8_text(source_bytes) {
                    results.push((name.to_string(), rel.clone()));
                }
            }
        }
    }

    results.sort_by(|a, b| a.0.cmp(&b.0));
    results
}
