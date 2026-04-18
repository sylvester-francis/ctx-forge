//! P3 integration tests. Uses canned registry + api.github.com JSON
//! fixtures to keep tests deterministic and offline. Live live-smoke
//! (real api.github.com) is exercised manually via Task 17's smoke step.

use ctxforge::bundle::{Bundle, Item};
use ctxforge::paths::CtxforgeRoot;
use ctxforge::source::{GhResource, GhSource, Source};
use tempfile::TempDir;

fn fixture(name: &str) -> String {
    std::fs::read_to_string(std::path::Path::new("tests/fixtures/gh").join(name))
        .unwrap_or_else(|_| panic!("missing fixture: {name}"))
}

#[test]
fn gh_issue_resource_has_correct_browser_url() {
    let r = GhResource::Issue {
        owner: "foo".into(),
        repo: "bar".into(),
        number: 1,
    };
    assert_eq!(r.browser_url(), "https://github.com/foo/bar/issues/1");
}

#[test]
fn pasted_github_url_canonicalises_and_attaches() {
    let td = TempDir::new().unwrap();
    let root = CtxforgeRoot::find_or_create(td.path()).unwrap();
    let mut bundle = Bundle::new();
    let item = Item::parse_add_argument("https://github.com/tokio-rs/axum/issues/1234").unwrap();
    bundle.add(item);
    bundle.save(&root).unwrap();

    let reloaded = Bundle::load_or_default(&root).unwrap();
    let item = &reloaded.items[0];
    assert!(matches!(
        &item.source,
        Source::Gh(GhSource {
            resource: GhResource::Issue { number: 1234, .. },
        })
    ));
}

#[test]
fn forge_parsed_from_crates_io_fixture() {
    use ctxforge::gh::forge::{self, ForgeHost};
    let body = fixture("crates-axum.json");
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    let repo = v["crate"]["repository"].as_str().unwrap();
    let f = forge::parse_forge_url(repo).unwrap();
    assert_eq!(f.host, ForgeHost::GitHub);
    assert_eq!(f.path, "tokio-rs/axum");
}

#[test]
fn crates_without_repository_yields_no_forge() {
    let body = fixture("crates-no-repo.json");
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(v["crate"].get("repository").is_none());
}

#[test]
fn npm_object_form_repository_parses() {
    use ctxforge::gh::forge;
    let body = fixture("npm-next.json");
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    let url = v["repository"]["url"].as_str().unwrap();
    let f = forge::parse_forge_url(url).unwrap();
    assert_eq!(f.path, "vercel/next.js");
}

#[test]
fn pypi_project_urls_source_parses() {
    use ctxforge::gh::forge;
    let body = fixture("pypi-fastapi.json");
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    let url = v["info"]["project_urls"]["Source"].as_str().unwrap();
    let f = forge::parse_forge_url(url).unwrap();
    assert_eq!(f.path, "tiangolo/fastapi");
}

#[test]
fn token_budget_guard_rust_fixture_with_gh_enrichment_under_500() {
    use ctxforge::gh::forge::{ForgeHost, ForgeRef};
    use ctxforge::source::{DocsSource, DocsTier, Ecosystem, Source};

    let mut bundle = Bundle::new();
    let mut add = |name: &str, tier: DocsTier| {
        bundle.add(Item {
            source: Source::Docs(DocsSource {
                name: name.into(),
                version: "1.0.0".into(),
                ecosystem: Ecosystem::Rust,
                tier,
                url: format!("https://docs.rs/{name}/1.0.0/"),
                description: Some(format!("{name} description line")),
                manifest_path: Some("Cargo.toml".into()),
                forge: Some(ForgeRef {
                    host: ForgeHost::GitHub,
                    path: format!("fake/{name}"),
                    raw_url: format!("https://github.com/fake/{name}"),
                }),
            }),
            label: None,
        });
    };
    add("axum", DocsTier::Framework);
    add("sqlx", DocsTier::Database);
    add("tokio", DocsTier::AsyncRuntime);
    add("serde", DocsTier::LanguageCore);

    let td = TempDir::new().unwrap();
    let resolved = ctxforge::resolve::resolve_all(&bundle.items, td.path()).unwrap();
    let rendered =
        ctxforge::format::render(ctxforge::format::Format::Markdown, &resolved, &[], true);
    let model = ctxforge::models::lookup(ctxforge::models::DEFAULT_MODEL);
    let tokens = ctxforge::tokens::count(&rendered, &model).tokens;
    assert!(
        tokens < 500,
        "enriched stack rendered to {tokens} tokens (budget: 500)"
    );
}
