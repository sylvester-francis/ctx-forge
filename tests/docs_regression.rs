//! Integration tests for P1 library docs gatherer: fixture projects,
//! full detection runs, and a token-budget guard.

use ctxforge::bundle::Bundle;
use ctxforge::docs::{DetectMode, DetectOptions, run_detect};
use ctxforge::paths::CtxforgeRoot;
use ctxforge::source::{DocsTier, Ecosystem, Source};
use tempfile::TempDir;

fn copy_fixture(fixture: &str) -> (TempDir, CtxforgeRoot) {
    let td = TempDir::new().unwrap();
    let src = std::path::Path::new("tests/fixtures/docs").join(fixture);
    copy_tree(&src, td.path());
    let root = CtxforgeRoot::find_or_create(td.path()).unwrap();
    (td, root)
}

fn copy_tree(src: &std::path::Path, dst: &std::path::Path) {
    for entry in std::fs::read_dir(src).unwrap().flatten() {
        let dst_path = dst.join(entry.file_name());
        if entry.path().is_dir() {
            std::fs::create_dir_all(&dst_path).unwrap();
            copy_tree(&entry.path(), &dst_path);
        } else {
            std::fs::copy(entry.path(), &dst_path).unwrap();
        }
    }
}

#[test]
fn detect_rust_fixture_classifies_correctly() {
    let (_td, root) = copy_fixture("rust-axum-sqlx");
    let mut bundle = Bundle::new();
    let options = DetectOptions {
        mode: DetectMode::All,
        include_library_tier: false,
    };
    let report = run_detect(&mut bundle, root.project_root(), &options, None).unwrap();
    assert!(!report.manifests.is_empty());

    let by_name: std::collections::HashMap<String, &ctxforge::source::DocsSource> = bundle
        .items
        .iter()
        .filter_map(|i| match &i.source {
            Source::Docs(d) => Some((d.name.clone(), d)),
            _ => None,
        })
        .collect();

    let axum = by_name.get("axum").unwrap();
    assert_eq!(axum.tier, DocsTier::Framework);
    assert_eq!(axum.version, "0.7.5");
    assert_eq!(axum.url, "https://docs.rs/axum/0.7.5/");

    let sqlx = by_name.get("sqlx").unwrap();
    assert_eq!(sqlx.tier, DocsTier::Database);

    let tokio = by_name.get("tokio").unwrap();
    assert_eq!(tokio.tier, DocsTier::AsyncRuntime);

    let anyhow = by_name.get("anyhow").unwrap();
    assert_eq!(anyhow.tier, DocsTier::LanguageCore);
}

#[test]
fn detect_js_fixture() {
    let (_td, root) = copy_fixture("js-next-prisma");
    let mut bundle = Bundle::new();
    let options = DetectOptions {
        mode: DetectMode::All,
        include_library_tier: false,
    };
    run_detect(&mut bundle, root.project_root(), &options, None).unwrap();

    assert!(bundle.items.iter().any(|i| matches!(&i.source,
        Source::Docs(d) if d.name == "next" && d.version == "14.2.5")));
    assert!(bundle.items.iter().any(|i| matches!(&i.source,
        Source::Docs(d) if d.name == "prisma" && d.tier == DocsTier::Database)));
}

#[test]
fn detect_python_fixture() {
    let (_td, root) = copy_fixture("python-fastapi");
    let mut bundle = Bundle::new();
    let options = DetectOptions {
        mode: DetectMode::All,
        include_library_tier: false,
    };
    run_detect(&mut bundle, root.project_root(), &options, None).unwrap();
    assert!(bundle.items.iter().any(|i| matches!(&i.source,
        Source::Docs(d) if d.name == "fastapi" && d.version == "0.115.0")));
}

#[test]
fn detect_go_fixture_filters_indirect() {
    let (_td, root) = copy_fixture("go-gin-pgx");
    let mut bundle = Bundle::new();
    let options = DetectOptions {
        mode: DetectMode::All,
        include_library_tier: false,
    };
    run_detect(&mut bundle, root.project_root(), &options, None).unwrap();

    assert!(bundle.items.iter().any(|i| matches!(&i.source,
        Source::Docs(d) if d.name == "github.com/gin-gonic/gin")));
    assert!(!bundle.items.iter().any(|i| matches!(&i.source,
        Source::Docs(d) if d.name.contains("sonic"))));
}

#[test]
fn monorepo_detects_both_ecosystems() {
    let (_td, root) = copy_fixture("monorepo-rust-js");
    let mut bundle = Bundle::new();
    let options = DetectOptions {
        mode: DetectMode::All,
        include_library_tier: false,
    };
    run_detect(&mut bundle, root.project_root(), &options, None).unwrap();

    let has_rust = bundle.items.iter().any(|i| matches!(&i.source,
        Source::Docs(d) if d.ecosystem == Ecosystem::Rust));
    let has_js = bundle.items.iter().any(|i| matches!(&i.source,
        Source::Docs(d) if d.ecosystem == Ecosystem::Js));
    assert!(has_rust, "expected Rust deps from crates/api");
    assert!(has_js, "expected JS deps from apps/web");
}

#[test]
fn token_budget_guard_rust_fixture_stays_under_300() {
    let (_td, root) = copy_fixture("rust-axum-sqlx");
    let mut bundle = Bundle::new();
    let options = DetectOptions {
        mode: DetectMode::All,
        include_library_tier: false,
    };
    run_detect(&mut bundle, root.project_root(), &options, None).unwrap();

    let resolved =
        ctxforge::resolve::resolve_all(&bundle.items, root.project_root()).unwrap();
    let rendered =
        ctxforge::format::render(ctxforge::format::Format::Markdown, &resolved, &[], true);

    let model = ctxforge::models::lookup(ctxforge::models::DEFAULT_MODEL);
    let tokens = ctxforge::tokens::count(&rendered, &model).tokens;

    assert!(
        tokens < 300,
        "Project stack section for rust-axum-sqlx fixture rendered to {tokens} tokens (budget: 300)",
    );
}
