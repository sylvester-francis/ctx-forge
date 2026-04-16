//! Scenario picker — stub (populated in next task).

use crate::tui::scenario::{self, Scenario};
use crate::tui2::theme::Theme;
use iocraft::prelude::*;

pub fn load(root: &crate::paths::CtxforgeRoot) -> Vec<Scenario> {
    scenario::available(root)
}

pub fn render_body(
    _scenarios: &[Scenario],
    _cursor: usize,
    _current: Option<&str>,
    _theme: &Theme,
) -> Vec<AnyElement<'static>> {
    Vec::new()
}
