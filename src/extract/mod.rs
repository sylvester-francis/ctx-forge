//! Tree-sitter-based function and type extraction (feature-gated by `extract`).
//! Supports Go, Rust, Python, TypeScript, JavaScript.

mod queries;
mod runner;
pub mod scan;

use tree_sitter::Language;

pub fn extract_function(
    source: &str,
    language_name: &str,
    fn_name: &str,
) -> Result<Option<String>, String> {
    let (lang, query) = resolve_function_query(language_name)?;
    runner::extract_by_name(source, &lang, query, fn_name)
}

pub fn extract_type(
    source: &str,
    language_name: &str,
    type_name: &str,
) -> Result<Option<String>, String> {
    let (lang, query) = resolve_type_query(language_name)?;
    runner::extract_by_name(source, &lang, query, type_name)
}

pub(crate) fn resolve_function_query(
    language_name: &str,
) -> Result<(Language, &'static str), String> {
    match language_name {
        "rust" => Ok((tree_sitter_rust::LANGUAGE.into(), queries::RUST_FUNCTIONS)),
        "go" => Ok((tree_sitter_go::LANGUAGE.into(), queries::GO_FUNCTIONS)),
        "python" => Ok((
            tree_sitter_python::LANGUAGE.into(),
            queries::PYTHON_FUNCTIONS,
        )),
        "typescript" => Ok((
            tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            queries::TYPESCRIPT_FUNCTIONS,
        )),
        "javascript" => Ok((
            tree_sitter_javascript::LANGUAGE.into(),
            queries::JAVASCRIPT_FUNCTIONS,
        )),
        other => Err(format!(
            "tree-sitter extraction not supported for `{other}` (supported: rust, go, python, typescript, javascript)"
        )),
    }
}

pub(crate) fn resolve_type_query(language_name: &str) -> Result<(Language, &'static str), String> {
    match language_name {
        "rust" => Ok((tree_sitter_rust::LANGUAGE.into(), queries::RUST_TYPES)),
        "go" => Ok((tree_sitter_go::LANGUAGE.into(), queries::GO_TYPES)),
        "python" => Ok((tree_sitter_python::LANGUAGE.into(), queries::PYTHON_TYPES)),
        "typescript" => Ok((
            tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            queries::TYPESCRIPT_TYPES,
        )),
        "javascript" => Ok((
            tree_sitter_javascript::LANGUAGE.into(),
            queries::JAVASCRIPT_TYPES,
        )),
        other => Err(format!(
            "tree-sitter extraction not supported for `{other}` (supported: rust, go, python, typescript, javascript)"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_rust_function() {
        let source = r#"
fn helper() {}

fn process_check(input: &str) -> bool {
    input.len() > 0
}

fn another() {}
"#;
        let result = extract_function(source, "rust", "process_check").unwrap();
        assert!(result.is_some());
        let text = result.unwrap();
        assert!(text.contains("fn process_check"));
        assert!(text.contains("input.len()"));
    }

    #[test]
    fn extract_rust_function_not_found() {
        let source = "fn main() {}";
        let result = extract_function(source, "rust", "nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn extract_rust_struct() {
        let source = r#"
struct Config {
    host: String,
    port: u16,
}

struct Other;
"#;
        let result = extract_type(source, "rust", "Config").unwrap();
        assert!(result.is_some());
        let text = result.unwrap();
        assert!(text.contains("struct Config"));
        assert!(text.contains("host: String"));
    }

    #[test]
    fn extract_python_function() {
        let source = r#"
def helper():
    pass

def process_check(input):
    return len(input) > 0

def another():
    pass
"#;
        let result = extract_function(source, "python", "process_check").unwrap();
        assert!(result.is_some());
        let text = result.unwrap();
        assert!(text.contains("def process_check"));
    }

    #[test]
    fn extract_python_class() {
        let source = r#"
class Config:
    def __init__(self):
        self.host = "localhost"
"#;
        let result = extract_type(source, "python", "Config").unwrap();
        assert!(result.is_some());
        assert!(result.unwrap().contains("class Config"));
    }

    #[test]
    fn extract_go_function() {
        let source = r#"
package main

func ProcessCheck(input string) bool {
    return len(input) > 0
}
"#;
        let result = extract_function(source, "go", "ProcessCheck").unwrap();
        assert!(result.is_some());
        assert!(result.unwrap().contains("func ProcessCheck"));
    }

    #[test]
    fn extract_typescript_function() {
        let source = r#"
function processCheck(input: string): boolean {
    return input.length > 0;
}
"#;
        let result = extract_function(source, "typescript", "processCheck").unwrap();
        assert!(result.is_some());
        assert!(result.unwrap().contains("function processCheck"));
    }

    #[test]
    fn extract_typescript_interface() {
        let source = r#"
interface Config {
    host: string;
    port: number;
}
"#;
        let result = extract_type(source, "typescript", "Config").unwrap();
        assert!(result.is_some());
        assert!(result.unwrap().contains("interface Config"));
    }

    #[test]
    fn unsupported_language_errors() {
        assert!(extract_function("", "haskell", "main").is_err());
        assert!(extract_type("", "haskell", "Main").is_err());
    }
}
