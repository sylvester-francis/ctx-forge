//! `ctxforge profiles` — list or remove profiles.

use crate::cli::ProfilesAction;
use crate::error::Result;
use crate::paths::CtxforgeRoot;
use crate::profile;

pub fn run(root: &CtxforgeRoot, action: Option<ProfilesAction>) -> Result<()> {
    match action {
        None => {
            let list = profile::list(root)?;
            if list.is_empty() {
                println!("(no profiles)");
            } else {
                for name in list {
                    println!("  {name}");
                }
            }
            Ok(())
        }
        Some(ProfilesAction::Rm { name }) => {
            profile::remove(root, &name)?;
            println!("removed profile `{name}`");
            Ok(())
        }
    }
}
