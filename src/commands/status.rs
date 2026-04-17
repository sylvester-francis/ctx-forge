//! `ctxforge status` — show the current bundle and token usage.

use crate::bundle::Bundle;
use crate::error::Result;
use crate::models;
use crate::output;
use crate::paths::CtxforgeRoot;
use crate::resolve;
use crate::tokens;
use comfy_table::presets::UTF8_FULL;
use comfy_table::{Cell, Color, ContentArrangement, Table};

pub fn run(root: &CtxforgeRoot, model_override: Option<&str>) -> Result<()> {
    let bundle = Bundle::load_or_default(root)?;

    if bundle.is_empty() {
        println!("(empty bundle — use `ctxforge add` to add files)");
        return Ok(());
    }

    let model_name = model_override
        .map(String::from)
        .or_else(|| bundle.model.clone())
        .unwrap_or_else(|| models::DEFAULT_MODEL.to_string());
    let model = models::lookup(&model_name);

    let resolved = resolve::resolve_all(&bundle.items, root.project_root())?;
    let item_tokens: Vec<usize> = resolved
        .iter()
        .map(|r| tokens::count(&r.content, &model).tokens)
        .collect();
    let total_tokens: usize = item_tokens.iter().sum();
    let pct = if model.window > 0 {
        (total_tokens as f64 / model.window as f64) * 100.0
    } else {
        0.0
    };

    // Header line above the table.
    println!(
        " ctxforge  ·  {} items  ·  ~{} tokens  ·  {} ({} window)",
        bundle.len(),
        total_tokens,
        model.name,
        format_window(model.window),
    );

    // Table
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            Cell::new("#").fg(Color::DarkGrey),
            Cell::new("path").fg(Color::DarkGrey),
            Cell::new("tokens").fg(Color::DarkGrey),
            Cell::new("%").fg(Color::DarkGrey),
        ]);

    for (i, (item, tok)) in bundle.items.iter().zip(item_tokens.iter()).enumerate() {
        let item_pct = if total_tokens > 0 {
            (*tok as f64 / total_tokens as f64) * 100.0
        } else {
            0.0
        };
        let pct_color = if item_pct > 25.0 {
            Color::Red
        } else if item_pct > 10.0 {
            Color::Yellow
        } else {
            Color::Green
        };
        table.add_row(vec![
            Cell::new(i + 1),
            Cell::new(item.display()),
            Cell::new(format!("~{tok}")),
            Cell::new(format!("{item_pct:5.1}%")).fg(pct_color),
        ]);
    }

    println!("{table}");

    // Budget gauge line under the table.
    let bar_color_label = if pct < 25.0 {
        "green"
    } else if pct < 50.0 {
        "yellow"
    } else if pct < 75.0 {
        "orange"
    } else {
        "red"
    };
    println!();
    println!(
        "   budget:  [{}]  {:.1}%  ({})",
        render_bar(pct, 42),
        pct,
        bar_color_label,
    );

    // Hotspot callout: warn if any single item exceeds 25% of the bundle.
    if let Some((idx, hot_pct)) = find_hotspot(&item_tokens, total_tokens) {
        let item = &bundle.items[idx];
        println!();
        output::warn(&format!(
            "item {} ({}) consumes {:.1}% of the bundle.",
            idx + 1,
            item.display(),
            hot_pct
        ));
        let narrow_hint = item
            .source
            .display_path()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| item.source.display_label());
        println!(
            "  Tip: narrow to a line range with: ctxforge rm {} && ctxforge add {}:10-50",
            idx + 1,
            narrow_hint
        );
    }

    Ok(())
}

fn format_window(window: usize) -> String {
    if window >= 1_000_000 {
        format!("{:.1}M", window as f64 / 1_000_000.0)
    } else if window >= 1_000 {
        format!("{}k", window / 1_000)
    } else {
        format!("{window}")
    }
}

fn render_bar(pct: f64, width: usize) -> String {
    let filled = ((pct / 100.0) * width as f64).round() as usize;
    let filled = filled.min(width);
    let empty = width - filled;
    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}

fn find_hotspot(item_tokens: &[usize], total_tokens: usize) -> Option<(usize, f64)> {
    if total_tokens == 0 || item_tokens.is_empty() {
        return None;
    }
    let (idx, max) = item_tokens.iter().enumerate().max_by_key(|(_, t)| *t)?;
    let pct = (*max as f64 / total_tokens as f64) * 100.0;
    if pct > 25.0 { Some((idx, pct)) } else { None }
}
