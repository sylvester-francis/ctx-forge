//! `ctxforge save [name]` — snapshot the current bundle as a named profile.

use crate::bundle::Bundle;
use crate::error::{CtxforgeError, Result};
use crate::output;
use crate::paths::CtxforgeRoot;
use crate::profile;
use dialoguer::{Input, theme::ColorfulTheme};
use std::io::IsTerminal;

pub fn run(root: &CtxforgeRoot, name: Option<&str>) -> Result<()> {
    let resolved_name = match name {
        Some(n) => n.to_string(),
        None => {
            if !std::io::stdin().is_terminal() {
                return Err(CtxforgeError::Msg(
                    "profile name required (stdin is not a TTY, cannot prompt)".into(),
                ));
            }
            Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Profile name")
                .interact_text()
                .map_err(|e| CtxforgeError::Msg(format!("prompt cancelled: {e}")))?
        }
    };

    let bundle = Bundle::load_or_default(root)?;
    profile::save(root, &resolved_name, &bundle)?;
    output::success(&format!("saved profile '{resolved_name}'"));
    Ok(())
}
