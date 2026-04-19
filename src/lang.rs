//! Map file extensions to human-readable language names.
//!
//! Used by the markdown/XML exporters to annotate code blocks.

#![allow(dead_code)]

use std::path::Path;

/// Returns the language name for a file based on its extension.
/// Unknown extensions return `"text"`.
pub fn detect(path: &Path) -> &'static str {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    match ext.as_str() {
        "rs" => "rust",
        "go" => "go",
        "py" | "pyi" => "python",
        "ts" | "tsx" => "typescript",
        "js" | "jsx" | "mjs" | "cjs" => "javascript",
        "java" => "java",
        "kt" | "kts" => "kotlin",
        "c" | "h" => "c",
        "cpp" | "cc" | "cxx" | "hpp" | "hh" | "hxx" => "cpp",
        "cs" => "csharp",
        "rb" => "ruby",
        "php" => "php",
        "swift" => "swift",
        "scala" => "scala",
        "sh" | "bash" | "zsh" => "bash",
        "fish" => "fish",
        "sql" => "sql",
        "yaml" | "yml" => "yaml",
        "toml" => "toml",
        "json" => "json",
        "xml" => "xml",
        "html" | "htm" => "html",
        "css" => "css",
        "scss" | "sass" => "scss",
        "md" | "markdown" => "markdown",
        "dockerfile" => "dockerfile",
        "makefile" | "mk" => "makefile",
        "vim" => "vim",
        "lua" => "lua",
        "zig" => "zig",
        "nim" => "nim",
        "ex" | "exs" => "elixir",
        "erl" | "hrl" => "erlang",
        "clj" | "cljs" | "cljc" => "clojure",
        "hs" => "haskell",
        "ml" | "mli" => "ocaml",
        "fs" | "fsi" | "fsx" => "fsharp",
        "dart" => "dart",
        "r" => "r",
        "jl" => "julia",
        _ => {
            if let Some(name) = path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.to_ascii_lowercase())
            {
                match name.as_str() {
                    "dockerfile" | "containerfile" => return "dockerfile",
                    "makefile" | "gnumakefile" => return "makefile",
                    ".gitignore" | ".dockerignore" => return "gitignore",
                    _ => {}
                }
            }
            "text"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn detects_common_languages() {
        assert_eq!(detect(&PathBuf::from("main.rs")), "rust");
        assert_eq!(detect(&PathBuf::from("server.go")), "go");
        assert_eq!(detect(&PathBuf::from("app.ts")), "typescript");
        assert_eq!(detect(&PathBuf::from("script.py")), "python");
    }

    #[test]
    fn detects_nested_paths() {
        assert_eq!(detect(&PathBuf::from("src/hub/server.go")), "go");
    }

    #[test]
    fn unknown_extension_is_text() {
        assert_eq!(detect(&PathBuf::from("notes.xyz")), "text");
    }

    #[test]
    fn detects_dockerfile_by_name() {
        assert_eq!(detect(&PathBuf::from("Dockerfile")), "dockerfile");
    }

    #[test]
    fn detects_makefile_by_name() {
        assert_eq!(detect(&PathBuf::from("Makefile")), "makefile");
    }

    #[test]
    fn case_insensitive_extensions() {
        assert_eq!(detect(&PathBuf::from("Main.RS")), "rust");
    }
}
