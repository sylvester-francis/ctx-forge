//! Render a fetched GhBody into the Markdown section that ends up in
//! the exported prompt. Truncates body with `…[N more chars]` marker.

#[cfg(feature = "fetch")]
use crate::gh::fetch::GhBody;
use crate::source::GhResource;

/// Maximum body size for issue / PR / release sections in the rendered
/// prompt. Keeps the token budget bounded when a user attaches a
/// chatty issue.
const ISSUE_PR_RELEASE_CAP: usize = 2 * 1_024;

/// Blobs are changelogs and similar long files — larger cap.
const BLOB_CAP: usize = 10 * 1_024;

#[cfg(feature = "fetch")]
pub fn render_markdown_section(resource: &GhResource, body: &GhBody) -> String {
    let cap = match resource {
        GhResource::Blob { .. } => BLOB_CAP,
        _ => ISSUE_PR_RELEASE_CAP,
    };
    let mut body_text = body.body.clone();
    truncate_body_in_place(&mut body_text, cap);

    let heading = format!(
        "## `gh://{}` — {}\n\n",
        resource.canonical_path(),
        body.title,
    );
    let mut metadata = String::from("*");
    if let Some(author) = &body.author {
        metadata.push_str(&format!("by {author}"));
        metadata.push_str(" · ");
    }
    metadata.push_str(&body.state);
    if let Some(when) = &body.published_at {
        metadata.push_str(&format!(" · {when}"));
    }
    metadata.push_str("*\n\n");

    format!("{heading}{metadata}{body_text}\n")
}

fn truncate_body_in_place(body: &mut String, cap: usize) {
    if body.len() <= cap {
        return;
    }
    let mut cut = cap.min(body.len());
    while !body.is_char_boundary(cut) {
        cut -= 1;
    }
    let original_len = body.len();
    body.truncate(cut);
    body.push_str(&format!("\n\n…[{} more chars]", original_len - cut));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_body_is_not_truncated() {
        let mut s = String::from("short");
        truncate_body_in_place(&mut s, 100);
        assert_eq!(s, "short");
    }

    #[test]
    fn long_body_is_truncated_with_marker() {
        let mut s = "x".repeat(5000);
        truncate_body_in_place(&mut s, 100);
        assert!(s.starts_with(&"x".repeat(100)));
        assert!(s.contains("more chars"));
    }

    #[test]
    fn truncation_respects_utf8_boundary() {
        let mut s = "—".repeat(50);
        truncate_body_in_place(&mut s, 100);
        assert!(s.is_char_boundary(s.find("\n\n…").unwrap()));
    }

    #[cfg(feature = "fetch")]
    #[test]
    fn render_markdown_section_includes_heading_metadata_body() {
        let r = GhResource::Issue {
            owner: "foo".into(),
            repo: "bar".into(),
            number: 7,
        };
        let body = GhBody {
            title: "test title".into(),
            state: "open".into(),
            author: Some("alice".into()),
            published_at: Some("2026-04-18T00:00:00Z".into()),
            body: "test body".into(),
        };
        let rendered = render_markdown_section(&r, &body);
        assert!(rendered.contains("gh:///foo/bar/issues/7"));
        assert!(rendered.contains("test title"));
        assert!(rendered.contains("by alice"));
        assert!(rendered.contains("open"));
        assert!(rendered.contains("test body"));
    }
}
