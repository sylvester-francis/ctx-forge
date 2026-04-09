//! `ctxforge resume` — show the current bundle + recent notes.

use crate::bundle::Bundle;
use crate::error::Result;
use crate::memory;
use crate::paths::CtxforgeRoot;

pub fn run(root: &CtxforgeRoot, memory_limit: usize) -> Result<()> {
    let bundle = Bundle::load_or_default(root)?;
    let notes = memory::collect_for_attach(root, false, None, memory_limit)?;

    // Bundle summary — mirrors `status` but terser.
    if bundle.is_empty() {
        println!("Bundle: (empty — use `ctxforge add` to start)");
    } else {
        println!("Bundle: {} item(s)", bundle.len());
        for (i, item) in bundle.items.iter().enumerate() {
            println!("  {:>2}  {}", i + 1, item.display());
        }
    }

    // Recent notes summary.
    if notes.is_empty() {
        println!("\n(no notes yet — use `ctxforge note` to capture decisions)");
    } else {
        println!("\nRecent notes ({} most recent):", notes.len());
        for note in &notes {
            let ts = note.timestamp.format("%Y-%m-%d %H:%M UTC");
            match &note.tag {
                Some(t) => println!("  [{ts}] [{t}] {}", note.body),
                None => println!("  [{ts}] {}", note.body),
            }
        }
    }

    println!("\nRun `ctxforge copy` to hand off to your agent.");
    Ok(())
}
