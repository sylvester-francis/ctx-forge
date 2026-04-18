//! Library docs gatherer. Scans manifest + lock files in the project root
//! (or recursively in monorepo mode), classifies deps via the built-in
//! registry, resolves canonical doc URLs, and attaches DocsSource items
//! to the bundle.

pub mod classify;
pub mod describe;
pub mod detect;
pub mod parsers;
pub mod registry;
pub mod resolve;
