pub mod bundle_summary;
pub mod header;
pub mod prompt_input;
pub mod prompt_preview;
pub mod search_bar;
pub mod status_bar;
pub mod tree;
pub mod viewer;

/// Shrink a path to fit `max` chars while keeping it readable.
/// Strategy: full path if it fits, else `…/<filename>`, else truncate the
/// filename stem but preserve the extension.
pub fn smart_truncate_path(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let filename = s.rsplit('/').next().unwrap_or(s);
    let filename_chars: Vec<char> = filename.chars().collect();

    let with_marker_len = filename_chars.len() + 2;
    if with_marker_len <= max {
        return format!("…/{}", filename);
    }

    if let Some(dot_pos) = filename.rfind('.') {
        let (stem, ext) = filename.split_at(dot_pos);
        let stem_chars: Vec<char> = stem.chars().collect();
        let ext_len = ext.chars().count();
        let stem_budget = max.saturating_sub(1 + ext_len);
        if stem_budget > 0 {
            let trimmed_stem: String = stem_chars.iter().take(stem_budget).collect();
            return format!("{trimmed_stem}…{ext}");
        }
    }
    let head = max.saturating_sub(1);
    let head_str: String = filename_chars.iter().take(head).collect();
    format!("{head_str}…")
}
