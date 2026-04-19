//! Compiled-in classification registry: (ecosystem, name) -> DocsTier.

use crate::source::{DocsTier, Ecosystem};
use std::collections::HashMap;

pub struct Registry {
    entries: HashMap<(Ecosystem, String), DocsTier>,
}

#[derive(serde::Deserialize)]
struct RegistryFile {
    #[serde(default)]
    rust: HashMap<String, Entry>,
    #[serde(default)]
    js: HashMap<String, Entry>,
    #[serde(default)]
    python: HashMap<String, Entry>,
    #[serde(default)]
    go: HashMap<String, Entry>,
}

#[derive(serde::Deserialize)]
struct Entry {
    tier: DocsTier,
}

impl Registry {
    pub fn builtin() -> Self {
        let raw = include_str!("registry.toml");
        let parsed: RegistryFile =
            toml::from_str(raw).expect("registry.toml must parse (checked at build time)");
        let mut entries = HashMap::new();
        for (name, e) in parsed.rust {
            entries.insert((Ecosystem::Rust, name), e.tier);
        }
        for (name, e) in parsed.js {
            entries.insert((Ecosystem::Js, name), e.tier);
        }
        for (name, e) in parsed.python {
            entries.insert((Ecosystem::Python, name), e.tier);
        }
        for (name, e) in parsed.go {
            entries.insert((Ecosystem::Go, name), e.tier);
        }
        Self { entries }
    }

    pub fn classify(&self, ecosystem: Ecosystem, name: &str) -> DocsTier {
        self.entries
            .get(&(ecosystem, name.to_string()))
            .copied()
            .unwrap_or(DocsTier::Library)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_registry_parses() {
        let r = Registry::builtin();
        assert_eq!(r.classify(Ecosystem::Rust, "axum"), DocsTier::Framework);
        assert_eq!(r.classify(Ecosystem::Rust, "tokio"), DocsTier::AsyncRuntime);
        assert_eq!(r.classify(Ecosystem::Rust, "sqlx"), DocsTier::Database);
        assert_eq!(r.classify(Ecosystem::Js, "next"), DocsTier::Framework);
        assert_eq!(
            r.classify(Ecosystem::Python, "fastapi"),
            DocsTier::Framework
        );
        assert_eq!(
            r.classify(Ecosystem::Go, "github.com/gin-gonic/gin"),
            DocsTier::Framework,
        );
    }

    #[test]
    fn unknown_dep_falls_to_library() {
        let r = Registry::builtin();
        assert_eq!(
            r.classify(Ecosystem::Rust, "some-random-unknown-crate"),
            DocsTier::Library,
        );
    }
}
