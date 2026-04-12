//! `ctxforge add` — add files, globs, line-ranges, functions, types, or
//! diff'd files to the bundle.

use crate::bundle::{Bundle, Item, ItemKind};
use crate::error::{CtxforgeError, Result};
use crate::output;
use crate::paths::CtxforgeRoot;
use crate::walk;
use std::path::{Path, PathBuf};

pub fn run(
    root: &CtxforgeRoot,
    _cwd: &Path,
    patterns: Vec<String>,
    exclude: Vec<String>,
    diff: Option<String>,
    functions: Vec<String>,
    types: Vec<String>,
) -> Result<()> {
    let project_root = root.project_root().to_path_buf();
    let mut bundle = Bundle::load_or_default(root)?;
    let mut added_count: usize = 0;

    // --fn <name>: requires exactly one file path in patterns.
    if !functions.is_empty() {
        let file_path = require_single_file(&patterns, "--fn")?;
        let pb = output::spinner(&format!("scanning {file_path} for functions..."));
        #[cfg(feature = "extract")]
        validate_symbols(
            &project_root,
            &file_path,
            &functions,
            crate::extract::extract_function,
            "function",
        );
        for name in &functions {
            let item = Item {
                path: PathBuf::from(&file_path),
                kind: ItemKind::Function { name: name.clone() },
                label: None,
            };
            bundle.add(item);
            added_count += 1;
        }
        pb.finish_and_clear();
    }

    // --type <name>: requires exactly one file path in patterns.
    if !types.is_empty() {
        let file_path = require_single_file(&patterns, "--type")?;
        let pb = output::spinner(&format!("scanning {file_path} for types..."));
        #[cfg(feature = "extract")]
        validate_symbols(
            &project_root,
            &file_path,
            &types,
            crate::extract::extract_type,
            "type",
        );
        for name in &types {
            let item = Item {
                path: PathBuf::from(&file_path),
                kind: ItemKind::Type { name: name.clone() },
                label: None,
            };
            bundle.add(item);
            added_count += 1;
        }
        pb.finish_and_clear();
    }

    // If --fn or --type were used, we're done with patterns (they served as the file path).
    if !functions.is_empty() || !types.is_empty() {
        bundle.save(root)?;
        output::success(&format!(
            "added {added_count} item(s); bundle now has {}",
            bundle.len()
        ));
        return Ok(());
    }

    // --diff mode: fetch changed files from git and queue them as whole-file items.
    if let Some(branch) = diff {
        let changed = crate::git::changed_files(&project_root, &branch)?;
        for p in changed {
            let item = Item {
                path: p,
                kind: ItemKind::File,
                label: None,
            };
            if !exclude_matches(&item.path, &exclude)? {
                bundle.add(item);
                added_count += 1;
            }
        }
    }

    // Expand each explicit pattern.
    for pat in &patterns {
        // Ranged paths (contain `:`) parse directly as a single item.
        if looks_like_ranged_path(pat) {
            let item = Item::parse_add_argument(pat)?;
            if !exclude_matches(&item.path, &exclude)? {
                bundle.add(item);
                added_count += 1;
            }
            continue;
        }

        // Otherwise expand via the walker (handles literal files, dirs, globs).
        let paths = walk::expand(pat, &project_root, &exclude)?;
        if paths.is_empty() {
            // Literal path (no glob metacharacters) that doesn't exist → NotFound.
            if !pat.contains('*') && !pat.contains('?') && !pat.contains('[') && !pat.contains('{')
            {
                let target = project_root.join(pat);
                let parent = target.parent().unwrap_or(&project_root);
                let siblings: Vec<String> = std::fs::read_dir(parent)
                    .ok()
                    .into_iter()
                    .flatten()
                    .filter_map(|entry| entry.ok())
                    .filter_map(|entry| entry.file_name().into_string().ok())
                    .collect();
                let query = target.file_name().and_then(|n| n.to_str()).unwrap_or("");
                let suggestions = output::close_matches(query, &siblings, 3);
                return Err(CtxforgeError::NotFound {
                    path: std::path::PathBuf::from(pat),
                    suggestions,
                });
            }
            output::warn(&format!("`{pat}` matched no files"));
        }
        for p in paths {
            let item = Item {
                path: p,
                kind: ItemKind::File,
                label: None,
            };
            bundle.add(item);
            added_count += 1;
        }
    }

    if patterns.is_empty() && added_count == 0 {
        return Err(CtxforgeError::Msg(
            "ctxforge add: no patterns provided (use --diff, --fn, --type, or pass path arguments)"
                .into(),
        ));
    }

    bundle.save(root)?;
    output::success(&format!(
        "added {added_count} item(s); bundle now has {}",
        bundle.len()
    ));
    Ok(())
}

/// Best-effort validation that each `name` actually exists in `file_path` as
/// `kind_label` (e.g. "function" or "type"). Warns via `output::warn` for any
/// name that can't be found — does not fail the add so existing scripts keep
/// working, but catches typos with a visible message. Silently tolerates
/// unsupported languages or unreadable files; resolve-time extraction will
/// surface those errors later.
#[cfg(feature = "extract")]
fn validate_symbols(
    project_root: &Path,
    file_path: &str,
    names: &[String],
    extractor: fn(&str, &str, &str) -> std::result::Result<Option<String>, String>,
    kind_label: &str,
) {
    let abs = project_root.join(file_path);
    let Ok(source) = std::fs::read_to_string(&abs) else {
        return;
    };
    let language = crate::lang::detect(Path::new(file_path));
    for name in names {
        match extractor(&source, language, name) {
            Ok(Some(_)) => {}
            Ok(None) => output::warn(&format!(
                "{kind_label} `{name}` not found in {file_path} (added anyway)"
            )),
            Err(_) => {
                // Unsupported language or query error — defer to resolve time.
            }
        }
    }
}

/// When using `--fn` or `--type`, exactly one file path must be provided.
fn require_single_file(patterns: &[String], flag: &str) -> Result<String> {
    if patterns.len() != 1 {
        return Err(CtxforgeError::Msg(format!(
            "{flag} requires exactly one file path (got {})",
            patterns.len()
        )));
    }
    Ok(patterns[0].clone())
}

fn looks_like_ranged_path(s: &str) -> bool {
    if let Some((_, after)) = s.rsplit_once(':') {
        after.contains('-') && after.chars().all(|c| c.is_ascii_digit() || c == '-')
    } else {
        false
    }
}

fn exclude_matches(path: &Path, excludes: &[String]) -> Result<bool> {
    use globset::{Glob, GlobSetBuilder};
    if excludes.is_empty() {
        return Ok(false);
    }
    let mut b = GlobSetBuilder::new();
    for e in excludes {
        b.add(Glob::new(e)?);
    }
    Ok(b.build()?.is_match(path))
}
