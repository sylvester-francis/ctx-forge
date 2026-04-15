//! Render the TUI layout.

use crate::tui::app::{App, Focus};
use crate::tui::theme;
use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Gauge, List, ListItem, ListState, Paragraph, Widget};

pub fn draw(f: &mut Frame, app: &App) {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header + gauge
            Constraint::Min(5),    // main panels
            Constraint::Length(2), // status + keybindings
        ])
        .split(area);

    // Pass 1 — Normal content into the frame buffer.
    draw_header(f, app, chunks[0]);

    // Layout selection. Viewer opt-in + width-adaptive:
    //   width ≥ 140, height ≥ 30, viewer on  → three-column horizontal
    //   width ≥ 100, viewer on                → vertical stack (bundle/viewer/tree)
    //   width < 100 OR viewer off              → existing two-panel layouts
    let want_viewer = app.viewer.enabled && area.width >= 100;
    let wide_three_col = want_viewer && area.width >= 140 && area.height >= 30;

    if wide_three_col {
        let panel_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25),
                Constraint::Percentage(45),
                Constraint::Percentage(30),
            ])
            .split(chunks[1]);
        let left_handled = draw_left_panel(f, app, panel_chunks[0]);
        if !left_handled {
            draw_file_tree(f, app, panel_chunks[0]);
        }
        draw_viewer(f, app, panel_chunks[1]);
        draw_right_panel(f, app, panel_chunks[2]);
    } else if want_viewer {
        let panel_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(34),
                Constraint::Percentage(33),
                Constraint::Percentage(33),
            ])
            .split(chunks[1]);
        draw_right_panel(f, app, panel_chunks[0]);
        draw_viewer(f, app, panel_chunks[1]);
        let left_handled = draw_left_panel(f, app, panel_chunks[2]);
        if !left_handled {
            draw_file_tree(f, app, panel_chunks[2]);
        }
    } else if area.width >= 120 && area.height >= 30 {
        let panel_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(chunks[1]);
        let left_handled = draw_left_panel(f, app, panel_chunks[0]);
        if !left_handled {
            draw_file_tree(f, app, panel_chunks[0]);
        }
        draw_right_panel(f, app, panel_chunks[1]);
    } else {
        let panel_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[1]);
        draw_right_panel(f, app, panel_chunks[0]);
        let left_handled = draw_left_panel(f, app, panel_chunks[1]);
        if !left_handled {
            draw_file_tree(f, app, panel_chunks[1]);
        }
    }

    draw_footer(f, app, chunks[2]);

    // Pass 2 — dim the Normal content when an overlay is visible.
    let dim = app.backdrop_dim.opacity(app.clock.now());
    if dim > 0.001 {
        apply_dim_to_buffer(f.buffer_mut(), dim, app.theme.bg);
    }

    // Pass 3a — outgoing overlay (during cross-fade).
    let outgoing_opacity = app.outgoing_overlay_opacity();
    if outgoing_opacity > 0.01 {
        if let Some(prev_mode) = app.outgoing_overlay_mode() {
            let prev = prev_mode.clone();
            render_overlay_blended(f, app, &prev, outgoing_opacity);
        }
    }

    // Pass 3b — incoming (or currently-visible) overlay.
    if app.mode().is_overlay() || app.show_help {
        let opacity = if app.show_help && matches!(app.mode(), crate::tui::mode::Mode::Normal) {
            1.0 // Legacy show_help flag without mode transition — full opacity.
        } else {
            app.incoming_overlay_opacity()
        };
        if opacity > 0.01 {
            let current = app.mode().clone();
            render_overlay_blended(f, app, &current, opacity);
        }
    }

    // Pass 4 — startup fade. Applied to the entire frame buffer so the whole
    // TUI eases in on launch.
    let startup_opacity = app.startup_fade.opacity(app.clock.now());
    if startup_opacity < 0.999 {
        apply_opacity_to_buffer(f.buffer_mut(), startup_opacity, app.theme.bg);
    }
}

/// Fade every cell toward the theme bg by `1 - opacity`. Used by the
/// startup fade-in to ease the whole TUI into view.
fn apply_opacity_to_buffer(buf: &mut Buffer, opacity: f32, bg: ratatui::style::Color) {
    use crate::tui::motion::blend;
    let area = buf.area;
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            let cell = &mut buf[(x, y)];
            cell.fg = blend(opacity, cell.fg, bg);
            cell.bg = blend(opacity, cell.bg, bg);
        }
    }
}

