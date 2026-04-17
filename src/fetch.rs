//! HTTP fetcher. Sync, blocking, ureq-backed. Encapsulates the redirect
//! policy, content-type/size guards, charset handling, and SSRF
//! resolver-check applied to every hop.

#![cfg(feature = "fetch")]

use crate::source::url::{is_accepted_content_type, resolve_host_and_check_ssrf, validate_url};
use std::io::Read;
use std::time::Duration;

pub struct FetchConfig {
    pub max_bytes: usize,
    pub timeout: Duration,
    pub max_redirects: u32,
    pub allow_http: bool,
    pub allow_private_net: bool,
    pub strict_charset: bool,
}

impl Default for FetchConfig {
    fn default() -> Self {
        Self {
            max_bytes: 5 * 1024 * 1024,
            timeout: Duration::from_secs(30),
            max_redirects: 3,
            allow_http: false,
            allow_private_net: false,
            strict_charset: false,
        }
    }
}

pub struct FetchResult {
    pub body: Vec<u8>,
    pub etag: Option<String>,
    pub content_type: Option<String>,
    pub charset_replaced: bool,
}

/// Fetch a URL with all guards applied.
pub fn fetch(url: &str, cfg: &FetchConfig) -> Result<FetchResult, String> {
    validate_url(url, cfg.allow_http, cfg.allow_private_net)?;

    let mut current = url.to_string();
    for hop in 0..=cfg.max_redirects {
        let (host, port) = extract_host_port(&current)?;
        resolve_host_and_check_ssrf(&host, port, cfg.allow_private_net)?;

        let agent = build_agent(cfg);
        let req = agent.get(&current).header(
            "User-Agent",
            format!("ctxforge/{}", env!("CARGO_PKG_VERSION")),
        );
        let resp = req.call().map_err(|e| format!("HTTP {current}: {e}"))?;

        let status = resp.status().as_u16();
        if (300..400).contains(&status) {
            if hop == cfg.max_redirects {
                return Err(format!("exceeded {} redirects", cfg.max_redirects));
            }
            let Some(loc) = resp.headers().get("location") else {
                return Err(format!("HTTP {status} with no Location header"));
            };
            let loc_str = loc
                .to_str()
                .map_err(|e| format!("location: {e}"))?
                .to_string();
            let next = resolve_redirect_target(&current, &loc_str)?;
            validate_url(&next, cfg.allow_http, cfg.allow_private_net)?;
            current = next;
            continue;
        }
        if !resp.status().is_success() {
            return Err(format!("HTTP {status}"));
        }

        let ct_header = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
            .ok_or("missing or unparseable Content-Type header")?;
        if !is_accepted_content_type(&ct_header) {
            return Err(format!("rejected content-type: {ct_header}"));
        }

        let etag = resp
            .headers()
            .get("etag")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let mut reader = resp.into_body().into_reader();
        let mut body = Vec::new();
        let limit = (cfg.max_bytes as u64).saturating_add(1);
        reader
            .by_ref()
            .take(limit)
            .read_to_end(&mut body)
            .map_err(|e| format!("body read: {e}"))?;
        if body.len() > cfg.max_bytes {
            return Err(format!(
                "response exceeds {} bytes (max-bytes cap)",
                cfg.max_bytes
            ));
        }

        let (body, replaced) = enforce_utf8(body, cfg.strict_charset)?;
        return Ok(FetchResult {
            body,
            etag,
            content_type: Some(ct_header),
            charset_replaced: replaced,
        });
    }
    Err("redirect loop detected".into())
}

fn build_agent(cfg: &FetchConfig) -> ureq::Agent {
    let config = ureq::Agent::config_builder()
        .timeout_global(Some(cfg.timeout))
        .max_redirects(0) // we handle redirects ourselves
        .build();
    config.into()
}

