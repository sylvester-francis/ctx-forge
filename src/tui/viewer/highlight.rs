//! Syntect wrapper that produces pre-styled iocraft `MixedTextContent` rows.

use iocraft::components::MixedTextContent;
use iocraft::components::TextDecoration;
use iocraft::{Color, Weight};
use syntect::easy::HighlightLines;
use syntect::highlighting::{FontStyle, Style as SyntectStyle, Theme, ThemeSet};
use syntect::parsing::SyntaxSet;

pub struct Highlighter {
    syntax_set: SyntaxSet,
    theme: Theme,
}

impl Highlighter {
    pub fn new() -> Self {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let theme_set = ThemeSet::load_defaults();
        let theme = theme_set.themes["base16-ocean.dark"].clone();
        Self { syntax_set, theme }
    }

    pub fn highlight(&self, extension: &str, content: &str) -> Vec<Vec<MixedTextContent>> {
        let syntax = self
            .syntax_set
            .find_syntax_by_extension(extension)
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());
        let mut hl = HighlightLines::new(syntax, &self.theme);

        let mut out = Vec::new();
        for line in content.split_inclusive('\n') {
            let trimmed = line.strip_suffix('\n').unwrap_or(line);
            let ranges = hl
                .highlight_line(trimmed, &self.syntax_set)
                .unwrap_or_default();
            let spans: Vec<MixedTextContent> = ranges
                .into_iter()
                .map(|(style, text)| syntect_to_content(style, text))
                .collect();
            if spans.is_empty() {
                out.push(vec![MixedTextContent::new(trimmed.to_string())]);
            } else {
                out.push(spans);
            }
        }
        out
    }
}

impl Default for Highlighter {
    fn default() -> Self {
        Self::new()
    }
}

/// First call pays ~200ms of SyntaxSet deserialization; warm at startup
/// so the first viewer toggle doesn't stall the UI.
pub fn shared() -> &'static Highlighter {
    static INSTANCE: std::sync::LazyLock<Highlighter> = std::sync::LazyLock::new(Highlighter::new);
    &INSTANCE
}

fn syntect_to_content(s: SyntectStyle, text: &str) -> MixedTextContent {
    let mut c = MixedTextContent::new(text.to_string()).color(Color::Rgb {
        r: s.foreground.r,
        g: s.foreground.g,
        b: s.foreground.b,
    });
    if s.font_style.contains(FontStyle::BOLD) {
        c = c.weight(Weight::Bold);
    }
    if s.font_style.contains(FontStyle::ITALIC) {
        c = c.italic();
    }
    if s.font_style.contains(FontStyle::UNDERLINE) {
        c = c.decoration(TextDecoration::Underline);
    }
    c
}