/// Render an overlay for `mode` into a scratch buffer, then blend it onto the
/// frame's buffer with `opacity`. At 0.0 the overlay is invisible (frame
/// untouched); at 1.0 cells are written as-is. Empty (unset) scratch cells
/// are skipped so only the overlay's own rect is affected.
fn render_overlay_blended(f: &mut Frame, app: &App, mode: &crate::tui::mode::Mode, opacity: f32) {
    use crate::tui::motion::blend;
    use ratatui::style::Color;

    let area = f.area();
    let mut scratch = Buffer::empty(area);
    draw_overlay_into_buffer(&mut scratch, area, app, mode);

    let dst = f.buffer_mut();
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            let s = scratch[(x, y)].clone();
            // Treat cells the overlay never touched as "transparent": they keep
            // the default symbol (" ") and default style. A cell with a
            // non-default bg was explicitly written (Clear or a widget fill).
            let wrote_symbol = s.symbol() != " ";
            let wrote_bg = !matches!(s.bg, Color::Reset);
            let wrote_fg = !matches!(s.fg, Color::Reset);
            if !(wrote_symbol || wrote_bg || wrote_fg) {
                continue;
            }
            let d = &mut dst[(x, y)];
            let new_fg = blend(opacity, s.fg, d.fg);
            let new_bg = blend(opacity, s.bg, d.bg);
            if opacity >= 0.5 {
                d.set_symbol(s.symbol());
            }
            d.fg = new_fg;
            d.bg = new_bg;
        }
    }
}

/// Dispatch: render the given mode's overlay into `buf`. Mirrors the legacy
/// `draw_overlays` function but writes to a caller-supplied buffer so we can
/// blend the result during cross-fades.
fn draw_overlay_into_buffer(
    buf: &mut Buffer,
    frame_area: Rect,
    app: &App,
    mode: &crate::tui::mode::Mode,
) {
    use crate::tui::mode::Mode;

    if matches!(mode, Mode::CommandPalette { .. }) {
        draw_command_palette_buf(buf, frame_area, app, mode);
    }

    if matches!(mode, Mode::Help) {
        draw_help_overlay_buf(buf, frame_area, app.theme);
    }

    match mode {
        Mode::PipeMenu => {
            let area = centered_rect(30, 7, frame_area);
            ratatui::widgets::Clear.render(area, buf);
            let block = Block::default()
                .title(" Pipe to Agent ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(app.theme.border_focused));
            let text = vec![
                Line::from("  c  claude (XML)"),
                Line::from("  a  agent (markdown)"),
                Line::from("  g  gemini (markdown)"),
                Line::from(""),
                Line::from("  Esc cancel"),
            ];
            Paragraph::new(text).block(block).render(area, buf);
        }
        Mode::ModelSwitch { cursor } => {
            let models = crate::models::all_models();
            let height = (models.len() + 2).min(20) as u16;
            let area = centered_rect(45, height, frame_area);
            ratatui::widgets::Clear.render(area, buf);
            let block = Block::default()
                .title(" Switch Model ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(app.theme.border_focused));
            let items: Vec<ListItem> = models
                .iter()
                .enumerate()
                .map(|(i, m)| {
                    let style = if i == *cursor {
                        Style::default()
                            .fg(app.theme.selected_fg)
                            .bg(app.theme.selected_bg)
                    } else {
                        Style::default()
                    };
                    ListItem::new(Span::styled(
                        format!("  {:25} {:>10}", m.name, m.window),
                        style,
                    ))
                })
                .collect();
            List::new(items).block(block).render(area, buf);
        }
        Mode::TemplatePick { cursor, templates } => {
            draw_template_pick_overlay_buf(buf, frame_area, *cursor, templates, app.theme);
        }
        Mode::TemplateTask {
            template_name,
            task,
        } => {
            draw_template_task_overlay_buf(buf, frame_area, template_name, task, app.theme);
        }
        _ => {}
    }
}

/// Blend every cell's fg and bg toward `bg` by `dim`, leaving symbols intact.
/// Used to darken Normal content behind an overlay.
fn apply_dim_to_buffer(buf: &mut ratatui::buffer::Buffer, dim: f32, bg: ratatui::style::Color) {
    use crate::tui::motion::blend;
    let area = buf.area;
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            let cell = &mut buf[(x, y)];
            cell.fg = blend(1.0 - dim, cell.fg, bg);
            cell.bg = blend(1.0 - dim, cell.bg, bg);
        }
    }
}

