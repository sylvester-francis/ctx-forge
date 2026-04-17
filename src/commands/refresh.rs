//! `ctxforge refresh` — force-refresh cached sources.

use crate::bundle::Bundle;
use crate::cache::{CacheRead, ContentCache};
use crate::error::{CtxforgeError, Result};
use crate::output;
use crate::paths::{self, CtxforgeRoot};

pub fn run(root: &CtxforgeRoot, uri: Option<&str>, all: bool) -> Result<()> {
    let bundle = Bundle::load_or_default(root)?;
    let Some(dir) = paths::global_cache_dir() else {
        return Err(CtxforgeError::Msg("cannot locate cache dir".into()));
    };
    let cache = ContentCache::open(dir)?;

    #[cfg(feature = "fetch")]
    let cfg = crate::fetch::FetchConfig::default();

    let mut refreshed = 0;
    let mut failed: Vec<(String, String)> = vec![];

    for item in &bundle.items {
        if !item.source.is_cacheable() {
            continue;
        }
        let item_uri = item.source.to_uri().to_string();
        if let Some(f) = uri {
            if item_uri != f {
                continue;
            }
        }

        let key = item.source.cache_key();
        if !all && matches!(cache.get(&key)?, CacheRead::Fresh { .. }) {
            continue;
        }

        let url = match &item.source {
            crate::source::Source::Url(u) => u.url.clone(),
            _ => continue,
        };

        #[cfg(feature = "fetch")]
        let outcome = crate::fetch::fetch(&url, &cfg);
        #[cfg(not(feature = "fetch"))]
        let outcome: std::result::Result<FetchShim, String> =
            Err("ctxforge built without `fetch` feature".into());

        match outcome {
            Ok(fr) => {
                cache.put(
                    &key,
                    &item_uri,
                    item.source.scheme_name(),
                    &fr.body,
                    item.source.default_ttl().as_secs(),
                    fr.etag,
                    fr.content_type,
                )?;
                refreshed += 1;
            }
            Err(e) => failed.push((item_uri, e)),
        }
    }

    output::success(&format!("refreshed {refreshed} source(s)"));
    for (uri, err) in &failed {
        output::warn(&format!("failed {uri}: {err}"));
    }
    if !failed.is_empty() {
        return Err(CtxforgeError::Msg(format!(
            "{} source(s) failed to refresh",
            failed.len()
        )));
    }
    Ok(())
}

#[cfg(not(feature = "fetch"))]
#[allow(dead_code)]
struct FetchShim {
    body: Vec<u8>,
    etag: Option<String>,
    content_type: Option<String>,
}
