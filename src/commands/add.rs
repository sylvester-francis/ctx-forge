//! `ctxforge add` — add files, globs, line-ranges, or diff'd files to the bundle.

use crate::bundle::{Bundle, Item};
use crate::error::{CtxforgeError, Result};
use crate::paths::CtxforgeRoot;
use crate::walk;
use std::path::Path;

pub fn run(
    root: &CtxforgeRoot,
    _cwd: &Path,
    patterns: Vec<String>,
    exclude: Vec<String>,
    diff: Option<String>,
) -> Result<()> {
    let project_root = root.project_root().to_path_buf();
    let mut bundle = Bundle::load_or_default(root)?;
    let mut added_count: usize = 0;

    // --diff mode: fetch changed files from git and queue them as whole-file items.
    if let Some(branch) = diff {
        let changed = crate::git::changed_files(&project_root, &branch)?;
        for p in changed {
            let item = Item {
                path: p,
                kind: crate::bundle::ItemKind::File,
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
            eprintln!("warning: `{pat}` matched no files");
        }
        for p in paths {
            let item = Item {
                path: p,
                kind: crate::bundle::ItemKind::File,
                label: None,
            };
            bundle.add(item);
            added_count += 1;
        }
    }

    if patterns.is_empty() && added_count == 0 {
        return Err(CtxforgeError::Msg(
            "ctxforge add: no patterns provided (use --diff or pass path arguments)".into(),
        ));
    }

    bundle.save(root)?;
    println!(
        "added {added_count} item(s); bundle now has {}",
        bundle.len()
    );
    Ok(())
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