// Legacy dispatcher kept only as a compile-time no-op placeholder — the real
// rendering now goes through `draw_overlay_into_buffer`. Left here so other
// callers inside this file (none currently) don't break silently.

fn draw_command_palette_buf(
    buf: &mut Buffer,
    frame_area: Rect,
    app: &App,
    mode: &crate::tui::mode::Mode,
) {
    use crate::tui::mode::Mode;
    let (query, cursor) = match mode {
        Mode::CommandPalette { query, cursor } => (query.as_str(), *cursor),
        _ => return,
    };
    let results = crate::tui::commands::fuzzy_filter(query);
    let area = centered_rect(70, 16, frame_area);
    ratatui::widgets::Clear.render(area, buf);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(3),
        ])
        .split(area);

    let input_block = Block::default()
        .title(" / command palette ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.border_focused));
    Paragraph::new(format!(" /{query}"))
        .block(input_block)
        .render(chunks[0], buf);

    let items: Vec<ListItem> = results
        .iter()
        .enumerate()
        .map(|(i, cmd)| {
            let marker = if i == cursor { " > " } else { "   " };
            let style = if i == cursor {
                Style::default()
                    .fg(app.theme.selected_fg)
                    .bg(app.theme.selected_bg)
            } else {
                Style::default()
            };
            ListItem::new(Span::styled(
                format!("{marker}/{:<14} {}", cmd.name, cmd.description),
                style,
            ))
        })
        .collect();
    List::new(items)
        .block(Block::default().borders(Borders::ALL))
        .render(chunks[1], buf);

    let footer_text = format!(
        " {} of {} matches  |  Enter run  |  Esc cancel ",
        results.len(),
        crate::tui::commands::COMMANDS.len()
    );
    Paragraph::new(footer_text)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().add_modifier(Modifier::DIM))
        .render(chunks[2], buf);
}

fn draw_help_overlay_buf(buf: &mut Buffer, frame_area: Rect, theme: &crate::tui::theme::AppTheme) {
    let area = centered_rect(80, 22, frame_area);
    ratatui::widgets::Clear.render(area, buf);
    let block = Block::default()
        .title(" ctxforge -- navigation keys ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border_focused));
    let lines = vec![
        Line::from(""),
        Line::from("  Movement                          Selection"),
        Line::from("    j / k       down / up             space  toggle file (in tree)"),
        Line::from("    g / G       top / bottom          Enter  expand directory"),
        Line::from("    PgDn / PgUp page down / up        E / C  expand / collapse all"),
        Line::from("    Ctrl+D/U    half-page down / up   Tab    switch panel"),
        Line::from(""),
        Line::from("  Discoverable input"),
        Line::from("    /         open command palette  (every feature lives here)"),
        Line::from("    Ctrl+F    file fuzzy search     (alias for /find)"),
        Line::from("    v         toggle code viewer"),
        Line::from("    ?         this help overlay"),
        Line::from("    q         quit"),
        Line::from(""),
        Line::from("  Most-used commands (preview -- full list in / palette)"),
        Line::from("    /copy        copy bundle to clipboard"),
        Line::from("    /save        save current as profile"),
        Line::from("    /load        load a profile"),
        Line::from("    /pipe        pipe to agent (claude / agent / gemini)"),
        Line::from("    /narrow      narrow item to line range"),
        Line::from("    /template    pick template + task -> copy"),
        Line::from(""),
        Line::from("                          Esc or ? to close"),
    ];
    Paragraph::new(lines).block(block).render(area, buf);
}

fn draw_template_pick_overlay_buf(
    buf: &mut Buffer,
    frame_area: Rect,
    cursor: usize,
    templates: &[(String, crate::tui::mode::TemplateSource)],
    theme: &crate::tui::theme::AppTheme,
) {
    use crate::tui::mode::TemplateSource;
    let area = centered_rect(60, (templates.len() + 4).min(20) as u16, frame_area);
    ratatui::widgets::Clear.render(area, buf);
    let block = Block::default()
        .title(" / template -- pick a template ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border_focused));
    let items: Vec<ListItem> = templates
        .iter()
        .enumerate()
        .map(|(i, (name, source))| {
            let source_label = match source {
                TemplateSource::Project => "project",
                TemplateSource::Global => "global",
            };
            let marker = if i == cursor { " > " } else { "   " };
            let style = if i == cursor {
                Style::default().fg(theme.selected_fg).bg(theme.selected_bg)
            } else {
                Style::default()
            };
            ListItem::new(Span::styled(
                format!("{marker}{:<20} ({source_label})", name),
                style,
            ))
        })
        .collect();
    List::new(items).block(block).render(area, buf);
}

fn draw_template_task_overlay_buf(
    buf: &mut Buffer,
    frame_area: Rect,
    template_name: &str,
    task: &str,
    theme: &crate::tui::theme::AppTheme,
) {
    let area = centered_rect(70, 7, frame_area);
    ratatui::widgets::Clear.render(area, buf);
    let block = Block::default()
        .title(format!(" task for template '{template_name}' "))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border_focused));
    let text = vec![
        Line::from(""),
        Line::from(format!("  > {task}")),
        Line::from(""),
        Line::from("  Enter to copy with template  |  Esc to cancel"),
    ];
    Paragraph::new(text).block(block).render(area, buf);
}

