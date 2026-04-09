//! `ctxforge status` — show bundle items with token counts and percentages.

use crate::bundle::Bundle;
use crate::error::Result;
use crate::models;
use crate::paths::CtxforgeRoot;
use crate::resolve;
use crate::tokens;

pub fn run(root: &CtxforgeRoot, model_override: Option<&str>) -> Result<()> {
    let bundle = Bundle::load_or_default(root)?;

    if bundle.is_empty() {
        println!("(empty bundle — use `ctxforge add` to add files)");
        return Ok(());
    }

    let model_name = model_override
        .map(|s| s.to_string())
        .or(bundle.model.clone())
        .unwrap_or_else(|| models::DEFAULT_MODEL.to_string());
    let model = models::lookup(&model_name);

    let resolved = resolve::resolve_all(&bundle.items, root.project_root())?;

    // Per-item counts.
    let counts: Vec<tokens::TokenCount> = resolved
        .iter()
        .map(|r| tokens::count(&r.content, &model))
        .collect();
    let total: usize = counts.iter().map(|c| c.tokens).sum();

    // Header + rows.
    for (i, (r, c)) in resolved.iter().zip(counts.iter()).enumerate() {
        let pct = if total == 0 {
            0.0
        } else {
            (c.tokens as f64 / total as f64) * 100.0
        };
        println!(
            "  {:>2}  {:<40}  {:>10}  {:>5.1}%",
            i + 1,
            truncate(&r.item.display(), 40),
            c.format(),
            pct
        );
    }
    println!("  ──────────────────────────────────────────────────────────");
    let window_pct = (total as f64 / model.window as f64) * 100.0;
    println!(
        "  Total: {} items, {} tokens ({}, {} window, {:.1}% used)",
        bundle.len(),
        tokens::TokenCount {
            tokens: total,
            exact: counts.iter().all(|c| c.exact)
        }
        .format(),
        model.name,
        thousands(model.window),
        window_pct,
    );

    Ok(())
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let taken: String = s.chars().take(max - 1).collect();
        format!("{taken}…")
    }
}

fn thousands(n: usize) -> String {
    let s = n.to_string();
    let bytes = s.as_bytes();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, b) in bytes.iter().enumerate() {
        if i > 0 && (bytes.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(*b as char);
    }
    out
}
