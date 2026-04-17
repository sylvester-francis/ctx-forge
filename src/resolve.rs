//! Resolve bundle items into string content at export time.
//!
//! The `Bundle` holds references (Source variants); this module reads the
//! filesystem (or cache, for network sources in Task 9) to produce content
//! strings plus a `Provenance` record.

#![allow(dead_code)]

use crate::bundle::Item;
use crate::error::{CtxforgeError, Result};
use crate::source::{Provenance, Source};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct ResolvedItem {
    pub item: Item,
    pub content: String,
    pub language: &'static str,
    pub provenance: Provenance,
}

pub fn resolve_all(items: &[Item], project_root: &Path) -> Result<Vec<ResolvedItem>> {
    items
        .iter()
        .map(|it| resolve_one(it, project_root))
        .collect()
}

pub fn resolve_one(item: &Item, project_root: &Path) -> Result<ResolvedItem> {
    match &item.source {
        Source::File(f) => resolve_file(item, project_root, &f.path, None),
        Source::Range(r) => resolve_file(item, project_root, &r.path, Some((r.start, r.end))),
        #[cfg(feature = "extract")]
        Source::Func(f) => resolve_symbol(item, project_root, &f.path, &f.name, true),
        #[cfg(feature = "extract")]
        Source::Type(t) => resolve_symbol(item, project_root, &t.path, &t.name, false),
        #[cfg(not(feature = "extract"))]
        Source::Func(_) | Source::Type(_) => Err(CtxforgeError::Msg(
            "function/type extraction requires `cargo install ctxforge --features=extract`".into(),
        )),
        Source::Url(_) => Err(CtxforgeError::Msg(
            "network sources require resolve::resolve_all_with_ctx (Task 8)".into(),
        )),
    }
}

fn resolve_file(
    item: &Item,
    project_root: &Path,
    rel: &Path,
    range: Option<(usize, usize)>,
) -> Result<ResolvedItem> {
    let full = project_root.join(rel);
    let language = crate::lang::detect(rel);
    let bytes = std::fs::read(&full)?;
    let all = match String::from_utf8(bytes) {
        Ok(s) => s,
        Err(e) => format!("<non-UTF-8 file, {} bytes — omitted>\n", e.as_bytes().len()),
    };
    let content = match range {
        None => all,
        Some((start, end)) => extract_range(&all, start, end),
    };
    let sha = crate::cache::sha256_hex(content.as_bytes());
    Ok(ResolvedItem {
        item: item.clone(),
        content,
        language,
        provenance: Provenance::local(item.source.to_uri().to_string(), sha),
    })
}

fn extract_range(raw: &str, start: usize, end: usize) -> String {
    let lines: Vec<&str> = raw.lines().collect();
    if lines.is_empty() {
        return String::new();
    }
    let start = start.min(lines.len()).max(1);
    let end = end.min(lines.len());
    lines[start - 1..end].join("\n")
}

#[cfg(feature = "extract")]
fn resolve_symbol(
    item: &Item,
    project_root: &Path,
    rel: &Path,
    name: &str,
    is_func: bool,
) -> Result<ResolvedItem> {
    let full = project_root.join(rel);
    let language = crate::lang::detect(rel);
    let source_text = std::fs::read_to_string(&full)?;
    let extracted = if is_func {
        crate::extract::extract_function(&source_text, language, name)
            .map_err(CtxforgeError::Msg)?
    } else {
        crate::extract::extract_type(&source_text, language, name).map_err(CtxforgeError::Msg)?
    };
    let content = extracted.ok_or_else(|| {
        CtxforgeError::Msg(format!(
            "{} `{}` not found in {}",
            if is_func { "function" } else { "type" },
            name,
            rel.display()
        ))
    })?;
    let sha = crate::cache::sha256_hex(content.as_bytes());
    Ok(ResolvedItem {
        item: item.clone(),
        content,
        language,
        provenance: Provenance::local(item.source.to_uri().to_string(), sha),
    })
}

