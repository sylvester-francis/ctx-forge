//! Template wrapper splitting for the preview.

use crate::paths::CtxforgeRoot;

pub struct Wrapped {
    pub prefix: String,
    pub suffix: String,
}

/// Split a scenario body around `{{task}}` / `{{bundle}}`:
/// `prefix` is text before the earliest placeholder, `suffix` is text after
/// the latest. The structural preview drops text between the two; the
/// composed prompt (P overlay and delivery) still uses the whole template.
pub fn load_wrapped(root: &CtxforgeRoot, name: &str) -> Result<Wrapped, String> {
    let body = crate::scenario::load_body(root, name)?;
    Ok(split(&body))
}

fn split(body: &str) -> Wrapped {
    let positions = [find(body, "{{task}}"), find(body, "{{bundle}}")];
    let first = positions.iter().copied().flatten().min();
    let last = positions
        .iter()
        .copied()
        .flatten()
        .max_by_key(|(_, end)| *end);

    let prefix = match first {
        Some((start, _)) => body[..start].trim().to_string(),
        None => body.trim().to_string(),
    };
    let suffix = match last {
        Some((_, end)) => body[end..].trim().to_string(),
        None => String::new(),
    };
    Wrapped { prefix, suffix }
}

fn find(haystack: &str, needle: &str) -> Option<(usize, usize)> {
    haystack
        .find(needle)
        .map(|start| (start, start + needle.len()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_with_task_then_bundle() {
        let w = split("PREFIX\n{{task}}\nMID\n{{bundle}}\nSUFFIX");
        assert_eq!(w.prefix, "PREFIX");
        assert_eq!(w.suffix, "SUFFIX");
    }

    #[test]
    fn split_with_bundle_then_task() {
        let w = split("PREFIX\n{{bundle}}\nMID\n{{task}}\nSUFFIX");
        assert_eq!(w.prefix, "PREFIX");
        assert_eq!(w.suffix, "SUFFIX");
    }

    #[test]
    fn split_without_placeholders_returns_whole_body_as_prefix() {
        let w = split("just some text");
        assert_eq!(w.prefix, "just some text");
        assert_eq!(w.suffix, "");
    }

    #[test]
    fn split_with_only_task_placeholder() {
        let w = split("PRE\n{{task}}\nPOST");
        assert_eq!(w.prefix, "PRE");
        assert_eq!(w.suffix, "POST");
    }
}