/// Render the right panel (bundle list, profile list, or memory panel).
fn draw_right_panel(f: &mut Frame, app: &App, area: Rect) {
    // LoadProfile mode replaces the right panel with the profile picker.
    if let crate::tui::mode::Mode::LoadProfile { cursor, profiles } = app.mode() {
        draw_profile_list(f, *cursor, profiles, area, app.theme);
        return;
    }

    // MemoryPanel mode replaces the right panel with the recall view.
    if matches!(app.mode(), crate::tui::mode::Mode::MemoryPanel { .. }) {
        draw_memory_panel(f, app, area);
        return;
    }

    draw_bundle_list(f, app, area);
}

/// Render the code viewer pane.
fn draw_viewer(f: &mut Frame, app: &App, area: Rect) {
    let focused = app.focus == Focus::Viewer;
    let border_color = if focused {
        app.focus_highlight.current(app.clock.now())
    } else {
        app.theme.border
    };

    // Compute body viewport (inside borders).
    let inner_height = (area.height as usize).saturating_sub(2);
    let truncated_footer = app.viewer.truncated();
    let body_height = if truncated_footer {
        inner_height.saturating_sub(1)
    } else {
        inner_height
    };
    // Remember for key-handler scroll clamping and mouse hit-testing.
    app.set_viewer_viewport_height(body_height);
    app.set_viewer_pane_rect(area);

    let title = build_viewer_title(app, body_height, area.width);
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    // Error / empty states take precedence over content.
    if let Some(err) = app.viewer.error() {
        let msg = format!("  {err}");
        let style = match err {
            crate::tui::viewer::ViewerError::Directory
            | crate::tui::viewer::ViewerError::Binary(_) => {
                Style::default().add_modifier(Modifier::DIM)
            }
            _ => Style::default().fg(app.theme.danger),
        };
        let p = Paragraph::new(Line::styled(msg, style)).block(block);
        f.render_widget(p, area);
        return;
    }
    if app.viewer.lines().is_empty() {
        f.render_widget(block, area);
        return;
    }

    let total = app.viewer.lines().len();
    let top = app.viewer.scroll;
    let bottom = (top + body_height).min(total);

    // Gutter width: enough digits for the largest visible line number + 1
    // padding space. `nnn │ ` — the vertical bar is the divider.
    let max_line_no = bottom.max(1);
    let digits = max_line_no.to_string().len();
    let gutter_style = Style::default()
        .fg(app.theme.muted)
        .add_modifier(Modifier::DIM);
    let divider_style = Style::default().fg(app.theme.muted);

    let selection = app.viewer.selection();
    let selection_bg = app.theme.drag_selection_bg;
    let slice: Vec<Line<'static>> = app.viewer.lines()[top..bottom]
        .iter()
        .enumerate()
        .map(|(i, line)| {
            let line_no = top + i + 1;
            let absolute = top + i;
            let selected = selection
                .map(|(a, b)| absolute >= a && absolute <= b)
                .unwrap_or(false);

            let gutter = format!("{line_no:>width$} ", width = digits);
            let divider = "│ ".to_string();
            let row_bg = if selected { Some(selection_bg) } else { None };
            let apply_bg = |style: Style| -> Style {
                if let Some(bg) = row_bg {
                    style.bg(bg)
                } else {
                    style
                }
            };
            let mut spans: Vec<Span<'static>> = vec![
                Span::styled(gutter, apply_bg(gutter_style)),
                Span::styled(divider, apply_bg(divider_style)),
            ];
            for span in &line.spans {
                let styled = apply_bg(span.style);
                spans.push(Span::styled(span.content.clone().into_owned(), styled));
            }
            Line::from(spans)
        })
        .collect();
    let p = Paragraph::new(slice).block(block);
    f.render_widget(p, area);

    if truncated_footer && area.height >= 3 {
        let footer_area = Rect {
            x: area.x + 1,
            y: area.y + area.height.saturating_sub(2),
            width: area.width.saturating_sub(2),
            height: 1,
        };
        let footer = Paragraph::new(Line::styled(
            "  … truncated at 2 MB",
            Style::default().add_modifier(Modifier::DIM),
        ));
        f.render_widget(footer, footer_area);
    }
}

