//! `ctxforge suggest` CLI handler.

use crate::bundle::Bundle;
use crate::error::Result;
use crate::paths::CtxforgeRoot;
use crate::suggest::{self, SuggestOptions, Suggestion};

pub fn run(
    root: &CtxforgeRoot,
    all: bool,
    missing_only: bool,
    as_json: bool,
    apply: bool,
    yes: bool,
) -> Result<i32> {
    let bundle = Bundle::load_or_default(root)?;
    let project_root = root.project_root();
    let options = SuggestOptions {
        scan_all_project: all,
        missing_only,
    };
    let report = suggest::run_suggest(&bundle, project_root, &options)?;

    if as_json {
        println!("{}", suggest::report::render_json(&report));
        return Ok(exit_code(&report, apply));
    }

    if !apply {
        print!("{}", suggest::report::render_human(&report));
        return Ok(exit_code(&report, apply));
    }

    if report.is_empty() {
        println!("✓ nothing to apply");
        return Ok(0);
    }

    print!("{}", suggest::report::render_human(&report));
    if !yes && !confirm_apply(report.total()) {
        println!("cancelled");
        return Ok(2);
    }

    apply_all(root, &report)
}

fn exit_code(report: &suggest::SuggestReport, applied: bool) -> i32 {
    if report.is_empty() || applied {
        0
    } else {
        2
    }
}

fn confirm_apply(total: usize) -> bool {
    use std::io::Write;
    print!("Apply all {total} suggestions? [y/N] ");
    let _ = std::io::stdout().flush();
    let mut line = String::new();
    if std::io::stdin().read_line(&mut line).is_err() {
        return false;
    }
    let answer = line.trim().to_ascii_lowercase();
    matches!(answer.as_str(), "y" | "yes")
}

fn apply_all(root: &CtxforgeRoot, report: &suggest::SuggestReport) -> Result<i32> {
    let mut applied = 0;
    let mut failed = 0;
    for m in &report.missing {
        match suggest::apply_suggestion(&Suggestion::Missing(m), root) {
            Ok(()) => {
                applied += 1;
                println!("  ✓ added: {}", m.name);
            }
            Err(e) => {
                failed += 1;
                eprintln!("  ✗ {}: {e}", m.name);
            }
        }
    }
    for s in &report.stale {
        match suggest::apply_suggestion(&Suggestion::Stale(s), root) {
            Ok(()) => {
                applied += 1;
                println!("  ✓ removed: {}", s.name);
            }
            Err(e) => {
                failed += 1;
                eprintln!("  ✗ {}: {e}", s.name);
            }
        }
    }
    println!("applied {applied}, failed {failed}");
    Ok(if failed == 0 { 0 } else { 1 })
}
