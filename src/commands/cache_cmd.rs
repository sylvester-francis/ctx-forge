//! `ctxforge cache list/clear/verify`.

use crate::cache::{ContentCache, policy};
use crate::error::{CtxforgeError, Result};
use crate::output;
use crate::paths;

pub fn list(scheme: Option<&str>) -> Result<()> {
    let Some(dir) = paths::global_cache_dir() else {
        return Err(CtxforgeError::Msg("cannot locate cache dir".into()));
    };
    if !dir.exists() {
        println!("(cache is empty)");
        return Ok(());
    }

    let cache = ContentCache::open(dir)?;
    let metas = cache.list()?;
    if metas.is_empty() {
        println!("(cache is empty)");
        return Ok(());
    }

    let mut shown = 0;
    for meta in &metas {
        if let Some(s) = scheme {
            if meta.scheme != s {
                continue;
            }
        }
        shown += 1;
        let age = chrono::Utc::now().signed_duration_since(meta.fetched_at);
        let age_str = match age.num_days() {
            d if d > 0 => format!("{d}d ago"),
            _ => match age.num_hours() {
                h if h > 0 => format!("{h}h ago"),
                _ => format!("{}m ago", age.num_minutes().max(0)),
            },
        };
        let fresh = if policy::is_fresh(
            meta.fetched_at,
            std::time::Duration::from_secs(meta.ttl_secs),
        ) {
            "fresh"
        } else {
            "stale"
        };
        println!(
            "  {} ({}, {} bytes, {age_str}, {fresh})",
            meta.uri, meta.scheme, meta.body_size,
        );
    }
    println!(
        "\n{shown} cached entr{}",
        if shown == 1 { "y" } else { "ies" }
    );
    Ok(())
}

pub fn clear(all: bool, stale: bool) -> Result<()> {
    let Some(dir) = paths::global_cache_dir() else {
        return Err(CtxforgeError::Msg("cannot locate cache dir".into()));
    };
    if !dir.exists() {
        println!("(cache already empty)");
        return Ok(());
    }

    let cache = ContentCache::open(dir)?;
    if stale && !all {
        let mut cleared = 0;
        for meta in cache.list()? {
            if !policy::is_fresh(
                meta.fetched_at,
                std::time::Duration::from_secs(meta.ttl_secs),
            ) {
                let key = crate::cache::cache_key_for_uri(&meta.uri);
                cache.invalidate(&key)?;
                cleared += 1;
            }
        }
        output::success(&format!("cleared {cleared} stale entries"));
    } else {
        let count = cache.clear()?;
        output::success(&format!("cleared {count} cached entries"));
    }
    Ok(())
}

pub fn verify() -> Result<()> {
    let Some(dir) = paths::global_cache_dir() else {
        return Err(CtxforgeError::Msg("cannot locate cache dir".into()));
    };
    if !dir.exists() {
        println!("(cache empty — nothing to verify)");
        return Ok(());
    }

    let cache = ContentCache::open(dir)?;
    let r = cache.verify()?;
    println!("checked: {}", r.checked);
    println!("ok:      {}", r.ok);
    if !r.body_mismatch.is_empty() {
        output::warn(&format!(
            "body SHA mismatch: {}",
            r.body_mismatch.join(", ")
        ));
    }
    if !r.hmac_mismatch.is_empty() {
        output::warn(&format!("HMAC mismatch: {}", r.hmac_mismatch.join(", ")));
    }
    if !r.read_error.is_empty() {
        output::warn(&format!("read errors: {}", r.read_error.join(", ")));
    }
    if r.body_mismatch.is_empty() && r.hmac_mismatch.is_empty() && r.read_error.is_empty() {
        output::success("cache verified OK");
    }
    Ok(())
}
