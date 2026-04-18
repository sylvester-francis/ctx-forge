//! GitHub context miner — Project stack enrichment (Capability A) and
//! gh:// specific-resource attachment (Capability B).

pub mod forge;
pub mod parse;
pub mod render;
#[cfg(feature = "fetch")]
pub mod fetch;
