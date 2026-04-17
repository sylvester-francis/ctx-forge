//! Security regression suite — each control from spec §6 has at least
//! one negative test that MUST return an error.

use ctxforge::source::url::{is_accepted_content_type, is_private_ip, validate_url};
use ctxforge::source::{Uri, UriParseError};
use std::net::IpAddr;

#[test]
fn uri_rejects_unknown_schemes() {
    for bad in &["ftp://example.com", "javascript://x", "data://x"] {
        assert!(matches!(
            bad.parse::<Uri>().unwrap_err(),
            UriParseError::UnknownScheme(_)
        ));
    }
}

#[test]
fn uri_rejects_dot_segments() {
    assert!(matches!(
        "file:///abs/../etc/passwd".parse::<Uri>().unwrap_err(),
        UriParseError::DotSegment
    ));
}

#[test]
fn uri_rejects_null_and_control_chars() {
    assert!(matches!(
        "file:///a/\0b".parse::<Uri>().unwrap_err(),
        UriParseError::NullByte
    ));
    assert!(matches!(
        "file:///a/\x01b".parse::<Uri>().unwrap_err(),
        UriParseError::InvalidChar(_)
    ));
}

#[test]
fn uri_rejects_userinfo() {
    assert!(matches!(
        "url://u:p@example.com/".parse::<Uri>().unwrap_err(),
        UriParseError::UserInfo
    ));
}

#[test]
fn validate_url_rejects_http_by_default() {
    assert!(validate_url("http://example.com/", false, false).is_err());
}

#[test]
fn validate_url_rejects_literal_private_v4() {
    for h in &[
        "127.0.0.1",
        "10.0.0.1",
        "172.16.0.1",
        "192.168.1.1",
        "169.254.1.1",
        "100.64.0.1",
    ] {
        let url = format!("https://{h}/");
        assert!(
            validate_url(&url, false, false).is_err(),
            "should reject {h}"
        );
    }
}

#[test]
fn validate_url_rejects_literal_private_v6() {
    for h in &["[::1]", "[fe80::1]", "[fc00::1]"] {
        let url = format!("https://{h}/");
        assert!(
            validate_url(&url, false, false).is_err(),
            "should reject {h}"
        );
    }
}

#[test]
fn validate_url_rejects_internal_and_local_tlds() {
    assert!(validate_url("https://db.internal/", false, false).is_err());
    assert!(validate_url("https://x.local/", false, false).is_err());
    assert!(validate_url("https://localhost/", false, false).is_err());
}

#[test]
fn validate_url_rejects_userinfo() {
    assert!(validate_url("https://u:p@example.com/", false, false).is_err());
}

#[test]
fn private_ip_classifies_known_ranges() {
    assert!(is_private_ip(IpAddr::from([127, 0, 0, 1])));
    assert!(is_private_ip(IpAddr::from([10, 0, 0, 1])));
    assert!(is_private_ip(IpAddr::from([169, 254, 1, 1])));
    assert!(!is_private_ip(IpAddr::from([8, 8, 8, 8])));
}

#[test]
fn content_type_gate_rejects_binary() {
    assert!(!is_accepted_content_type("image/png"));
    assert!(!is_accepted_content_type("application/octet-stream"));
    assert!(!is_accepted_content_type(""));
}

#[test]
fn cache_detects_body_tampering() {
    use ctxforge::cache::{CacheRead, ContentCache, cache_key_for_uri, object_dir};
    use tempfile::TempDir;

    let td = TempDir::new().unwrap();
    let cache = ContentCache::open(td.path().to_path_buf()).unwrap();
    let k = cache_key_for_uri("url://example.com/t");
    cache
        .put(
            &k,
            "url://example.com/t",
            "url",
            b"orig",
            86_400,
            None,
            None,
        )
        .unwrap();
    std::fs::write(object_dir(td.path(), &k).join("body"), b"EVIL").unwrap();
    assert!(matches!(cache.get(&k).unwrap(), CacheRead::Miss));
}

#[test]
fn cache_detects_meta_tampering() {
    use ctxforge::cache::{CacheRead, ContentCache, cache_key_for_uri, object_dir};
    use tempfile::TempDir;

    let td = TempDir::new().unwrap();
    let cache = ContentCache::open(td.path().to_path_buf()).unwrap();
    let k = cache_key_for_uri("url://example.com/m");
    cache
        .put(&k, "url://example.com/m", "url", b"x", 86_400, None, None)
        .unwrap();
    let dir = object_dir(td.path(), &k);
    let mut meta = std::fs::read(dir.join("meta.json")).unwrap();
    meta[0] = b' ';
    std::fs::write(dir.join("meta.json"), meta).unwrap();
    assert!(matches!(cache.get(&k).unwrap(), CacheRead::Miss));
}

#[test]
fn different_urls_with_different_queries_have_different_cache_keys() {
    use ctxforge::source::Source;
    let a = Source::from_uri(&"url://e.com/s?q=a".parse::<Uri>().unwrap()).unwrap();
    let b = Source::from_uri(&"url://e.com/s?q=b".parse::<Uri>().unwrap()).unwrap();
    assert_ne!(a.cache_key(), b.cache_key());
}
