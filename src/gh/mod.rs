//! GitHub context miner — Project stack enrichment (Capability A) and
//! gh:// specific-resource attachment (Capability B).

#[cfg(feature = "fetch")]
pub mod fetch;
pub mod forge;
pub mod parse;
pub mod render;
