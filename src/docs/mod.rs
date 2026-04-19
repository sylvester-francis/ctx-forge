//! Library docs gatherer: scans manifest + lock files, classifies deps,
//! resolves canonical doc URLs, and attaches DocsSource items to the bundle.

pub mod classify;
pub mod describe;
pub mod detect;
pub mod parsers;
pub mod registry;
pub mod resolve;

use crate::bundle::{Bundle, Item};
use crate::cache::ContentCache;
use crate::docs::detect::DetectedManifest;
use crate::docs::parsers::cargo;
use crate::docs::registry::Registry;
use crate::error::Result;
use crate::source::{DocsSource, Ecosystem, Source};
use std::path::Path;

pub enum DetectMode {
    Scoped,
    All,
    Explicit(std::path::PathBuf),
}

pub struct DetectOptions {
    pub mode: DetectMode,
    /// If true, Library-tier deps are also attached (otherwise they only
    /// count toward the teaser line at render time).
    pub include_library_tier: bool,
}

pub struct DetectReport {
    pub added: usize,
    pub skipped: usize,
    pub manifests: Vec<DetectedManifest>,
    pub warnings: Vec<String>,
}

/// Scan, classify, resolve URLs + descriptions, attach DocsSource items
/// to the bundle (deduped by canonical URI). Does not persist the bundle.
pub fn run_detect(
    bundle: &mut Bundle,
    project_root: &Path,
    options: &DetectOptions,
    cache: Option<&ContentCache>,
) -> Result<DetectReport> {
    let registry = Registry::builtin();
    let manifests = match &options.mode {
        DetectMode::Scoped => detect::scoped(bundle, project_root),
        DetectMode::All => detect::full_scan(project_root),
        DetectMode::Explicit(p) => detect::explicit(p).into_iter().collect(),
    };

    let mut report = DetectReport {
        added: 0,
        skipped: 0,
        manifests: manifests.clone(),
        warnings: Vec::new(),
    };

    for m in &manifests {
        let deps = match m.ecosystem {
            Ecosystem::Rust => cargo::parse(&m.path).unwrap_or_default(),
            Ecosystem::Js => parsers::npm::parse(&m.path).unwrap_or_default(),
            Ecosystem::Python => parsers::python::parse(&m.path).unwrap_or_default(),
            Ecosystem::Go => parsers::go::parse(&m.path).unwrap_or_default(),
        };
        for dep in deps {
            let tier = registry.classify(dep.ecosystem, &dep.name);
            if !options.include_library_tier && !tier.is_default_emit() {
                report.skipped += 1;
                continue;
            }
            let url = resolve::canonical_url(dep.ecosystem, &dep.name, &dep.version);
            let md = cache
                .map(|c| describe::fetch_metadata(c, dep.ecosystem, &dep.name))
                .unwrap_or_default();
            let manifest_rel = dep
                .manifest_path
                .strip_prefix(project_root)
                .unwrap_or(&dep.manifest_path)
                .to_path_buf();
            let source = Source::Docs(DocsSource {
                name: dep.name,
                version: dep.version,
                ecosystem: dep.ecosystem,
                tier,
                url,
                description: md.description,
                manifest_path: Some(manifest_rel),
                forge: md.forge,
            });
            let uri = source.to_uri().to_string();
            let duplicate = bundle
                .items
                .iter()
                .any(|i| i.source.to_uri().to_string() == uri);
            if duplicate {
                continue;
            }
            bundle.add(Item {
                source,
                label: None,
            });
            report.added += 1;
        }
    }

    Ok(report)
}
