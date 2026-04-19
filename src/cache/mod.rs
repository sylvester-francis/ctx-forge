//! Content-addressable cache for network-fetched sources.

pub mod policy;
pub mod store;

use crate::error::{CtxforgeError, Result};
use chrono::Utc;
use std::path::{Path, PathBuf};

pub use store::{Meta, cache_key_for_uri, object_dir, sha256_hex};

pub enum CacheRead {
    Fresh { body: Vec<u8>, meta: Meta },
    Stale { body: Vec<u8>, meta: Meta },
    Miss,
}

#[derive(Debug, Default)]
pub struct VerifyReport {
    pub checked: usize,
    pub ok: usize,
    pub body_mismatch: Vec<String>,
    pub hmac_mismatch: Vec<String>,
    pub read_error: Vec<String>,
}

pub struct ContentCache {
    pub root: PathBuf,
    hmac_key: [u8; 32],
}

impl ContentCache {
    pub fn open(root: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(root.join("objects"))?;
        let hmac_key = load_or_create_hmac_key(&root)?;
        Ok(Self { root, hmac_key })
    }

    pub fn get(&self, key: &str) -> Result<CacheRead> {
        let dir = store::object_dir(&self.root, key);
        let meta_path = dir.join("meta.json");
        let hmac_path = dir.join("meta.hmac");
        let body_path = dir.join("body");
        if !meta_path.exists() || !hmac_path.exists() || !body_path.exists() {
            return Ok(CacheRead::Miss);
        }

        let meta_bytes = std::fs::read(&meta_path)?;
        let hmac_bytes = std::fs::read_to_string(&hmac_path)?;
        if !store::verify_hmac(&self.hmac_key, &meta_bytes, hmac_bytes.trim()) {
            return Ok(CacheRead::Miss);
        }

        let meta: Meta = serde_json::from_slice(&meta_bytes)?;
        let body = std::fs::read(&body_path)?;
        if store::sha256_hex(&body) != meta.body_sha256 {
            return Ok(CacheRead::Miss);
        }

        let ttl = std::time::Duration::from_secs(meta.ttl_secs);
        if policy::is_fresh(meta.fetched_at, ttl) {
            Ok(CacheRead::Fresh { body, meta })
        } else {
            Ok(CacheRead::Stale { body, meta })
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn put(
        &self,
        key: &str,
        uri: &str,
        scheme: &str,
        body: &[u8],
        ttl_secs: u64,
        etag: Option<String>,
        content_type: Option<String>,
    ) -> Result<Meta> {
        let dir = store::object_dir(&self.root, key);
        std::fs::create_dir_all(&dir)?;

        let meta = Meta {
            uri: uri.into(),
            scheme: scheme.into(),
            fetched_at: Utc::now(),
            ttl_secs,
            etag,
            content_type,
            body_size: body.len() as u64,
            body_sha256: store::sha256_hex(body),
        };

        let meta_json = serde_json::to_vec_pretty(&meta)?;
        let hmac_hex = store::compute_hmac(&self.hmac_key, &meta_json);

        atomic_write(&dir, "body", body)?;
        atomic_write(&dir, "meta.json", &meta_json)?;
        atomic_write(&dir, "meta.hmac", hmac_hex.as_bytes())?;

        Ok(meta)
    }

    /// Refresh `fetched_at` after an HTTP 304 without rewriting body.
    pub fn touch(&self, key: &str) -> Result<()> {
        let dir = store::object_dir(&self.root, key);
        let meta_path = dir.join("meta.json");
        if !meta_path.exists() {
            return Ok(());
        }
        let raw = std::fs::read(&meta_path)?;
        let mut meta: Meta = serde_json::from_slice(&raw)?;
        meta.fetched_at = Utc::now();
        let meta_json = serde_json::to_vec_pretty(&meta)?;
        let hmac_hex = store::compute_hmac(&self.hmac_key, &meta_json);
        atomic_write(&dir, "meta.json", &meta_json)?;
        atomic_write(&dir, "meta.hmac", hmac_hex.as_bytes())?;
        Ok(())
    }

    pub fn invalidate(&self, key: &str) -> Result<()> {
        let dir = store::object_dir(&self.root, key);
        if dir.exists() {
            std::fs::remove_dir_all(&dir)?;
        }
        Ok(())
    }

    pub fn list(&self) -> Result<Vec<Meta>> {
        let objects = self.root.join("objects");
        if !objects.exists() {
            return Ok(vec![]);
        }
        let mut metas = Vec::new();
        for shard in std::fs::read_dir(&objects)?.flatten() {
            if !shard.path().is_dir() {
                continue;
            }
            for entry in std::fs::read_dir(shard.path())?.flatten() {
                let meta_path = entry.path().join("meta.json");
                if let Ok(raw) = std::fs::read(&meta_path) {
                    if let Ok(meta) = serde_json::from_slice::<Meta>(&raw) {
                        metas.push(meta);
                    }
                }
            }
        }
        Ok(metas)
    }

    pub fn clear(&self) -> Result<usize> {
        let metas = self.list()?;
        let count = metas.len();
        let objects = self.root.join("objects");
        if objects.exists() {
            std::fs::remove_dir_all(&objects)?;
            std::fs::create_dir_all(&objects)?;
        }
        Ok(count)
    }

    pub fn verify(&self) -> Result<VerifyReport> {
        let mut report = VerifyReport::default();
        let objects = self.root.join("objects");
        if !objects.exists() {
            return Ok(report);
        }
        for shard in std::fs::read_dir(&objects)?.flatten() {
            if !shard.path().is_dir() {
                continue;
            }
            for entry in std::fs::read_dir(shard.path())?.flatten() {
                report.checked += 1;
                let dir = entry.path();
                let key = dir
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();

                let meta_path = dir.join("meta.json");
                let hmac_path = dir.join("meta.hmac");
                let body_path = dir.join("body");

                let Ok(meta_bytes) = std::fs::read(&meta_path) else {
                    report.read_error.push(format!("{key}: meta"));
                    continue;
                };
                let Ok(hmac_hex) = std::fs::read_to_string(&hmac_path) else {
                    report.read_error.push(format!("{key}: hmac"));
                    continue;
                };
                if !store::verify_hmac(&self.hmac_key, &meta_bytes, hmac_hex.trim()) {
                    report.hmac_mismatch.push(key.clone());
                    continue;
                }
                let meta: Meta = match serde_json::from_slice(&meta_bytes) {
                    Ok(m) => m,
                    Err(_) => {
                        report.read_error.push(format!("{key}: meta parse"));
                        continue;
                    }
                };
                let body = match std::fs::read(&body_path) {
                    Ok(b) => b,
                    Err(_) => {
                        report.read_error.push(format!("{key}: body"));
                        continue;
                    }
                };
                if store::sha256_hex(&body) != meta.body_sha256 {
                    report.body_mismatch.push(key.clone());
                } else {
                    report.ok += 1;
                }
            }
        }
        Ok(report)
    }
}

fn atomic_write(dir: &Path, name: &str, bytes: &[u8]) -> Result<()> {
    let tmp = tempfile::NamedTempFile::new_in(dir).map_err(CtxforgeError::from)?;
    std::fs::write(tmp.path(), bytes)?;
    tmp.persist(dir.join(name))
        .map_err(|e| CtxforgeError::Cache(format!("persist {name}: {e}")))?;
    Ok(())
}

fn load_or_create_hmac_key(root: &Path) -> Result<[u8; 32]> {
    let key_path = root.join("cache.key");
    if key_path.exists() {
        let raw = std::fs::read(&key_path)?;
        if raw.len() >= 32 {
            let mut key = [0u8; 32];
            key.copy_from_slice(&raw[..32]);
            return Ok(key);
        }
    }
    let mut key = [0u8; 32];
    getrandom::fill(&mut key).map_err(|e| CtxforgeError::Cache(format!("OS RNG: {e}")))?;
    std::fs::write(&key_path, key)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn cache_with_root() -> (TempDir, ContentCache) {
        let td = TempDir::new().unwrap();
        let c = ContentCache::open(td.path().to_path_buf()).unwrap();
        (td, c)
    }

    #[test]
    fn put_then_get_fresh() {
        let (_td, cache) = cache_with_root();
        let key = cache_key_for_uri("url://example.com/a");
        cache
            .put(
                &key,
                "url://example.com/a",
                "url",
                b"hello",
                86_400,
                None,
                None,
            )
            .unwrap();
        match cache.get(&key).unwrap() {
            CacheRead::Fresh { body, meta } => {
                assert_eq!(body, b"hello");
                assert_eq!(meta.uri, "url://example.com/a");
            }
            _ => panic!("expected Fresh"),
        }
    }

    #[test]
    fn miss_on_unknown_key() {
        let (_td, cache) = cache_with_root();
        assert!(matches!(cache.get("nope").unwrap(), CacheRead::Miss));
    }

    #[test]
    fn tampered_body_returns_miss() {
        let (td, cache) = cache_with_root();
        let key = cache_key_for_uri("url://example.com/tamper");
        cache
            .put(
                &key,
                "url://example.com/tamper",
                "url",
                b"orig",
                86_400,
                None,
                None,
            )
            .unwrap();
        let dir = object_dir(td.path(), &key);
        std::fs::write(dir.join("body"), b"BAD").unwrap();
        assert!(matches!(cache.get(&key).unwrap(), CacheRead::Miss));
    }

    #[test]
    fn tampered_meta_returns_miss() {
        let (td, cache) = cache_with_root();
        let key = cache_key_for_uri("url://example.com/t2");
        cache
            .put(
                &key,
                "url://example.com/t2",
                "url",
                b"body",
                86_400,
                None,
                None,
            )
            .unwrap();
        let dir = object_dir(td.path(), &key);
        std::fs::write(dir.join("meta.json"), br#"{"uri":"evil"}"#).unwrap();
        assert!(matches!(cache.get(&key).unwrap(), CacheRead::Miss));
    }

    #[test]
    fn stale_after_ttl_expires() {
        let (_td, cache) = cache_with_root();
        let key = cache_key_for_uri("url://example.com/stale");
        cache
            .put(&key, "url://example.com/stale", "url", b"x", 0, None, None)
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        match cache.get(&key).unwrap() {
            CacheRead::Stale { .. } => {}
            CacheRead::Fresh { .. } => panic!("expected Stale, got Fresh"),
            CacheRead::Miss => panic!("expected Stale, got Miss"),
        }
    }

    #[test]
    fn clear_removes_all_and_returns_count() {
        let (_td, cache) = cache_with_root();
        let k = cache_key_for_uri("url://e.com/c");
        cache
            .put(&k, "url://e.com/c", "url", b"x", 86_400, None, None)
            .unwrap();
        assert_eq!(cache.clear().unwrap(), 1);
        assert!(matches!(cache.get(&k).unwrap(), CacheRead::Miss));
    }

    #[test]
    fn verify_reports_ok_when_intact() {
        let (_td, cache) = cache_with_root();
        let k = cache_key_for_uri("url://e.com/v");
        cache
            .put(&k, "url://e.com/v", "url", b"ok", 86_400, None, None)
            .unwrap();
        let r = cache.verify().unwrap();
        assert_eq!(r.ok, 1);
        assert!(r.body_mismatch.is_empty());
        assert!(r.hmac_mismatch.is_empty());
    }

    #[test]
    fn verify_flags_body_tamper() {
        let (td, cache) = cache_with_root();
        let k = cache_key_for_uri("url://e.com/bad");
        cache
            .put(&k, "url://e.com/bad", "url", b"ok", 86_400, None, None)
            .unwrap();
        let dir = object_dir(td.path(), &k);
        std::fs::write(dir.join("body"), b"EVIL").unwrap();
        let r = cache.verify().unwrap();
        assert_eq!(r.body_mismatch.len(), 1);
    }
}
