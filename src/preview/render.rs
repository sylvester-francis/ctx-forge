//! Template wrapper splitting for the preview.

use crate::paths::CtxforgeRoot;

pub struct Wrapped {
    pub prefix: String,
    pub suffix: String,
}

/// Load a scenario's template body and split it into the text surrounding
/// the `{{task}}` and `{{bundle}}` placeholders:
///
///   prefix = text up to the earliest of the two placeholders
///   suffix = text after the latest of the two placeholders
///
/// Text between the two placeholders is dropped for the structural preview;
/// the full composed prompt (used by `P` overlay and by delivery) still
/// renders the whole template.
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

/// Returns `(start, end)` byte offsets of the first occurrence of `needle`
/// in `haystack`, or `None`.
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