/// Build the viewer title: `" preview — path  [line N-M / total] "`.
fn build_viewer_title(app: &App, body_height: usize, width: u16) -> String {
    let path_display = app
        .viewer
        .cached_path
        .as_ref()
        .map(|p| {
            p.strip_prefix(&app.project_root)
                .unwrap_or(p)
                .display()
                .to_string()
        })
        .unwrap_or_else(|| "—".into());
    let total = app.viewer.lines().len();
    let top = app.viewer.scroll;
    let bottom = (top + body_height).min(total);
    let indicator = if total > 0 {
        format!(" [line {}-{} / {}] ", top + 1, bottom, total)
    } else {
        String::new()
    };
    let prefix = format!(" preview — {path_display}{indicator}");
    if prefix.len() > width as usize {
        " preview ".into()
    } else {
        prefix
    }
}

/// Helper to create a centered rect of given width/height inside `area`,
/// clamped to fit if `area` is too small.
fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}

fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    // Text values use the real total (numbers update instantly).
    let pct = app.window_pct();
    let color = theme::gauge_color(pct);

    let token_str = if app.exact_tokens {
        format!("{}", app.total_tokens)
    } else {
        format!("~{}", app.total_tokens)
    };

    let profile_str = app
        .profile_name
        .as_deref()
        .map(|n| format!("profile: {n} │ "))
        .unwrap_or_default();

    let label = format!(
        " ctxforge │ {}{} │ {} / {} ({:.1}%)",
        profile_str, app.model_name, token_str, app.model_window, pct
    );

    // The bar fill tweens smoothly via `token_gauge`.
    let animated_tokens = app.token_gauge.current(app.clock.now()) as f64;
    let animated_ratio = if app.model_window == 0 {
        0.0
    } else {
        (animated_tokens / app.model_window as f64).clamp(0.0, 1.0)
    };

    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL))
        .gauge_style(Style::default().fg(color))
        .label(label)
        .ratio(animated_ratio);

    f.render_widget(gauge, area);
}

/// Returns `true` if a picker overlay was rendered into the left panel,
/// in which case the caller should NOT also render the file tree.
fn draw_left_panel(f: &mut Frame, app: &App, area: Rect) -> bool {
    use crate::tui::mode::Mode;
    #[cfg(feature = "extract")]
    {
        if let Mode::FunctionPick { cursor, items } = app.mode() {
            draw_symbol_pick(f, "Functions", "λ", *cursor, items, area, app.theme);
            return true;
        }
        if let Mode::TypePick { cursor, items } = app.mode() {
            draw_symbol_pick(f, "Types", "τ", *cursor, items, area, app.theme);
            return true;
        }
    }
    if let Mode::DiffPick {
        files,
        selected,
        cursor,
        entering_branch,
        ..
    } = app.mode()
    {
        if !*entering_branch {
            draw_diff_pick(f, *cursor, files, selected, area, app.theme);
            return true;
        }
        // entering_branch=true: tree stays visible; the input goes in the footer.
    }
    false
}

#[cfg(feature = "extract")]
fn draw_symbol_pick(
    f: &mut Frame,
    title: &str,
    icon: &str,
    cursor: usize,
    items: &[(String, std::path::PathBuf)],
    area: Rect,
    theme: &crate::tui::theme::AppTheme,
) {
    let block = Block::default()
        .title(format!(" {title} ({}) ", items.len()))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border_focused));
    let list_items: Vec<ListItem> = items
        .iter()
        .enumerate()
        .map(|(i, (name, path))| {
            let style = if i == cursor {
                Style::default().fg(theme.selected_fg).bg(theme.selected_bg)
            } else {
                Style::default()
            };
            ListItem::new(Span::styled(
                format!("  {icon} {name}  {}", path.display()),
                style,
            ))
        })
        .collect();
    f.render_widget(List::new(list_items).block(block), area);
}

