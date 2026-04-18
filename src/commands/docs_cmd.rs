//! `ctxforge docs` subcommand handlers.

use crate::bundle::{Bundle, Item};
use crate::cache::ContentCache;
use crate::docs::{self, DetectMode, DetectOptions};
use crate::error::{CtxforgeError, Result};
use crate::output;
use crate::paths::{self, CtxforgeRoot};
use crate::source::{Ecosystem, Source};
use std::path::PathBuf;

pub fn detect(root: &CtxforgeRoot, all: bool, path: Option<PathBuf>) -> Result<()> {
    let mut bundle = Bundle::load_or_default(root)?;
    let project_root = root.project_root();

    let mode = match path {
        Some(p) => {
            let abs = if p.is_absolute() {
                p
            } else {
                project_root.join(&p)
            };
            DetectMode::Explicit(abs)
        }
        None if bundle
            .items
            .iter()
            .any(|i| i.source.display_path().is_some()) =>
        {
            DetectMode::Scoped
        }
        None => DetectMode::All,
    };

    let cache = paths::global_cache_dir().and_then(|d| ContentCache::open(d).ok());
    let options = DetectOptions {
        mode,
        include_library_tier: all,
    };
    let report = docs::run_detect(&mut bundle, project_root, &options, cache.as_ref())?;

    bundle.save(root)?;
    output::success(&format!(
        "docs detect: scanned {} manifest(s), added {} item(s), skipped {}",
        report.manifests.len(),
        report.added,
        report.skipped,
    ));
    for w in &report.warnings {
        output::warn(w);
    }
    Ok(())
}

pub fn add(root: &CtxforgeRoot, name: String, ecosystem: Option<String>) -> Result<()> {
    let eco = match ecosystem {
        Some(s) => Ecosystem::from_str(&s).ok_or_else(|| {
            CtxforgeError::Msg(format!(
                "unknown ecosystem `{s}` (valid: rust, js, python, go)"
            ))
        })?,
        None => infer_ecosystem_from_bundle(root, &name)?,
    };

    let mut bundle = Bundle::load_or_default(root)?;
    let registry = docs::registry::Registry::builtin();
    let tier = registry.classify(eco, &name);

    // MVP for manual add: no version resolution from manifest — use "latest"
    // Rust / Python; skip JS / Go (no reliable "latest" URL).
    let version = match eco {
        Ecosystem::Rust | Ecosystem::Python => "latest".to_string(),
        Ecosystem::Js | Ecosystem::Go => {
            return Err(CtxforgeError::Msg(
                "ctxforge docs add for JS/Go requires an existing detected version — run `ctxforge docs detect` first".into(),
            ));
        }
    };
    let url = docs::resolve::canonical_url(eco, &name, &version);
    let cache = paths::global_cache_dir().and_then(|d| ContentCache::open(d).ok());
    let description = cache.and_then(|c| docs::describe::fetch_description(&c, eco, &name));

    bundle.add(Item {
        source: Source::Docs(crate::source::DocsSource {
            name: name.clone(),
            version,
            ecosystem: eco,
            tier,
            url,
            description,
            manifest_path: None,
            forge: None,
        }),
        label: None,
    });
    bundle.save(root)?;
    output::success(&format!("added docs: {name} ({})", eco.display_name()));
    Ok(())
}

pub fn rm(root: &CtxforgeRoot, name: String) -> Result<()> {
    let mut bundle = Bundle::load_or_default(root)?;
    let before = bundle.items.len();
    bundle
        .items
        .retain(|item| !matches!(&item.source, Source::Docs(d) if d.name == name));
    let removed = before - bundle.items.len();
    bundle.save(root)?;
    if removed == 0 {
        output::warn(&format!("no docs item named `{name}` in bundle"));
    } else {
        output::success(&format!("removed {removed} docs item(s)"));
    }
    Ok(())
}

pub fn list(root: &CtxforgeRoot) -> Result<()> {
    let bundle = Bundle::load_or_default(root)?;
    let mut docs_items: Vec<_> = bundle
        .items
        .iter()
        .filter_map(|item| match &item.source {
            Source::Docs(d) => Some(d),
            _ => None,
        })
        .collect();
    if docs_items.is_empty() {
        println!("(no docs items in bundle)");
        return Ok(());
    }
    docs_items.sort_by(|a, b| {
        a.ecosystem
            .as_str()
            .cmp(b.ecosystem.as_str())
            .then_with(|| a.name.cmp(&b.name))
    });
    for d in docs_items {
        println!(
            "  [{}] {} {} — {} ({:?})",
            d.ecosystem.as_str(),
            d.name,
            d.version,
            d.url,
            d.tier,
        );
    }
    Ok(())
}

pub fn refresh(root: &CtxforgeRoot) -> Result<()> {
    let mut bundle = Bundle::load_or_default(root)?;
    let project_root = root.project_root();
    let registry = docs::registry::Registry::builtin();
    let mut updated = 0;

    // Re-parse each unique manifest referenced by a docs item.
    let mut manifest_deps: std::collections::HashMap<
        (PathBuf, Ecosystem),
        Vec<crate::docs::parsers::DetectedDep>,
    > = std::collections::HashMap::new();

    for item in &bundle.items {
        let Source::Docs(d) = &item.source else {
            continue;
        };
        let Some(mp) = &d.manifest_path else {
            continue;
        };
        let abs = project_root.join(mp);
        let key = (abs.clone(), d.ecosystem);
        manifest_deps
            .entry(key)
            .or_insert_with(|| match d.ecosystem {
                Ecosystem::Rust => docs::parsers::cargo::parse(&abs).unwrap_or_default(),
                _ => Vec::new(),
            });
    }

    let cache = paths::global_cache_dir().and_then(|d| ContentCache::open(d).ok());
    for item in &mut bundle.items {
        let Source::Docs(d) = &mut item.source else {
            continue;
        };
        let Some(mp) = &d.manifest_path else {
            continue;
        };
        let abs = project_root.join(mp);
        let Some(deps) = manifest_deps.get(&(abs, d.ecosystem)) else {
            continue;
        };
        if let Some(fresh) = deps.iter().find(|x| x.name == d.name) {
            if fresh.version != d.version {
                d.version = fresh.version.clone();
                d.url = docs::resolve::canonical_url(d.ecosystem, &d.name, &d.version);
                d.tier = registry.classify(d.ecosystem, &d.name);
                d.description = cache
                    .as_ref()
                    .and_then(|c| docs::describe::fetch_description(c, d.ecosystem, &d.name));
                updated += 1;
            }
        }
    }
    bundle.save(root)?;
    output::success(&format!("refreshed {updated} docs item(s)"));
    Ok(())
}

fn infer_ecosystem_from_bundle(root: &CtxforgeRoot, name: &str) -> Result<Ecosystem> {
    let bundle = Bundle::load_or_default(root)?;
    let ecosystems: std::collections::HashSet<Ecosystem> = bundle
        .items
        .iter()
        .filter_map(|i| match &i.source {
            Source::Docs(d) => Some(d.ecosystem),
            _ => None,
        })
        .collect();
    match ecosystems.len() {
        0 => Err(CtxforgeError::Msg(format!(
            "no docs items in bundle yet — pass --ecosystem to disambiguate `{name}`"
        ))),
        1 => Ok(*ecosystems.iter().next().unwrap()),
        _ => Err(CtxforgeError::Msg(format!(
            "multiple ecosystems detected ({}) — pass --ecosystem for `{name}`",
            ecosystems
                .iter()
                .map(|e| e.as_str())
                .collect::<Vec<_>>()
                .join(", "),
        ))),
    }
}
