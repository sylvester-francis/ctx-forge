//! The `Note` data model.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Note {
    pub timestamp: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    pub body: String,
}

impl Note {
    pub fn new(body: impl Into<String>, tag: Option<String>) -> Self {
        Note {
            timestamp: Utc::now(),
            tag,
            body: body.into(),
        }
    }

    pub fn to_jsonl(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    pub fn from_jsonl(line: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(line)
    }

    pub fn to_markdown_block(&self) -> String {
        let ts = self.timestamp.format("%Y-%m-%d %H:%M UTC");
        let heading = match &self.tag {
            Some(t) => format!("## {ts} — {t}\n\n"),
            None => format!("## {ts}\n\n"),
        };
        format!("{heading}{}\n\n---\n", self.body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_sets_body_and_tag() {
        let n = Note::new("hello", Some("auth".into()));
        assert_eq!(n.body, "hello");
        assert_eq!(n.tag.as_deref(), Some("auth"));
    }

    #[test]
    fn new_untagged_has_none_tag() {
        let n = Note::new("hello", None);
        assert!(n.tag.is_none());
    }

    #[test]
    fn jsonl_roundtrip() {
        let n = Note::new("body", Some("tag".into()));
        let line = n.to_jsonl().unwrap();
        let parsed = Note::from_jsonl(&line).unwrap();
        assert_eq!(parsed, n);
    }

    #[test]
    fn jsonl_omits_tag_when_none() {
        let n = Note::new("body", None);
        let line = n.to_jsonl().unwrap();
        assert!(!line.contains("tag"));
    }

    #[test]
    fn markdown_block_with_tag_has_tag_in_heading() {
        let n = Note::new("decided X", Some("auth".into()));
        let md = n.to_markdown_block();
        assert!(md.starts_with("## "));
        assert!(md.contains(" — auth"));
        assert!(md.contains("decided X"));
        assert!(md.ends_with("---\n"));
    }

    #[test]
    fn markdown_block_without_tag_has_no_dash() {
        let n = Note::new("decided X", None);
        let md = n.to_markdown_block();
        assert!(!md.contains(" — "));
    }
}