use crate::cache::{CacheRead, ContentCache};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolveMode {
    Cli,
    Mcp,
}

pub struct ResolveCtx<'a> {
    pub project_root: &'a Path,
    pub cache: Option<&'a ContentCache>,
    pub mode: ResolveMode,
    pub strict: bool,
    pub offline: bool,
    pub warnings: std::cell::RefCell<Vec<String>>,
    #[cfg(feature = "fetch")]
    pub fetch_config: crate::fetch::FetchConfig,
}

impl<'a> ResolveCtx<'a> {
    pub fn cli(project_root: &'a Path, cache: Option<&'a ContentCache>) -> Self {
        Self {
            project_root,
            cache,
            mode: ResolveMode::Cli,
            strict: false,
            offline: false,
            warnings: Default::default(),
            #[cfg(feature = "fetch")]
            fetch_config: Default::default(),
        }
    }

    pub fn mcp(project_root: &'a Path, cache: Option<&'a ContentCache>) -> Self {
        let mut s = Self::cli(project_root, cache);
        s.mode = ResolveMode::Mcp;
        s
    }

    pub fn warn(&self, msg: impl Into<String>) {
        self.warnings.borrow_mut().push(msg.into());
    }

    pub fn drain_warnings(&self) -> Vec<String> {
        std::mem::take(&mut self.warnings.borrow_mut())
    }
}

pub fn resolve_all_with_ctx(items: &[Item], ctx: &ResolveCtx) -> Result<Vec<ResolvedItem>> {
    items
        .iter()
        .map(|it| resolve_one_with_ctx(it, ctx))
        .collect()
}

pub fn resolve_one_with_ctx(item: &Item, ctx: &ResolveCtx) -> Result<ResolvedItem> {
    if !item.source.is_cacheable() {
        return resolve_one(item, ctx.project_root);
    }
    resolve_network(item, ctx)
}

fn resolve_network(item: &Item, ctx: &ResolveCtx) -> Result<ResolvedItem> {
    let Source::Url(u) = &item.source else {
        return Err(CtxforgeError::Msg(
            "resolve_network called on non-Url source".into(),
        ));
    };
    let uri_str = item.source.to_uri().to_string();
    let key = item.source.cache_key();
    let ttl = item.source.default_ttl().as_secs();

    let cache = ctx
        .cache
        .ok_or_else(|| CtxforgeError::Cache("cache not initialized".into()))?;
    let lookup = cache.get(&key)?;

    match lookup {
        CacheRead::Fresh { body, meta } => Ok(render_network(item, body, &meta, false)),
        CacheRead::Stale { body, meta } if ctx.offline => {
            ctx.warn(format!(
                "offline; serving stale {uri_str} from {}",
                meta.fetched_at
            ));
            Ok(render_network(item, body, &meta, true))
        }
        CacheRead::Stale { body, meta } => match do_fetch(u.url.as_str(), ctx) {
            Ok(fr) => {
                let meta2 = cache.put(
                    &key,
                    &uri_str,
                    item.source.scheme_name(),
                    &fr.body,
                    ttl,
                    fr.etag,
                    fr.content_type,
                )?;
                if fr.charset_replaced {
                    ctx.warn(format!("{uri_str}: non-UTF-8 bytes replaced with U+FFFD"));
                }
                Ok(render_network(item, fr.body, &meta2, false))
            }
            Err(e) if !ctx.strict => {
                ctx.warn(format!(
                    "refresh failed for {uri_str}: {e} — serving cached from {}",
                    meta.fetched_at,
                ));
                Ok(render_network(item, body, &meta, true))
            }
            Err(e) => Err(CtxforgeError::Fetch(format!("{uri_str}: {e}"))),
        },
        CacheRead::Miss if ctx.offline => {
            let msg = format!("offline and no cache for {uri_str}");
            if ctx.strict || ctx.mode == ResolveMode::Mcp {
                Err(CtxforgeError::Fetch(msg))
            } else {
                Ok(placeholder(item, msg))
            }
        }
        CacheRead::Miss => match do_fetch(u.url.as_str(), ctx) {
            Ok(fr) => {
                let meta = cache.put(
                    &key,
                    &uri_str,
                    item.source.scheme_name(),
                    &fr.body,
                    ttl,
                    fr.etag,
                    fr.content_type,
                )?;
                if fr.charset_replaced {
                    ctx.warn(format!("{uri_str}: non-UTF-8 bytes replaced with U+FFFD"));
                }
                Ok(render_network(item, fr.body, &meta, false))
            }
            Err(e) if !ctx.strict && ctx.mode == ResolveMode::Cli => {
                ctx.warn(format!("{uri_str}: {e}"));
                Ok(placeholder(item, e))
            }
            Err(e) => Err(CtxforgeError::Fetch(format!("{uri_str}: {e}"))),
        },
    }
}

