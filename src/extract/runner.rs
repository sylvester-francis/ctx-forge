//! Generic tree-sitter query runner. Given source code, a language, and a
//! query pattern, finds the named item and returns its text.

use tree_sitter::{Language, Parser, Query, QueryCursor, StreamingIterator};

/// Extract the text of the first definition whose @name capture matches
/// `target_name`. Returns `None` if no match is found.
pub fn extract_by_name(
    source: &str,
    language: &Language,
    query_source: &str,
    target_name: &str,
) -> Result<Option<String>, String> {
    let mut parser = Parser::new();
    parser
        .set_language(language)
        .map_err(|e| format!("failed to set language: {e}"))?;

    let tree = parser.parse(source, None).ok_or("failed to parse source")?;

    let query = Query::new(language, query_source).map_err(|e| format!("invalid query: {e}"))?;

    let name_idx = query
        .capture_names()
        .iter()
        .position(|n| *n == "name")
        .ok_or("query must have a @name capture")?;

    let definition_idx = query
        .capture_names()
        .iter()
        .position(|n| *n == "definition")
        .ok_or("query must have a @definition capture")?;

    let mut cursor = QueryCursor::new();
    let source_bytes = source.as_bytes();

    // tree-sitter 0.26 returns StreamingIterator, not Iterator.
    let mut matches = cursor.matches(&query, tree.root_node(), source_bytes);
    while let Some(m) = matches.next() {
        let name_node = m.captures.iter().find(|c| c.index as usize == name_idx);
        let def_node = m
            .captures
            .iter()
            .find(|c| c.index as usize == definition_idx);

        if let (Some(name_cap), Some(def_cap)) = (name_node, def_node) {
            let found_name = name_cap.node.utf8_text(source_bytes).unwrap_or("");

            if found_name == target_name {
                let text = def_cap
                    .node
                    .utf8_text(source_bytes)
                    .unwrap_or("")
                    .to_string();
                return Ok(Some(text));
            }
        }
    }

    Ok(None)
}
