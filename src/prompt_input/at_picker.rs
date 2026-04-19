//! `@` popover: fuzzy file mentions inside the prompt input.

#![allow(dead_code)]

use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use ignore::WalkBuilder;
use std::path::{Path, PathBuf};

/// Cap on ranked results — keeps fuzzy scoring responsive on large monorepos.
pub const RESULT_LIMIT: usize = 50;

/// Walk the project respecting `.gitignore`; returns project-relative paths.
pub fn walk_files(root: &Path) -> Vec<PathBuf> {
    WalkBuilder::new(root)
        .git_ignore(true)
        // Dotfiles (`.gitignore`, `.github/workflows/*`) are legitimate picks.
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

/// Top-N fuzzy matches for `query`; empty query returns the first N in order.
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
    scored.sort_by_key(|s| std::cmp::Reverse(s.0));
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
