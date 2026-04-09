//! `ctxforge note` — write a memory note.

use crate::error::Result;
use crate::memory;
use crate::paths::CtxforgeRoot;

pub fn run(root: &CtxforgeRoot, body: Vec<String>, tag: Option<String>) -> Result<()> {
    if body.is_empty() {
        return Err(crate::error::CtxforgeError::Msg(
            "ctxforge note: missing note body (example: `ctxforge note --tag auth \"JWT in header\"`)".into(),
        ));
    }
    let body = body.join(" ");
    let note = memory::write_note(root, body, tag.clone())?;
    let ts = note.timestamp.format("%Y-%m-%d %H:%M UTC");
    match tag {
        Some(t) => println!("noted [{ts}] [{t}] {}", note.body),
        None => println!("noted [{ts}] {}", note.body),
    }
    Ok(())
}