#[cfg(feature = "fetch")]
fn do_fetch(url: &str, ctx: &ResolveCtx) -> std::result::Result<crate::fetch::FetchResult, String> {
    crate::fetch::fetch(url, &ctx.fetch_config)
}
#[cfg(not(feature = "fetch"))]
fn do_fetch(_url: &str, _ctx: &ResolveCtx) -> std::result::Result<FetchShim, String> {
    Err("ctxforge built without `fetch` feature".into())
}

#[cfg(not(feature = "fetch"))]
struct FetchShim {
    body: Vec<u8>,
    etag: Option<String>,
    content_type: Option<String>,
    charset_replaced: bool,
}

fn render_network(
    item: &Item,
    body: Vec<u8>,
    meta: &crate::cache::Meta,
    stale: bool,
) -> ResolvedItem {
    let content = String::from_utf8_lossy(&body).into_owned();
    let provenance = Provenance::network(
        meta.uri.clone(),
        meta.body_sha256.clone(),
        meta.fetched_at,
        meta.etag.clone(),
        stale,
    );
    ResolvedItem {
        item: item.clone(),
        content,
        language: "text",
        provenance,
    }
}

fn placeholder(item: &Item, reason: impl Into<String>) -> ResolvedItem {
    let reason = reason.into();
    let uri = item.source.to_uri().to_string();
    ResolvedItem {
        item: item.clone(),
        content: format!("<!-- ctxforge: FAILED {uri}\n     reason: {reason} -->\n"),
        language: "text",
        provenance: Provenance::failed(uri, reason),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::{FileSource, RangeSource};
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn write(root: &Path, rel: &str, c: &str) {
        let f = root.join(rel);
        std::fs::create_dir_all(f.parent().unwrap()).unwrap();
        std::fs::write(f, c).unwrap();
    }

    #[test]
    fn file_has_local_provenance_with_sha() {
        let td = TempDir::new().unwrap();
        write(td.path(), "a.rs", "fn main() {}\n");
        let item = Item {
            source: Source::File(FileSource {
                path: "a.rs".into(),
            }),
            label: None,
        };
        let r = resolve_one(&item, td.path()).unwrap();
        assert_eq!(r.content, "fn main() {}\n");
        assert!(!r.provenance.sha256.is_empty());
        assert!(r.provenance.fetched_at.is_none());
        assert_eq!(r.provenance.uri, "file://a.rs");
    }

    #[test]
    fn range_extracts_slice() {
        let td = TempDir::new().unwrap();
        write(td.path(), "a.txt", "one\ntwo\nthree\nfour\n");
        let src = Source::Range(RangeSource::new("a.txt".into(), 2, 3).unwrap());
        let item = Item {
            source: src,
            label: None,
        };
        let r = resolve_one(&item, td.path()).unwrap();
        assert_eq!(r.content, "two\nthree");
    }

    #[test]
    fn range_clamps_past_end() {
        let td = TempDir::new().unwrap();
        write(td.path(), "a.txt", "one\ntwo\n");
        let src = Source::Range(RangeSource::new("a.txt".into(), 1, 100).unwrap());
        let item = Item {
            source: src,
            label: None,
        };
        let r = resolve_one(&item, td.path()).unwrap();
        assert_eq!(r.content, "one\ntwo");
    }

    #[test]
    fn non_utf8_produces_placeholder() {
        let td = TempDir::new().unwrap();
        std::fs::write(td.path().join("b.bin"), [0xFF, 0xFE, 0xFD]).unwrap();
        let item = Item {
            source: Source::File(FileSource {
                path: "b.bin".into(),
            }),
            label: None,
        };
        let r = resolve_one(&item, td.path()).unwrap();
        assert!(r.content.starts_with("<non-UTF-8"));
    }

    #[test]
    fn url_source_errors_in_base_resolve() {
        use crate::source::UrlSource;
        let item = Item {
            source: Source::Url(UrlSource {
                url: "https://x".into(),
            }),
            label: None,
        };
        assert!(resolve_one(&item, Path::new("/")).is_err());
    }

    fn url_item(url: &str) -> Item {
        use crate::source::UrlSource;
        Item {
            source: Source::Url(UrlSource { url: url.into() }),
            label: None,
        }
    }

    #[test]
    fn offline_miss_cli_returns_placeholder() {
        let td = TempDir::new().unwrap();
        let cache = ContentCache::open(td.path().to_path_buf()).unwrap();
        let item = url_item("https://example.invalid/x");
        let mut ctx = ResolveCtx::cli(td.path(), Some(&cache));
        ctx.offline = true;
        let r = resolve_one_with_ctx(&item, &ctx).unwrap();
        assert!(r.provenance.failed);
        assert!(r.content.contains("FAILED"));
    }

    #[test]
    fn offline_miss_mcp_errors() {
        let td = TempDir::new().unwrap();
        let cache = ContentCache::open(td.path().to_path_buf()).unwrap();
        let item = url_item("https://example.invalid/x");
        let mut ctx = ResolveCtx::mcp(td.path(), Some(&cache));
        ctx.offline = true;
        assert!(resolve_one_with_ctx(&item, &ctx).is_err());
    }

    #[test]
    fn strict_cli_offline_miss_errors() {
        let td = TempDir::new().unwrap();
        let cache = ContentCache::open(td.path().to_path_buf()).unwrap();
        let item = url_item("https://example.invalid/x");
        let mut ctx = ResolveCtx::cli(td.path(), Some(&cache));
        ctx.offline = true;
        ctx.strict = true;
        assert!(resolve_one_with_ctx(&item, &ctx).is_err());
    }

    #[test]
    fn resolve_all_succeeds_with_mixed_text_and_binary() {
        let td = TempDir::new().unwrap();
        write(td.path(), "a.rs", "fn a() {}\n");
        write(td.path(), "b.rs", "fn b() {}\n");
        std::fs::write(td.path().join("bad.bin"), [0xFF, 0xFE, 0xFD]).unwrap();

        let items = vec![
            Item {
                source: Source::File(FileSource {
                    path: PathBuf::from("a.rs"),
                }),
                label: None,
            },
            Item {
                source: Source::File(FileSource {
                    path: PathBuf::from("bad.bin"),
                }),
                label: None,
            },
            Item {
                source: Source::File(FileSource {
                    path: PathBuf::from("b.rs"),
                }),
                label: None,
            },
        ];
        let resolved = resolve_all(&items, td.path()).expect("should not error");
        assert_eq!(resolved.len(), 3);
        assert_eq!(resolved[0].content, "fn a() {}\n");
        assert!(resolved[1].content.starts_with("<non-UTF-8 file"));
        assert_eq!(resolved[2].content, "fn b() {}\n");
    }
}
