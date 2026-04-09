//! Cross-platform clipboard wrapper. Wraps `arboard` and converts its errors
//! to our crate error type.
//!
//! Note: tests require an active clipboard, so they're marked #[ignore] and
//! run only manually with `cargo test -- --ignored` on a desktop machine.

#![allow(dead_code)]

use crate::error::{CtxforgeError, Result};

pub fn set(text: &str) -> Result<()> {
    let mut cb =
        arboard::Clipboard::new().map_err(|e| CtxforgeError::Clipboard(format!("init: {e}")))?;
    cb.set_text(text.to_string())
        .map_err(|e| CtxforgeError::Clipboard(format!("set: {e}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // requires active display server
    fn set_clipboard_roundtrip() {
        set("hello from ctxforge").unwrap();
        let mut cb = arboard::Clipboard::new().unwrap();
        assert_eq!(cb.get_text().unwrap(), "hello from ctxforge");
    }
}
