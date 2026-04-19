//! Viewer pane renderer — line-numbered, syntect-highlighted code with
//! scroll, mouse wheel, and drag-to-select a range of lines.
//!
//! `use_local_terminal_events` gives mouse coords relative to the
//! component; `use_component_rect` gives the rendered size (from the
//! previous frame) so we only translate clicks inside the viewport.

use crate::tui::theme::Theme;
use crate::tui::viewer::ViewerState;
use iocraft::prelude::*;
use iocraft::hooks::{UseComponentRect, UseTerminalEvents};

#[derive(Clone, Debug, PartialEq)]
pub enum ViewerMouseEvent {
    ScrollUp,
    ScrollDown,
    Down { line: usize },
    Drag { line: usize },
    Up,
}

/// Clone-only snapshot of viewer state. Separate from `ViewerState` so
/// the render component isn't coupled to the non-Clone highlighter field.
#[derive(Default, Clone)]
pub struct ViewerStateSnapshot {
    pub scroll: usize,
    pub lines: Vec<Vec<MixedTextContent>>,
    pub error_msg: Option<String>,
    pub truncated: bool,
    pub selection: Option<(usize, usize)>,
    pub loading: bool,
}

impl ViewerStateSnapshot {
    pub fn from_state(v: &ViewerState) -> Self {
        Self {
            scroll: v.scroll,
            lines: v.lines.clone(),
            error_msg: v.error.as_ref().map(|e| e.to_string()),
            truncated: v.truncated,
            selection: v.selection,
            loading: v.loading,
        }
    }
}

#[derive(Default, Props)]
pub struct ViewerProps {
    pub viewer: ViewerStateSnapshot,
    pub theme: Option<Theme>,
    /// Mouse-event queue. `State<Vec<_>>` (not Arc<Mutex<_>>) so iocraft
    /// re-renders on every event — critical for fluid drag-select.
    pub events: Option<State<Vec<ViewerMouseEvent>>>,
}

#[component]
pub fn Viewer(hooks: &mut Hooks, props: &ViewerProps) -> impl Into<AnyElement<'static>> {
    let scroll = props.viewer.scroll;
    let events = props.events;
    let total_lines = props.viewer.lines.len();

    // `None` on first frame — fall back to a default so the initial
    // render doesn't collapse to 0 rows.
    let rect = hooks.use_component_rect();
    let rendered_height = rect
        .map(|r| (r.bottom - r.top).max(0) as usize)
        .unwrap_or(40);

    // Cap at both the viewport height and lines remaining after scroll,
    // so clicks on blank rows past EOF don't produce phantom line indices.
    let rendered_rows = total_lines.saturating_sub(scroll).min(rendered_height);

    hooks.use_local_terminal_events(move |event| {
        let Some(mut events) = events else { return };
        if let TerminalEvent::FullscreenMouse(m) = event {
            use crossterm::event::{MouseButton, MouseEventKind};
            // Clicks past `rendered_rows` hit blank space and must be
            // ignored so we don't select invisible lines.
            let local_row = m.row as usize;
            let line_idx = local_row + scroll;
            let in_content = local_row < rendered_rows;
            let out = match m.kind {
                MouseEventKind::ScrollUp => Some(ViewerMouseEvent::ScrollUp),
                MouseEventKind::ScrollDown => Some(ViewerMouseEvent::ScrollDown),
                MouseEventKind::Down(MouseButton::Left) if in_content => {
                    Some(ViewerMouseEvent::Down { line: line_idx })
                }
                MouseEventKind::Drag(MouseButton::Left) if in_content => {
                    Some(ViewerMouseEvent::Drag { line: line_idx })
                }
                // Drag past EOF: extend to the last visible line so the
                // selection tracks the user's intent.
                MouseEventKind::Drag(MouseButton::Left) if rendered_rows > 0 => {
                    Some(ViewerMouseEvent::Drag {
                        line: scroll + rendered_rows - 1,
                    })
                }
                MouseEventKind::Up(MouseButton::Left) => Some(ViewerMouseEvent::Up),
                _ => None,
            };
            if let Some(ev) = out {
                events.write().push(ev);
            }
        }
    });

    let theme = props
        .theme
        .unwrap_or_else(|| Theme::from_app_theme(crate::theme::registry::default_theme()));
    let rows = render_rows(&props.viewer, &theme, rendered_height);

    element! {
        View(
            flex_direction: FlexDirection::Column,
            width: 100pct,
            flex_grow: 1.0,
        ) {
            #(rows)
        }
    }
}

fn render_rows(
    viewer: &ViewerStateSnapshot,
    theme: &Theme,
    viewport: usize,
) -> Vec<AnyElement<'static>> {
    if viewer.loading {
        return vec![
            element! {
                Text(content: "  ⣾ loading…", color: theme.muted, weight: Weight::Bold)
            }
            .into_any(),
        ];
    }

    if let Some(err) = &viewer.error_msg {
        let line = format!(" ⚠  {err}");
        return vec![
            element! {
                Text(content: line.leak() as &str, color: theme.danger, weight: Weight::Bold)
            }
            .into_any(),
        ];
    }

    if viewer.lines.is_empty() {
        return vec![
            element! {
                Text(content: "  empty file", color: theme.muted, weight: Weight::Light)
            }
            .into_any(),
        ];
    }

    // Clamp to a floor (so we still render something if the hook returns
    // 0) and to the real total (so we never render past EOF).
    let total = viewer.lines.len();
    let start = viewer.scroll;
    let end = (start + viewport.max(1)).min(total);

    let gutter_width = format!("{}", total).len();
    let mut rows: Vec<AnyElement<'static>> = Vec::new();

    for (i, spans) in viewer
        .lines
        .iter()
        .enumerate()
        .skip(start)
        .take(end - start)
    {
        let in_selection = viewer
            .selection
            .map(|(a, b)| i >= a && i <= b)
            .unwrap_or(false);
        let line_no = format!("{:>width$} │ ", i + 1, width = gutter_width);
        let mut row_spans = vec![MixedTextContent::new(line_no).color(theme.muted)];
        for span in spans {
            row_spans.push(span.clone());
        }
        let bg = if in_selection {
            Some(theme.drag_selection_bg)
        } else {
            None
        };
        rows.push(
            element! {
                View(background_color: bg, width: 100pct) {
                    MixedText(contents: row_spans)
                }
            }
            .into_any(),
        );
    }

    if viewer.truncated {
        rows.push(
            element! {
                Text(content: "  …truncated at 2 MB", color: theme.warning, weight: Weight::Light)
            }
            .into_any(),
        );
    }

    rows
}
