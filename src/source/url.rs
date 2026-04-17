//! `UrlSource` — a URL-backed context source. Validation, SSRF checks,
//! and fetch logic land in Tasks 7 and 8; this is the serde shape only.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UrlSource {
    pub url: String,
}