fn draw_diff_pick(
    f: &mut Frame,
    cursor: usize,
    files: &[std::path::PathBuf],
    selected: &std::collections::HashSet<usize>,
    area: Rect,
    theme: &crate::tui::theme::AppTheme,
) {
    let block = Block::default()
        .title(format!(" Changed Files ({}) ", files.len()))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border_focused));
    let items: Vec<ListItem> = files
        .iter()
        .enumerate()
        .map(|(i, path)| {
            let marker = if selected.contains(&i) { "■" } else { "▫" };
            let style = if i == cursor {
                Style::default().fg(theme.selected_fg).bg(theme.selected_bg)
            } else {
                Style::default()
            };
            ListItem::new(Span::styled(
                format!("  {marker} {}", path.display()),
                style,
            ))
        })
        .collect();
    f.render_widget(List::new(items).block(block), area);
}

fn draw_memory_panel(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(" Memory (recall) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.border_focused));

    let mut notes = crate::memory::index::read_all(&app.root).unwrap_or_default();
    // Show newest first to match recall semantics.
    notes.reverse();

    let cursor = if let crate::tui::mode::Mode::MemoryPanel { cursor, .. } = app.mode() {
        *cursor
    } else {
        0
    };

    let items: Vec<ListItem> = notes
        .iter()
        .enumerate()
        .map(|(i, note)| {
            let ts = note.timestamp.format("%Y-%m-%d");
            let tag_str = note
                .tag
                .as_deref()
                .map(|t| format!(" {t}"))
                .unwrap_or_default();
            let style = if i == cursor {
                Style::default()
                    .fg(app.theme.selected_fg)
                    .bg(app.theme.selected_bg)
            } else {
                Style::default()
            };
            ListItem::new(Span::styled(
                format!("  [{ts}{tag_str}] {}", note.body),
                style,
            ))
        })
        .collect();

    if items.is_empty() {
        let msg = Paragraph::new("  (no notes yet)")
            .style(Style::default().add_modifier(Modifier::DIM))
            .block(block);
        f.render_widget(msg, area);
    } else {
        let list = List::new(items).block(block);
        let mut state = ListState::default();
        let len = notes.len();
        state.select(Some(cursor.min(len.saturating_sub(1))));
        f.render_stateful_widget(list, area, &mut state);
    }
}

fn draw_profile_list(
    f: &mut Frame,
    cursor: usize,
    profiles: &[String],
    area: Rect,
    theme: &crate::tui::theme::AppTheme,
) {
    let block = Block::default()
        .title(" Load Profile ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border_focused));

    let items: Vec<ListItem> = profiles
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let style = if i == cursor {
                Style::default().fg(theme.selected_fg).bg(theme.selected_bg)
            } else {
                Style::default()
            };
            ListItem::new(Line::from(vec![Span::styled(format!("  {name}"), style)]))
        })
        .collect();

    let list = List::new(items).block(block);
    let mut state = ListState::default();
    if !profiles.is_empty() {
        state.select(Some(cursor.min(profiles.len() - 1)));
    }
    f.render_stateful_widget(list, area, &mut state);
}

