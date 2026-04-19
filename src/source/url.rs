//! `UrlSource` — HTTPS fetch with SSRF protection, content-type guard, etc.
//!
//! The fetcher lives in `src/fetch.rs`; this module owns the lexical
//! validation, the literal-IP guard, the DNS-resolve SSRF check, and the
//! content-type allowlist.

use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, ToSocketAddrs};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UrlSource {
    pub url: String,
}

/// Allow-list of response content types. Anything else is rejected.
pub const ACCEPTED_TYPES: &[&str] = &[
    "text/",
    "application/json",
    "application/xml",
    "application/x-yaml",
    "application/yaml",
];

pub fn is_accepted_content_type(raw: &str) -> bool {
    let lower = raw.to_ascii_lowercase();
    ACCEPTED_TYPES.iter().any(|p| lower.starts_with(p))
}

/// Lexical URL validation. Applied pre-fetch and again on every redirect
/// target. Does NOT do DNS — SSRF resolution happens in
/// `resolve_host_and_check_ssrf`.
pub fn validate_url(url: &str, allow_http: bool, allow_private_net: bool) -> Result<(), String> {
    if url.starts_with("http://") && !allow_http {
        return Err("http:// rejected — pass --allow-http to opt in".into());
    }
    if !url.starts_with("https://") && !url.starts_with("http://") {
        return Err(format!("unsupported protocol: {url}"));
    }

    let after = url.split("://").nth(1).unwrap_or("");
    let authority_end = after.find('/').unwrap_or(after.len());
    let authority = &after[..authority_end];

    if authority.contains('@') {
        return Err(
            "userinfo (user:pass@host) rejected — credentials must not appear in URLs".into(),
        );
    }

    let host = if let Some(stripped) = authority.strip_prefix('[') {
        if let Some(end) = stripped.find(']') {
            &stripped[..end]
        } else {
            return Err("malformed IPv6 authority (missing ])".into());
        }
    } else {
        authority.split(':').next().unwrap_or(authority)
    };
    if host.is_empty() {
        return Err("URL has no host".into());
    }

    if let Ok(ip) = host.parse::<IpAddr>()
        && !allow_private_net
        && is_private_ip(ip)
    {
        return Err(format!(
            "private/loopback IP `{ip}` rejected — use --allow-private-net"
        ));
    }

    let h = host.to_ascii_lowercase();
    if !allow_private_net && (h == "localhost" || h.ends_with(".internal") || h.ends_with(".local"))
    {
        return Err(format!(
            "host `{host}` looks internal — use --allow-private-net"
        ));
    }
    Ok(())
}

/// Resolve `host:port` and validate every resolved IP. Returns the set
/// of IPs so callers can enforce connection through exactly those.
pub fn resolve_host_and_check_ssrf(
    host: &str,
    port: u16,
    allow_private_net: bool,
) -> Result<Vec<IpAddr>, String> {
    let target = format!("{host}:{port}");
    let ips: Vec<IpAddr> = target
        .as_str()
        .to_socket_addrs()
        .map_err(|e| format!("DNS resolution failed for {host}: {e}"))?
        .map(|s| s.ip())
        .collect();
    if ips.is_empty() {
        return Err(format!("{host} resolves to no addresses"));
    }
    if !allow_private_net {
        for ip in &ips {
            if is_private_ip(*ip) {
                return Err(format!(
                    "host `{host}` resolves to private address {ip} — use --allow-private-net"
                ));
            }
        }
    }
    Ok(ips)
}

pub fn is_private_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_private_ipv4(v4),
        IpAddr::V6(v6) => is_private_ipv6(v6),
    }
}

fn is_private_ipv4(ip: Ipv4Addr) -> bool {
    ip.is_loopback()
        || ip.is_private()
        || ip.is_link_local()
        || ip.is_unspecified()
        || ip.is_broadcast()
        || ip.is_multicast()
        // CG-NAT / shared address space, not covered by is_private.
        || (ip.octets()[0] == 100 && (64..=127).contains(&ip.octets()[1]))
}

fn is_private_ipv6(ip: Ipv6Addr) -> bool {
    ip.is_loopback()
        || ip.is_unspecified()
        || ip.is_multicast()
        // Unique local addresses: fc00::/7
        || (ip.segments()[0] & 0xfe00) == 0xfc00
        // Link-local: fe80::/10
        || (ip.segments()[0] & 0xffc0) == 0xfe80
        // IPv4-mapped — reapply v4 rules.
        || ip.to_ipv4_mapped().is_some_and(is_private_ipv4)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_http_by_default() {
        assert!(validate_url("http://example.com", false, false).is_err());
    }

    #[test]
    fn allows_http_when_opted_in() {
        assert!(validate_url("http://example.com", true, false).is_ok());
    }

    #[test]
    fn rejects_literal_private_ipv4() {
        for url in &[
            "https://127.0.0.1/x",
            "https://10.0.0.1/x",
            "https://192.168.1.1/x",
            "https://172.16.0.1/x",
            "https://169.254.1.1/x",
            "https://100.64.0.1/x",
        ] {
            assert!(
                validate_url(url, false, false).is_err(),
                "should reject {url}"
            );
        }
    }

    #[test]
    fn rejects_literal_private_ipv6() {
        assert!(validate_url("https://[::1]/x", false, false).is_err());
        assert!(validate_url("https://[fe80::1]/x", false, false).is_err());
        assert!(validate_url("https://[fc00::1]/x", false, false).is_err());
    }

    #[test]
    fn rejects_internal_tld_and_localhost() {
        assert!(validate_url("https://service.internal/x", false, false).is_err());
        assert!(validate_url("https://x.local/x", false, false).is_err());
        assert!(validate_url("https://localhost/x", false, false).is_err());
    }

    #[test]
    fn rejects_userinfo() {
        assert!(validate_url("https://u:p@example.com/x", false, false).is_err());
    }

    #[test]
    fn accepts_known_content_types() {
        assert!(is_accepted_content_type("text/html; charset=utf-8"));
        assert!(is_accepted_content_type("application/json"));
        assert!(is_accepted_content_type("application/x-yaml"));
    }

    #[test]
    fn rejects_binary_content_types() {
        assert!(!is_accepted_content_type("image/png"));
        assert!(!is_accepted_content_type("application/octet-stream"));
        assert!(!is_accepted_content_type(""));
    }

    #[test]
    fn private_ip_classifies_known_ranges() {
        assert!(is_private_ip(IpAddr::from([127, 0, 0, 1])));
        assert!(is_private_ip(IpAddr::from([10, 0, 0, 1])));
        assert!(is_private_ip(IpAddr::from([169, 254, 1, 1])));
        assert!(!is_private_ip(IpAddr::from([8, 8, 8, 8])));
    }
}
