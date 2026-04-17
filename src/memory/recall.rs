//! Recall: filter memory notes by tag, search string, and time window.
//!
//! All filtering is applied to the `Vec<Note>` returned by
//! `index::read_all`. Pure logic, no I/O.

#![allow(dead_code)]

use crate::memory::note::Note;
use chrono::{DateTime, Duration, Utc};

/// Filter for `recall`. All fields are optional; an empty filter
/// returns all notes.
#[derive(Debug, Clone, Default)]
pub struct RecallFilter {
    pub tag: Option<String>,
    pub search: Option<String>,
    pub since: Option<DateTime<Utc>>,
    pub limit: Option<usize>,
}

impl RecallFilter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse a human-readable duration like `"1w"`, `"3d"`, `"12h"`,
    /// `"30m"` into `Utc::now() - duration`. Used by CLI `--since`.
    pub fn since_from_duration(s: &str) -> Option<DateTime<Utc>> {
        if s.len() < 2 {
            return None;
        }
        let (num_part, unit) = s.split_at(s.len() - 1);
        let n: i64 = num_part.parse().ok()?;
        let duration = match unit {
            "w" => Duration::weeks(n),
            "d" => Duration::days(n),
            "h" => Duration::hours(n),
            "m" => Duration::minutes(n),
            _ => return None,
        };
        Some(Utc::now() - duration)
    }
}

/// Apply `filter` to `notes`, returning matching notes in reverse order
/// (newest first). Respects `limit` if set.
pub fn recall(notes: &[Note], filter: &RecallFilter) -> Vec<Note> {
    let mut out: Vec<Note> = notes
        .iter()
        .filter(|n| matches_filter(n, filter))
        .cloned()
        .collect();

    out.sort_by_key(|n| std::cmp::Reverse(n.timestamp));

    if let Some(limit) = filter.limit {
        out.truncate(limit);
    }
    out
}

fn matches_filter(note: &Note, filter: &RecallFilter) -> bool {
    if let Some(tag) = &filter.tag
        && note.tag.as_deref() != Some(tag.as_str())
    {
        return false;
    }
    if let Some(search) = &filter.search {
        let needle = search.to_lowercase();
        let body_match = note.body.to_lowercase().contains(&needle);
        let tag_match = note
            .tag
            .as_deref()
            .is_some_and(|t| t.to_lowercase().contains(&needle));
        if !body_match && !tag_match {
            return false;
        }
    }
    if let Some(since) = &filter.since
        && note.timestamp < *since
    {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn note_at(ts_offset_secs: i64, tag: Option<&str>, body: &str) -> Note {
        Note {
            timestamp: Utc::now() - Duration::seconds(ts_offset_secs),
            tag: tag.map(String::from),
            body: body.to_string(),
        }
    }

    #[test]
    fn empty_filter_returns_all_sorted_newest_first() {
        let notes = vec![
            note_at(10, Some("a"), "one"),
            note_at(5, Some("b"), "two"),
            note_at(0, None, "three"),
        ];
        let r = recall(&notes, &RecallFilter::new());
        assert_eq!(r.len(), 3);
        assert_eq!(r[0].body, "three");
        assert_eq!(r[2].body, "one");
    }

    #[test]
    fn tag_filter_only_matches_exact_tag() {
        let notes = vec![
            note_at(10, Some("auth"), "one"),
            note_at(5, Some("tls"), "two"),
            note_at(0, None, "three"),
        ];
        let r = recall(
            &notes,
            &RecallFilter {
                tag: Some("auth".into()),
                ..Default::default()
            },
        );
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].body, "one");
    }

    #[test]
    fn search_matches_body_case_insensitive() {
        let notes = vec![
            note_at(10, None, "Abandoned rustls 0.22"),
            note_at(0, None, "Use tokio::select"),
        ];
        let r = recall(
            &notes,
            &RecallFilter {
                search: Some("RUSTLS".into()),
                ..Default::default()
            },
        );
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].body, "Abandoned rustls 0.22");
    }

    #[test]
    fn search_matches_tag_text() {
        let notes = vec![
            note_at(10, Some("auth"), "nothing relevant"),
            note_at(0, None, "unrelated"),
        ];
        let r = recall(
            &notes,
            &RecallFilter {
                search: Some("auth".into()),
                ..Default::default()
            },
        );
        assert_eq!(r.len(), 1);
    }

    #[test]
    fn since_filter_excludes_old_notes() {
        let notes = vec![
            note_at(3600, None, "an hour ago"),
            note_at(10, None, "just now"),
        ];
        let cutoff = Utc::now() - Duration::seconds(60);
        let r = recall(
            &notes,
            &RecallFilter {
                since: Some(cutoff),
                ..Default::default()
            },
        );
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].body, "just now");
    }

    #[test]
    fn limit_truncates_after_sort() {
        let notes = vec![
            note_at(30, None, "oldest"),
            note_at(20, None, "middle"),
            note_at(10, None, "newest"),
        ];
        let r = recall(
            &notes,
            &RecallFilter {
                limit: Some(2),
                ..Default::default()
            },
        );
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].body, "newest");
        assert_eq!(r[1].body, "middle");
    }

    #[test]
    fn since_from_duration_parses_common_units() {
        assert!(RecallFilter::since_from_duration("1w").is_some());
        assert!(RecallFilter::since_from_duration("3d").is_some());
        assert!(RecallFilter::since_from_duration("12h").is_some());
        assert!(RecallFilter::since_from_duration("30m").is_some());
    }

    #[test]
    fn since_from_duration_rejects_bad_input() {
        assert!(RecallFilter::since_from_duration("").is_none());
        assert!(RecallFilter::since_from_duration("x").is_none());
        assert!(RecallFilter::since_from_duration("abcd").is_none());
        assert!(RecallFilter::since_from_duration("1y").is_none());
    }
}
