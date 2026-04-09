//! `ctxforge recall` — search memory notes.

use crate::error::Result;
use crate::memory::{self, RecallFilter};
use crate::paths::CtxforgeRoot;

pub fn run(
    root: &CtxforgeRoot,
    tag: Option<String>,
    search: Option<String>,
    since: Option<String>,
    limit: usize,
) -> Result<()> {
    let all = memory::index::read_all(root)?;
    if all.is_empty() {
        println!("(no notes yet — use `ctxforge note` to add one)");
        return Ok(());
    }

    let since_dt = match since.as_deref() {
        Some(s) => match RecallFilter::since_from_duration(s) {
            Some(dt) => Some(dt),
            None => {
                return Err(crate::error::CtxforgeError::Msg(format!(
                    "invalid --since value `{s}` (expected e.g. 1w, 3d, 12h, 30m)"
                )));
            }
        },
        None => None,
    };

    let filter = RecallFilter {
        tag,
        search,
        since: since_dt,
        limit: Some(limit),
    };

    let matches = memory::run_recall(&all, &filter);
    if matches.is_empty() {
        println!("(no matching notes)");
        return Ok(());
    }

    for note in &matches {
        let ts = note.timestamp.format("%Y-%m-%d %H:%M UTC");
        match &note.tag {
            Some(t) => println!("  [{ts}] [{t}] {}", note.body),
            None => println!("  [{ts}] {}", note.body),
        }
    }
    println!("  ({} of {} note(s))", matches.len(), all.len());
    Ok(())
}
