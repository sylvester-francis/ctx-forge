//! Auto-suggest context — deterministic detector for missing and stale
//! documentation entries in the bundle's Project stack.
//!
//! "Missing" = a bundle (or project) file imports a package with no
//! matching `DocsSource` entry. "Stale" = a `DocsSource` entry exists
//! but no bundle/project file imports the package.
//!
//! Output is for the user, not the LLM — suggestions never leak into
//! `ctxforge export` rendered output.

pub mod detect;
pub mod match_;
pub mod report;
pub mod stdlib;