fn draw_file_tree(f: &mut Frame, app: &App, area: Rect) {
    let in_search = matches!(app.mode(), crate::tui::mode::Mode::Search { .. });

    // Split off a 3-row search input pane when in search mode.
    let (search_area, tree_area) = if in_search {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(1)])
            .split(area);
        (Some(chunks[0]), chunks[1])
    } else {
        (None, area)
    };

    if let Some(sa) = search_area {
        if let crate::tui::mode::Mode::Search { query } = app.mode() {
            let input = Paragraph::new(format!(" /{query}▏"))
                .block(Block::default().borders(Borders::ALL).title(" search "));
            f.render_widget(input, sa);
        }
    }

    // Choose which entries to show — filtered search results or visible tree.
    let entries_to_show: Vec<(usize, &crate::tui::tree::TreeEntry)> = if in_search {
        app.search_results
            .iter()
            .enumerate()
            .filter_map(|(vi, &actual)| app.tree_entries.get(actual).map(|e| (vi, e)))
            .collect()
    } else {
        app.visible_tree
            .iter()
            .enumerate()
            .filter_map(|(vi, &actual)| app.tree_entries.get(actual).map(|e| (vi, e)))
            .collect()
    };

    let title = if in_search {
        format!(" results ({}) ", entries_to_show.len())
    } else {
        format!(" files ({}) ", entries_to_show.len())
    };

    let border_style = if app.focus == Focus::FileTree {
        Style::default().fg(app.focus_highlight.current(app.clock.now()))
    } else {
        Style::default()
    };
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    let items: Vec<ListItem> = entries_to_show
        .iter()
        .map(|(vi, entry)| {
            let indent = if in_search {
                String::new()
            } else {
                "  ".repeat(entry.depth)
            };
            let display_name = if in_search {
                entry.rel_path.to_string_lossy().to_string()
            } else {
                entry.name.clone()
            };
            let marker = if entry.is_dir {
                if entry.expanded { "▾ " } else { "▸ " }
            } else if app.bundled_paths.contains(&entry.rel_path) {
                "■ "
            } else {
                "▫ "
            };

            let style = if *vi == app.tree_cursor && app.focus == Focus::FileTree {
                Style::default()
                    .fg(app.theme.selected_fg)
                    .bg(app.theme.selected_bg)
            } else if entry.is_dir {
                Style::default().fg(app.theme.dir)
            } else if app.bundled_paths.contains(&entry.rel_path) {
                Style::default().fg(app.theme.accent)
            } else {
                Style::default()
            };

            ListItem::new(Line::from(vec![Span::styled(
                format!("{indent}{marker}{display_name}"),
                style,
            )]))
        })
        .collect();

    // Capture viewport height for PageUp/PageDown/half-page movement. Subtract 2
    // for the top + bottom borders; clamp to 0 if the area is tiny.
    app.tree_viewport_height
        .set(tree_area.height.saturating_sub(2));

    let list = List::new(items)
        .block(block)
        .highlight_symbol("▶ ")
        .highlight_style(
            Style::default()
                .fg(app.theme.selected_fg)
                .bg(app.theme.selected_bg),
        );
    let mut state = app.tree_list_state.borrow_mut();
    let selected = if entries_to_show.is_empty() {
        None
    } else if in_search {
        // In search mode the cursor is implicit — always highlight the top result.
        Some(0usize)
    } else {
        Some(app.tree_cursor.min(entries_to_show.len() - 1))
    };
    state.select(selected);
    f.render_stateful_widget(list, tree_area, &mut state);
}

fn draw_bundle_list(f: &mut Frame, app: &App, area: Rect) {
    let title = format!(" bundle ({} items) ", app.bundle.len());
    let border_style = if app.focus == Focus::BundleList {
        Style::default().fg(app.focus_highlight.current(app.clock.now()))
    } else {
        Style::default()
    };
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    if app.bundle.is_empty() {
        let msg = Paragraph::new("  (empty — press space to add files)")
            .style(Style::default().add_modifier(Modifier::DIM))
            .block(block);
        f.render_widget(msg, area);
        return;
    }

    let items: Vec<ListItem> = app
        .bundle
        .items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let tokens = app.item_tokens.get(i).copied().unwrap_or(0);
            let pct = if app.total_tokens > 0 {
                (tokens as f64 / app.total_tokens as f64) * 100.0
            } else {
                0.0
            };

            let display = item.display();
            let truncated = if display.len() > 28 {
                format!("{}…", &display[..27])
            } else {
                display
            };

            let icon = match &item.kind {
                crate::bundle::ItemKind::Function { .. } => "λ",
                crate::bundle::ItemKind::Type { .. } => "τ",
                _ => "■",
            };

            let text = format!(
                "{:>2} {} {:<28} {:>6} {:>5.1}%",
                i + 1,
                icon,
                truncated,
                tokens,
                pct
            );

            let mut style = if i == app.bundle_cursor && app.focus == Focus::BundleList {
                Style::default()
                    .fg(app.theme.selected_fg)
                    .bg(app.theme.selected_bg)
            } else if pct > 25.0 {
                Style::default().fg(app.theme.hotspot)
            } else {
                Style::default()
            };

            // Apply per-row fade-in if this path was just added.
            if let Some(fade) = app.bundle_row_fades.get(&item.path) {
                use crate::tui::motion::blend;
                use ratatui::style::Color;
                let bg = app.theme.bg;
                let opacity = fade.opacity(app.clock.now());
                if opacity < 0.999 {
                    let fg = style.fg.unwrap_or(Color::Gray);
                    style = style.fg(blend(opacity, fg, bg));
                }
            }

            ListItem::new(Line::from(vec![Span::styled(text, style)]))
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_symbol("▶ ")
        .highlight_style(
            Style::default()
                .fg(app.theme.selected_fg)
                .bg(app.theme.selected_bg),
        );
    // Capture viewport height for PageUp/PageDown on the bundle list.
    app.bundle_viewport_height
        .set(area.height.saturating_sub(2));
    let mut state = app.bundle_list_state.borrow_mut();
    if app.bundle.is_empty() {
        state.select(None);
    } else {
        state.select(Some(app.bundle_cursor.min(app.bundle.len() - 1)));
    }
    f.render_stateful_widget(list, area, &mut state);
}

