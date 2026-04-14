//! Syntect wrapper that produces pre-styled ratatui `Line<'static>` values.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use syntect::easy::HighlightLines;
use syntect::highlighting::{FontStyle, Style as SyntectStyle, Theme, ThemeSet};
use syntect::parsing::SyntaxSet;

/// Owns the bundled syntect syntaxes and theme. Not `Clone` (syntect's
/// SyntaxSet isn't either) — keep a single instance on `App`.
pub struct Highlighter {
    syntax_set: SyntaxSet,
    theme: Theme,
}

impl Highlighter {
    pub fn new() -> Self {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let theme_set = ThemeSet::load_defaults();
        // base16-ocean.dark is bundled in the default ThemeSet and reads well
        // on the dark terminal background ctxforge targets.
        let theme = theme_set.themes["base16-ocean.dark"].clone();
        Self { syntax_set, theme }
    }

    /// Highlight `content` as the given file extension (e.g. `"rs"`, `"py"`).
    /// Falls back to plain-text rules when the extension is unknown.
    /// Returns one `Line` per newline-terminated line in the input.
    pub fn highlight(&self, extension: &str, content: &str) -> Vec<Line<'static>> {
        let syntax = self
            .syntax_set
            .find_syntax_by_extension(extension)
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());
        let mut highlighter = HighlightLines::new(syntax, &self.theme);

        let mut out = Vec::new();
        for line in content.split_inclusive('\n') {
            // Strip trailing newline: ratatui inserts line breaks itself;
            // keeping `\n` in a span prints a visible box.
            let trimmed = line.strip_suffix('\n').unwrap_or(line);
            let ranges = highlighter
                .highlight_line(trimmed, &self.syntax_set)
                .unwrap_or_default();
            let spans: Vec<Span<'static>> = ranges
                .into_iter()
                .map(|(style, text)| Span::styled(text.to_string(), syntect_to_ratatui(style)))
                .collect();
            out.push(Line::from(if spans.is_empty() {
                vec![Span::raw(trimmed.to_string())]
            } else {
                spans
            }));
        }
        out
    }
}

impl Default for Highlighter {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert a syntect `Style` to a ratatui `Style`. Foreground (syntect's
/// primary signal) maps to RGB; background is dropped (terminals own it).
/// Font-style bits pass through.
fn syntect_to_ratatui(s: SyntectStyle) -> Style {
    let mut style = Style::default().fg(Color::Rgb(s.foreground.r, s.foreground.g, s.foreground.b));
    if s.font_style.contains(FontStyle::BOLD) {
        style = style.add_modifier(Modifier::BOLD);
    }
    if s.font_style.contains(FontStyle::ITALIC) {
        style = style.add_modifier(Modifier::ITALIC);
    }
    if s.font_style.contains(FontStyle::UNDERLINE) {
        style = style.add_modifier(Modifier::UNDERLINED);
    }
    style
}
