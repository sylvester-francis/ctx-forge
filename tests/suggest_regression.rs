//! P4 integration tests. Uses fixture projects in tests/fixtures/suggest/
//! to verify the full scan → compare → report pipeline.

use ctxforge::bundle::{Bundle, Item};
use ctxforge::source::{DocsSource, DocsTier, Ecosystem, FileSource, Source};
use ctxforge::suggest::{SuggestOptions, run_suggest};
use std::path::PathBuf;

fn fixture_root(name: &str) -> PathBuf {
    PathBuf::from("tests/fixtures/suggest").join(name)
}

fn file_item(p: &str) -> Item {
    Item {
        source: Source::File(FileSource {
            path: PathBuf::from(p),
        }),
        label: None,
    }
}

fn docs_item(name: &str, eco: Ecosystem) -> Item {
    Item {
        source: Source::Docs(DocsSource {
            name: name.into(),
            version: "1.0".into(),
            ecosystem: eco,
            tier: DocsTier::Library,
            url: String::new(),
            description: None,
            manifest_path: Some("Cargo.toml".into()),
            forge: None,
        }),
        label: None,
    }
}

#[test]
fn rust_mixed_missing_regex_and_tokio_and_sqlx_and_axum_no_docs_yet() {
    let root = fixture_root("rust-mixed");
    let mut bundle = Bundle::new();
    bundle.items.push(file_item("src/main.rs"));
    bundle.items.push(file_item("src/db.rs"));

    let report = run_suggest(&bundle, &root, &SuggestOptions::default()).unwrap();
    let names: Vec<String> = report.missing.iter().map(|m| m.name.clone()).collect();
    assert!(names.contains(&"axum".to_string()));
    assert!(names.contains(&"tokio".to_string()));
    assert!(names.contains(&"sqlx".to_string()));
    assert!(names.contains(&"regex".to_string()));
    assert!(report.stale.is_empty());
}

#[test]
fn rust_mixed_adding_sqlx_docs_removes_from_missing_but_regex_remains() {
    let root = fixture_root("rust-mixed");
    let mut bundle = Bundle::new();
    bundle.items.push(file_item("src/main.rs"));
    bundle.items.push(file_item("src/db.rs"));
    bundle.items.push(docs_item("sqlx", Ecosystem::Rust));
    bundle.items.push(docs_item("axum", Ecosystem::Rust));
    bundle.items.push(docs_item("tokio", Ecosystem::Rust));

    let report = run_suggest(&bundle, &root, &SuggestOptions::default()).unwrap();
    let names: Vec<String> = report.missing.iter().map(|m| m.name.clone()).collect();
    assert!(!names.contains(&"sqlx".to_string()));
    assert!(!names.contains(&"axum".to_string()));
    assert!(!names.contains(&"tokio".to_string()));
    assert!(names.contains(&"regex".to_string()));
}

#[test]
fn rust_mixed_stale_flagged_when_docs_exists_without_import() {
    let root = fixture_root("rust-mixed");
    let mut bundle = Bundle::new();
    bundle.items.push(file_item("src/main.rs"));
    bundle.items.push(file_item("src/db.rs"));
    bundle.items.push(docs_item("unused-crate", Ecosystem::Rust));

    let report = run_suggest(&bundle, &root, &SuggestOptions::default()).unwrap();
    let stale_names: Vec<String> = report.stale.iter().map(|s| s.name.clone()).collect();
    assert!(stale_names.contains(&"unused-crate".to_string()));
}

#[test]
fn js_mixed_detects_tanstack_as_missing_and_skips_node_builtin() {
    let root = fixture_root("js-mixed");
    let mut bundle = Bundle::new();
    bundle.items.push(file_item("src/app.ts"));
    bundle.items.push(docs_item("react", Ecosystem::Js));

    let report = run_suggest(&bundle, &root, &SuggestOptions::default()).unwrap();
    let names: Vec<String> = report.missing.iter().map(|m| m.name.clone()).collect();
    assert!(names.contains(&"@tanstack/react-query".to_string()));
    assert!(!names.iter().any(|n| n.contains("fs")));
}

#[test]
fn python_mixed_detects_pydantic_missing_skips_os() {
    let root = fixture_root("py-mixed");
    let mut bundle = Bundle::new();
    bundle.items.push(file_item("src/server.py"));
    bundle.items.push(docs_item("fastapi", Ecosystem::Python));

    let report = run_suggest(&bundle, &root, &SuggestOptions::default()).unwrap();
    let names: Vec<String> = report.missing.iter().map(|m| m.name.clone()).collect();
    assert!(names.contains(&"pydantic".to_string()));
    assert!(!names.contains(&"os".to_string()));
}

#[test]
fn go_mixed_detects_errgroup_missing_skips_fmt() {
    let root = fixture_root("go-mixed");
    let mut bundle = Bundle::new();
    bundle.items.push(file_item("main.go"));
    bundle
        .items
        .push(docs_item("github.com/gin-gonic/gin", Ecosystem::Go));

    let report = run_suggest(&bundle, &root, &SuggestOptions::default()).unwrap();
    let names: Vec<String> = report.missing.iter().map(|m| m.name.clone()).collect();
    assert!(names.contains(&"golang.org/x/sync/errgroup".to_string()));
    assert!(!names.iter().any(|n| *n == "fmt"));
}

#[test]
fn all_flag_scans_files_not_in_bundle() {
    let root = fixture_root("rust-mixed");
    let bundle = Bundle::new();

    let default_report = run_suggest(&bundle, &root, &SuggestOptions::default()).unwrap();
    assert!(default_report.missing.is_empty());

    let all_report = run_suggest(
        &bundle,
        &root,
        &SuggestOptions {
            scan_all_project: true,
            missing_only: false,
        },
    )
    .unwrap();
    assert!(!all_report.missing.is_empty());
}