fn draw_footer(f: &mut Frame, app: &App, area: Rect) {
    use crate::tui::mode::{InputField, Mode};
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(area);

    // Status / input area — when in an input mode, render the inline prompt
    // instead of the regular status message.
    match app.mode() {
        Mode::Narrow { start, end, field } => {
            let start_style = if *field == InputField::First {
                Style::default().fg(app.theme.accent)
            } else {
                Style::default()
            };
            let end_style = if *field == InputField::Second {
                Style::default().fg(app.theme.accent)
            } else {
                Style::default()
            };
            let line = Line::from(vec![
                Span::raw(" Narrow lines: "),
                Span::styled(
                    if start.is_empty() {
                        "start"
                    } else {
                        start.as_str()
                    },
                    start_style,
                ),
                Span::raw(" - "),
                Span::styled(if end.is_empty() { "end" } else { end.as_str() }, end_style),
                Span::raw("  (Tab switch, Enter confirm, Esc cancel)"),
            ]);
            f.render_widget(Paragraph::new(line), chunks[0]);
        }
        Mode::SaveProfile { name } => {
            let line = Line::from(vec![
                Span::raw(" Profile name: "),
                Span::styled(
                    if name.is_empty() {
                        "type a name"
                    } else {
                        name.as_str()
                    },
                    Style::default().fg(app.theme.accent),
                ),
                Span::raw("  (Enter save, Esc cancel)"),
            ]);
            f.render_widget(Paragraph::new(line), chunks[0]);
        }
        Mode::DiffPick {
            branch,
            entering_branch: true,
            ..
        } => {
            let line = Line::from(vec![
                Span::raw(" Diff branch: "),
                Span::styled(branch.as_str(), Style::default().fg(app.theme.accent)),
                Span::raw("  (Enter to load, Esc cancel)"),
            ]);
            f.render_widget(Paragraph::new(line), chunks[0]);
        }
        Mode::AddNote { tag, body, field } => {
            let tag_style = if *field == InputField::First {
                Style::default().fg(app.theme.accent)
            } else {
                Style::default()
            };
            let body_style = if *field == InputField::Second {
                Style::default().fg(app.theme.accent)
            } else {
                Style::default()
            };
            let line = Line::from(vec![
                Span::raw(" Tag: "),
                Span::styled(
                    if tag.is_empty() {
                        "(optional)"
                    } else {
                        tag.as_str()
                    },
                    tag_style,
                ),
                Span::raw("  Body: "),
                Span::styled(
                    if body.is_empty() {
                        "type here"
                    } else {
                        body.as_str()
                    },
                    body_style,
                ),
                Span::raw("  (Tab switch, Enter save, Esc cancel)"),
            ]);
            f.render_widget(Paragraph::new(line), chunks[0]);
        }
        _ => {
            use crate::tui::motion::blend;
            use ratatui::style::Color;
            let bg = app.theme.bg;
            let opacity = app.status_fade.opacity(app.clock.now());
            // Default dim gray for the status bar; faded toward bg based on
            // the status-fade timeline (fade-in / hold / fade-out).
            let fg = blend(opacity, Color::Gray, bg);
            let status =
                Paragraph::new(format!(" {}", app.status_message)).style(Style::default().fg(fg));
            f.render_widget(status, chunks[0]);
        }
    }

    // Hint row.
    let focus_label = match app.focus {
        Focus::FileTree => "tree",
        Focus::Viewer => "viewer",
        Focus::BundleList => "bundle",
    };
    let nav_keys = match app.focus {
        Focus::FileTree => "j/k PgUp/PgDn  Enter expand  space add  E/C expand-all",
        Focus::Viewer => "j/k scroll  drag=select  a add  Esc clear  v close",
        Focus::BundleList => "j/k PgUp/PgDn  Enter select",
    };
    let keys_text = match app.mode() {
        Mode::Normal => {
            format!("  > {focus_label}  |  {nav_keys}  |  / commands  Ctrl+F find  ? help  q quit")
        }
        _ => " Esc cancel".into(),
    };
    let keys = Paragraph::new(keys_text).style(Style::default().add_modifier(Modifier::DIM));
    f.render_widget(keys, chunks[1]);
}