fn extract_host_port(url: &str) -> Result<(String, u16), String> {
    let after = url.split("://").nth(1).ok_or("malformed URL")?;
    let authority = after.split('/').next().unwrap_or(after);
    let default_port = if url.starts_with("https://") { 443 } else { 80 };

    // Handle bracketed IPv6 literals: [::1]:8080
    if let Some(stripped) = authority.strip_prefix('[') {
        let end = stripped
            .find(']')
            .ok_or("malformed IPv6 authority (missing ])")?;
        let host = &stripped[..end];
        let rest = &stripped[end + 1..];
        let port = if let Some(p) = rest.strip_prefix(':') {
            p.parse().map_err(|_| format!("invalid port in {url}"))?
        } else {
            default_port
        };
        return Ok((host.to_string(), port));
    }

    let (host, port) = match authority.rsplit_once(':') {
        Some((h, p)) => {
            let port: u16 = p.parse().map_err(|_| format!("invalid port in {url}"))?;
            (h.to_string(), port)
        }
        None => (authority.to_string(), default_port),
    };
    Ok((host, port))
}

fn resolve_redirect_target(base: &str, location: &str) -> Result<String, String> {
    if location.starts_with("https://") || location.starts_with("http://") {
        return Ok(location.to_string());
    }
    let origin_end = base.find("://").map(|i| i + 3).ok_or("bad base URL")?;
    let after_origin = &base[origin_end..];
    let origin_len = after_origin
        .find('/')
        .map(|i| origin_end + i)
        .unwrap_or(base.len());
    let origin = &base[..origin_len];
    if location.starts_with('/') {
        Ok(format!("{origin}{location}"))
    } else {
        let trim_end = base.rfind('/').unwrap_or(base.len());
        Ok(format!("{}/{location}", &base[..trim_end]))
    }
}

fn enforce_utf8(body: Vec<u8>, strict: bool) -> Result<(Vec<u8>, bool), String> {
    match String::from_utf8(body) {
        Ok(s) => Ok((s.into_bytes(), false)),
        Err(e) => {
            if strict {
                return Err(format!(
                    "response is not valid UTF-8 ({} bytes); re-run without --strict to replace",
                    e.as_bytes().len(),
                ));
            }
            let bytes = e.into_bytes();
            let replaced = String::from_utf8_lossy(&bytes).into_owned();
            Ok((replaced.into_bytes(), true))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_host_port_defaults() {
        let (h, p) = extract_host_port("https://example.com/a").unwrap();
        assert_eq!((h.as_str(), p), ("example.com", 443));
        let (h, p) = extract_host_port("http://example.com:8080/a").unwrap();
        assert_eq!((h.as_str(), p), ("example.com", 8080));
    }

    #[test]
    fn extract_host_port_ipv6_literal() {
        let (h, p) = extract_host_port("https://[::1]:8443/x").unwrap();
        assert_eq!((h.as_str(), p), ("::1", 8443));
        let (h, p) = extract_host_port("https://[fe80::1]/x").unwrap();
        assert_eq!((h.as_str(), p), ("fe80::1", 443));
    }

    #[test]
    fn redirect_target_absolute() {
        let r = resolve_redirect_target("https://a.com/x", "https://b.com/y").unwrap();
        assert_eq!(r, "https://b.com/y");
    }

    #[test]
    fn redirect_target_path_absolute() {
        let r = resolve_redirect_target("https://a.com/x/y", "/z").unwrap();
        assert_eq!(r, "https://a.com/z");
    }

    #[test]
    fn redirect_target_path_relative() {
        let r = resolve_redirect_target("https://a.com/x/y", "z").unwrap();
        assert_eq!(r, "https://a.com/x/z");
    }

    #[test]
    fn strict_utf8_errors_on_invalid() {
        let bad = vec![0xFF, 0xFE, 0xFD];
        assert!(enforce_utf8(bad, true).is_err());
    }

    #[test]
    fn non_strict_utf8_replaces() {
        let (_bytes, replaced) = enforce_utf8(vec![0xFF, 0xFE, 0xFD], false).unwrap();
        assert!(replaced);
    }
}
