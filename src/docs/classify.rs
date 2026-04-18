//! Thin wrapper over Registry for consumers that don't need direct access.

use crate::docs::registry::Registry;
use crate::source::{DocsTier, Ecosystem};

pub fn classify(registry: &Registry, ecosystem: Ecosystem, name: &str) -> DocsTier {
    registry.classify(ecosystem, name)
}
