//! Format dispatch: renders a list of resolved items into a string.

#![allow(dead_code)]

pub mod markdown;

use crate::memory::Note;
use crate::resolve::ResolvedItem;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Format {
    #[default]
    Markdown,
    // Xml and Json added in Plan 3.
}

pub fn render(format: Format, items: &[ResolvedItem], memory: &[Note]) -> String {
    match format {
        Format::Markdown => markdown::render(items, memory),
    }
}
