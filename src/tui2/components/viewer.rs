//! Viewer pane renderer — line-numbered, syntect-highlighted code with
//! scroll, mouse wheel, and drag-to-select a range of lines.

use crate::tui2::theme::Theme;
use crate::tui2::viewer::ViewerState;
use iocraft::prelude::*;

// `use_local_terminal_events` (method on UseTerminalEvents trait) gives
// mouse coords relative to the component — (0, 0) is the top-left of the
// Viewer's rendered area. Since Viewer's root View has no border/padding,
// y=0 = first line of the rendered code. Events fired outside the
// viewer bounds don't trigger the callback.
use iocraft::hooks::UseTerminalEvents;

#[derive(Clone, Debug, PartialEq)]
pub enum ViewerMouseEvent {
    ScrollUp,
    ScrollDown,
    Down { line: usize },
    Drag { line: usize },
    Up,
}

/// Cheap clone-only snapshot of viewer state needed to render. Separating
/// this from `ViewerState` avoids coupling the render component to the
/// highlighter field (which isn't Clone).
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
    /// Queue the mouse callback pushes into. Using `State<Vec<_>>` instead
    /// of a plain Arc<Mutex<..>> so iocraft re-renders on every event —
    /// critical for fluid drag-select and scroll.
    pub events: Option<State<Vec<ViewerMouseEvent>>>,
}

#[component]
pub fn Viewer(hooks: &mut Hooks, props: &ViewerProps) -> impl Into<AnyElement<'static>> {
    let scroll = props.viewer.scroll;
    let events = props.events;

    hooks.use_local_terminal_events(move |event| {
        let Some(mut events) = events else { return };
        if let TerminalEvent::FullscreenMouse(m) = event {
            use crossterm::event::{MouseButton, MouseEventKind};
            // Component-local coords: m.row = 0 is the viewer's first
            // rendered line. Add scroll to get the absolute line index.
            let line_idx = m.row as usize + scroll;
            let out = match m.kind {
                MouseEventKind::ScrollUp => Some(ViewerMouseEvent::ScrollUp),
                MouseEventKind::ScrollDown => Some(ViewerMouseEvent::ScrollDown),
                MouseEventKind::Down(MouseButton::Left) => {
                    Some(ViewerMouseEvent::Down { line: line_idx })
                }
                MouseEventKind::Drag(MouseButton::Left) => {
                    Some(ViewerMouseEvent::Drag { line: line_idx })
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
        .unwrap_or_else(|| Theme::from_app_theme(crate::tui::theme::registry::default_theme()));
    let rows = render_rows(&props.viewer, &theme);

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

fn render_rows(viewer: &ViewerStateSnapshot, theme: &Theme) -> Vec<AnyElement<'static>> {
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

    const VIEWPORT: usize = 48;
    let total = viewer.lines.len();
    let start = viewer.scroll;
    let end = (start + VIEWPORT).min(total);

    let gutter_width = format!("{}", total).len();
    let mut rows: Vec<AnyElement<'static>> = Vec::new();

    for (i, spans) in viewer.lines.iter().enumerate().skip(start).take(end - start) {
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
