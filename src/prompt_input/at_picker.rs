//! `@` popover: fuzzy file mentions inside the prompt input.
//!
//! When the user types `@` while focused on the prompt, we walk the
//! project for files (respecting `.gitignore`), cache the list, and
//! fuzzy-rank against whatever is typed after the `@`. A selection
//! inserts the path at the cursor and auto-adds the file to the
//! bundle (Task 21).

#![allow(dead_code)]

use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use ignore::WalkBuilder;
use std::path::{Path, PathBuf};

/// Cap on ranked results. Fuzzy scoring a 100k-file monorepo on every
/// keystroke stays responsive with this ceiling in place; the user
/// essentially never scrolls past the first handful of hits.
pub const RESULT_LIMIT: usize = 50;

/// Walk the project respecting `.gitignore`, returning every file as a
/// project-relative path. Called once when the popover opens; results
/// are cached on the Mode variant for the lifetime of the picker.
pub fn walk_files(root: &Path) -> Vec<PathBuf> {
    WalkBuilder::new(root)
        .git_ignore(true)
        // Don't hide dotfiles — `.gitignore` / `.github/workflows/*` are
        // legitimate picks a user might want to mention.
        .hidden(false)
        .build()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_some_and(|ft| ft.is_file()))
        .map(|e| {
            e.path()
                .strip_prefix(root)
                .unwrap_or(e.path())
                .to_path_buf()
        })
        .collect()
}

/// Return the top-N fuzzy matches for `query` ranked against `all`. An
/// empty query returns the first N in their on-disk order.
pub fn rank(all: &[PathBuf], query: &str, limit: usize) -> Vec<PathBuf> {
    if query.is_empty() {
        return all.iter().take(limit).cloned().collect();
    }
    let matcher = SkimMatcherV2::default();
    let mut scored: Vec<(i64, &PathBuf)> = all
        .iter()
        .filter_map(|p| {
            let s = p.to_string_lossy();
            matcher.fuzzy_match(&s, query).map(|score| (score, p))
        })
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0));
    scored
        .into_iter()
        .take(limit)
        .map(|(_, p)| p.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn sample() -> Vec<PathBuf> {
        vec![
            PathBuf::from("src/auth.rs"),
            PathBuf::from("src/authz.rs"),
            PathBuf::from("src/auth_test.rs"),
            PathBuf::from("src/unrelated.rs"),
        ]
    }

    #[test]
    fn empty_query_returns_first_n() {
        let all = sample();
        let got = rank(&all, "", 2);
        assert_eq!(got.len(), 2);
        assert_eq!(got[0], PathBuf::from("src/auth.rs"));
    }

    #[test]
    fn query_filters_to_matches() {
        let all = sample();
        let got = rank(&all, "auth", 50);
        let names: Vec<_> = got
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();
        assert!(names.iter().all(|n| n.contains("auth")));
        assert!(!names.iter().any(|n| n.contains("unrelated")));
    }

    #[test]
    fn rank_respects_limit() {
        let all = sample();
        let got = rank(&all, "auth", 2);
        assert!(got.len() <= 2);
    }
}
