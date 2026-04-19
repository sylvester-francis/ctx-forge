//! Canonical doc URL templates per ecosystem.

use crate::source::Ecosystem;

/// Go module paths may contain slashes — pkg.go.dev handles them unencoded.
pub fn canonical_url(eco: Ecosystem, name: &str, version: &str) -> String {
    match eco {
        Ecosystem::Rust => format!("https://docs.rs/{name}/{version}/"),
        Ecosystem::Js => {
            let pkg = urlencoding::encode(name);
            format!("https://www.npmjs.com/package/{pkg}/v/{version}")
        }
        Ecosystem::Python => format!("https://pypi.org/project/{name}/{version}/"),
        Ecosystem::Go => format!("https://pkg.go.dev/{name}@{version}"),
    }
}

/// Registry API URL for description lookup. Go returns None.
pub fn description_url(eco: Ecosystem, name: &str) -> Option<String> {
    match eco {
        Ecosystem::Rust => Some(format!("https://crates.io/api/v1/crates/{name}")),
        Ecosystem::Js => {
            let pkg = urlencoding::encode(name);
            Some(format!("https://registry.npmjs.org/{pkg}"))
        }
        Ecosystem::Python => Some(format!("https://pypi.org/pypi/{name}/json")),
        Ecosystem::Go => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_canonical_url() {
        assert_eq!(
            canonical_url(Ecosystem::Rust, "axum", "0.7.5"),
            "https://docs.rs/axum/0.7.5/",
        );
    }

    #[test]
    fn js_canonical_url_handles_scoped_packages() {
        assert_eq!(
            canonical_url(Ecosystem::Js, "@prisma/client", "5.0.0"),
            "https://www.npmjs.com/package/%40prisma%2Fclient/v/5.0.0",
        );
    }

    #[test]
    fn python_canonical_url() {
        assert_eq!(
            canonical_url(Ecosystem::Python, "fastapi", "0.115.0"),
            "https://pypi.org/project/fastapi/0.115.0/",
        );
    }

    #[test]
    fn go_canonical_url_keeps_slashes() {
        assert_eq!(
            canonical_url(Ecosystem::Go, "github.com/gin-gonic/gin", "v1.10.0"),
            "https://pkg.go.dev/github.com/gin-gonic/gin@v1.10.0",
        );
    }

    #[test]
    fn go_description_url_is_none() {
        assert!(description_url(Ecosystem::Go, "anything").is_none());
    }
}
