//! Render the TUI layout.

use crate::tui::app::{App, Focus};
use crate::tui::theme;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Gauge, List, ListItem, Paragraph};

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

    draw_header(f, app, chunks[0]);

    // Responsive layout: wide (≥120 cols) → 40/60 horizontal split;
    // narrow → vertical stack (bundle on top, tree below).
    if area.width >= 120 && area.height >= 30 {
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

    // Render overlays on top of the main layout.
    draw_overlays(f, app);
}

fn draw_overlays(f: &mut Frame, app: &App) {
    use crate::tui::mode::Mode;

    // Command palette overlay
    if matches!(app.mode, Mode::CommandPalette { .. }) {
        draw_command_palette(f, app);
    }

    // Help overlay
    if app.show_help || matches!(app.mode, Mode::Help) {
        draw_help_overlay(f);
    }

    // Legacy mode overlays
    match &app.mode {
        Mode::PipeMenu => {
            let area = centered_rect(30, 7, f.area());
            f.render_widget(ratatui::widgets::Clear, area);
            let block = Block::default()
                .title(" Pipe to Agent ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(ratatui::style::Color::Cyan));
            let text = vec![
                Line::from("  c  claude (XML)"),
                Line::from("  a  agent (markdown)"),
                Line::from("  g  gemini (markdown)"),
                Line::from(""),
                Line::from("  Esc cancel"),
            ];
            f.render_widget(Paragraph::new(text).block(block), area);
        }
        Mode::ModelSwitch { cursor } => {
            let models = crate::models::all_models();
            let height = (models.len() + 2).min(20) as u16;
            let area = centered_rect(45, height, f.area());
            f.render_widget(ratatui::widgets::Clear, area);
            let block = Block::default()
                .title(" Switch Model ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(ratatui::style::Color::Cyan));
            let items: Vec<ListItem> = models
                .iter()
                .enumerate()
                .map(|(i, m)| {
                    let style = if i == *cursor {
                        Style::default()
                            .fg(ratatui::style::Color::Black)
                            .bg(ratatui::style::Color::White)
                    } else {
                        Style::default()
                    };
                    ListItem::new(Span::styled(
                        format!("  {:25} {:>10}", m.name, m.window),
                        style,
                    ))
                })
                .collect();
            f.render_widget(List::new(items).block(block), area);
        }
        Mode::TemplatePick { cursor, templates } => {
            draw_template_pick_overlay(f, *cursor, templates);
        }
        Mode::TemplateTask {
            template_name,
            task,
        } => {
            draw_template_task_overlay(f, template_name, task);
        }
        _ => {}
    }
}

fn draw_command_palette(f: &mut Frame, app: &App) {
    use crate::tui::mode::Mode;
    let (query, cursor) = match &app.mode {
        Mode::CommandPalette { query, cursor } => (query, *cursor),
        _ => return,
    };
    let results = crate::tui::commands::fuzzy_filter(query);
    let area = centered_rect(70, 16, f.area());
    f.render_widget(ratatui::widgets::Clear, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // input
            Constraint::Min(5),    // results list
            Constraint::Length(3), // footer
        ])
        .split(area);

    // Input row
    let input_block = Block::default()
        .title(" / command palette ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ratatui::style::Color::Cyan));
    let input = Paragraph::new(format!(" /{query}")).block(input_block);
    f.render_widget(input, chunks[0]);

    // Results list
    let items: Vec<ListItem> = results
        .iter()
        .enumerate()
        .map(|(i, cmd)| {
            let marker = if i == cursor { " > " } else { "   " };
            let style = if i == cursor {
                Style::default()
                    .fg(ratatui::style::Color::Black)
                    .bg(ratatui::style::Color::White)
            } else {
                Style::default()
            };
            ListItem::new(Span::styled(
                format!("{marker}/{:<14} {}", cmd.name, cmd.description),
                style,
            ))
        })
        .collect();
    let list_block = Block::default().borders(Borders::ALL);
    let list = List::new(items).block(list_block);
    f.render_widget(list, chunks[1]);

    // Footer row
    let footer_text = format!(
        " {} of {} matches  |  Enter run  |  Esc cancel ",
        results.len(),
        crate::tui::commands::COMMANDS.len()
    );
    let footer = Paragraph::new(footer_text)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().add_modifier(Modifier::DIM));
    f.render_widget(footer, chunks[2]);
}

fn draw_help_overlay(f: &mut Frame) {
    let area = centered_rect(80, 22, f.area());
    f.render_widget(ratatui::widgets::Clear, area);
    let block = Block::default()
        .title(" ctxforge -- navigation keys ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ratatui::style::Color::Cyan));
    let lines = vec![
        Line::from(""),
        Line::from("  Movement                          Selection"),
        Line::from("    j / k     down / up               space  toggle file (in tree)"),
        Line::from("    g / G     top / bottom            Enter  expand directory"),
        Line::from("    Tab       switch panel"),
        Line::from(""),
        Line::from("  Discoverable input"),
        Line::from("    /         open command palette  (every feature lives here)"),
        Line::from("    Ctrl+F    file fuzzy search     (alias for /find)"),
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
    let p = Paragraph::new(lines).block(block);
    f.render_widget(p, area);
}

fn draw_template_pick_overlay(
    f: &mut Frame,
    cursor: usize,
    templates: &[(String, crate::tui::mode::TemplateSource)],
) {
    use crate::tui::mode::TemplateSource;
    let area = centered_rect(60, (templates.len() + 4).min(20) as u16, f.area());
    f.render_widget(ratatui::widgets::Clear, area);
    let block = Block::default()
        .title(" / template -- pick a template ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ratatui::style::Color::Cyan));
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
                Style::default()
                    .fg(ratatui::style::Color::Black)
                    .bg(ratatui::style::Color::White)
            } else {
                Style::default()
            };
            ListItem::new(Span::styled(
                format!("{marker}{:<20} ({source_label})", name),
                style,
            ))
        })
        .collect();
    f.render_widget(List::new(items).block(block), area);
}

fn draw_template_task_overlay(f: &mut Frame, template_name: &str, task: &str) {
    let area = centered_rect(70, 7, f.area());
    f.render_widget(ratatui::widgets::Clear, area);
    let block = Block::default()
        .title(format!(" task for template '{template_name}' "))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ratatui::style::Color::Cyan));
    let text = vec![
        Line::from(""),
        Line::from(format!("  > {task}")),
        Line::from(""),
        Line::from("  Enter to copy with template  |  Esc to cancel"),
    ];
    let p = Paragraph::new(text).block(block);
    f.render_widget(p, area);
}

/// Render the right panel (bundle list, profile list, or memory panel).
fn draw_right_panel(f: &mut Frame, app: &App, area: Rect) {
    // LoadProfile mode replaces the right panel with the profile picker.
    if let crate::tui::mode::Mode::LoadProfile { cursor, profiles } = &app.mode {
        draw_profile_list(f, *cursor, profiles, area);
        return;
    }

    // MemoryPanel mode replaces the right panel with the recall view.
    if matches!(app.mode, crate::tui::mode::Mode::MemoryPanel { .. }) {
        draw_memory_panel(f, app, area);
        return;
    }

    draw_bundle_list(f, app, area);
}

/// Helper to create a centered rect of given width/height inside `area`,
/// clamped to fit if `area` is too small.
fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}

fn draw_header(f: &mut Frame, app: &App, area: Rect) {
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

    let ratio = (pct / 100.0).min(1.0);

    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL))
        .gauge_style(Style::default().fg(color))
        .label(label)
        .ratio(ratio);

    f.render_widget(gauge, area);
}


/// Returns `true` if a picker overlay was rendered into the left panel,
/// in which case the caller should NOT also render the file tree.
fn draw_left_panel(f: &mut Frame, app: &App, area: Rect) -> bool {
    use crate::tui::mode::Mode;
    #[cfg(feature = "extract")]
    {
        if let Mode::FunctionPick { cursor, items } = &app.mode {
            draw_symbol_pick(f, "Functions", "λ", *cursor, items, area);
            return true;
        }
        if let Mode::TypePick { cursor, items } = &app.mode {
            draw_symbol_pick(f, "Types", "τ", *cursor, items, area);
            return true;
        }
    }
    if let Mode::DiffPick {
        files,
        selected,
        cursor,
        entering_branch,
        ..
    } = &app.mode
    {
        if !*entering_branch {
            draw_diff_pick(f, *cursor, files, selected, area);
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
) {
    let block = Block::default()
        .title(format!(" {title} ({}) ", items.len()))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ratatui::style::Color::Cyan));
    let list_items: Vec<ListItem> = items
        .iter()
        .enumerate()
        .map(|(i, (name, path))| {
            let style = if i == cursor {
                Style::default()
                    .fg(ratatui::style::Color::Black)
                    .bg(ratatui::style::Color::White)
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
) {
    let block = Block::default()
        .title(format!(" Changed Files ({}) ", files.len()))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ratatui::style::Color::Cyan));
    let items: Vec<ListItem> = files
        .iter()
        .enumerate()
        .map(|(i, path)| {
            let marker = if selected.contains(&i) { "■" } else { "▫" };
            let style = if i == cursor {
                Style::default()
                    .fg(ratatui::style::Color::Black)
                    .bg(ratatui::style::Color::White)
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
        .border_style(Style::default().fg(ratatui::style::Color::Cyan));

    let mut notes = crate::memory::index::read_all(&app.root).unwrap_or_default();
    // Show newest first to match recall semantics.
    notes.reverse();

    let cursor = if let crate::tui::mode::Mode::MemoryPanel { cursor, .. } = app.mode {
        cursor
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
                    .fg(ratatui::style::Color::Black)
                    .bg(ratatui::style::Color::White)
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
        f.render_widget(List::new(items).block(block), area);
    }
}

fn draw_profile_list(f: &mut Frame, cursor: usize, profiles: &[String], area: Rect) {
    let block = Block::default()
        .title(" Load Profile ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ratatui::style::Color::Cyan));

    let items: Vec<ListItem> = profiles
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let style = if i == cursor {
                Style::default()
                    .fg(ratatui::style::Color::Black)
                    .bg(ratatui::style::Color::White)
            } else {
                Style::default()
            };
            ListItem::new(Line::from(vec![Span::styled(format!("  {name}"), style)]))
        })
        .collect();

    f.render_widget(List::new(items).block(block), area);
}

fn draw_file_tree(f: &mut Frame, app: &App, area: Rect) {
    let in_search = matches!(app.mode, crate::tui::mode::Mode::Search { .. });

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
        if let crate::tui::mode::Mode::Search { ref query } = app.mode {
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
        Style::default().fg(ratatui::style::Color::Cyan)
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
                    .fg(ratatui::style::Color::Black)
                    .bg(ratatui::style::Color::White)
            } else if entry.is_dir {
                Style::default().fg(theme::dir_color())
            } else if app.bundled_paths.contains(&entry.rel_path) {
                Style::default().fg(theme::selected_color())
            } else {
                Style::default()
            };

            ListItem::new(Line::from(vec![Span::styled(
                format!("{indent}{marker}{display_name}"),
                style,
            )]))
        })
        .collect();

    let list = List::new(items).block(block);
    f.render_widget(list, tree_area);
}

fn draw_bundle_list(f: &mut Frame, app: &App, area: Rect) {
    let title = format!(" bundle ({} items) ", app.bundle.len());
    let border_style = if app.focus == Focus::BundleList {
        Style::default().fg(ratatui::style::Color::Cyan)
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

            let style = if i == app.bundle_cursor && app.focus == Focus::BundleList {
                Style::default()
                    .fg(ratatui::style::Color::Black)
                    .bg(ratatui::style::Color::White)
            } else if pct > 25.0 {
                Style::default().fg(theme::hotspot_color())
            } else {
                Style::default()
            };

            ListItem::new(Line::from(vec![Span::styled(text, style)]))
        })
        .collect();

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

fn draw_footer(f: &mut Frame, app: &App, area: Rect) {
    use crate::tui::mode::{InputField, Mode};
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(area);

    // Status / input area — when in an input mode, render the inline prompt
    // instead of the regular status message.
    match &app.mode {
        Mode::Narrow { start, end, field } => {
            let start_style = if *field == InputField::First {
                Style::default().fg(ratatui::style::Color::Cyan)
            } else {
                Style::default()
            };
            let end_style = if *field == InputField::Second {
                Style::default().fg(ratatui::style::Color::Cyan)
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
                    Style::default().fg(ratatui::style::Color::Cyan),
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
                Span::styled(
                    branch.as_str(),
                    Style::default().fg(ratatui::style::Color::Cyan),
                ),
                Span::raw("  (Enter to load, Esc cancel)"),
            ]);
            f.render_widget(Paragraph::new(line), chunks[0]);
        }
        Mode::AddNote { tag, body, field } => {
            let tag_style = if *field == InputField::First {
                Style::default().fg(ratatui::style::Color::Cyan)
            } else {
                Style::default()
            };
            let body_style = if *field == InputField::Second {
                Style::default().fg(ratatui::style::Color::Cyan)
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
            let status = Paragraph::new(format!(" {}", app.status_message))
                .style(Style::default().add_modifier(Modifier::DIM));
            f.render_widget(status, chunks[0]);
        }
    }

    // Hint row.
    let focus_label = match app.focus {
        Focus::FileTree => "tree",
        Focus::BundleList => "bundle",
    };
    let nav_keys = match app.focus {
        Focus::FileTree => "j/k move  Enter expand  space add",
        Focus::BundleList => "j/k move  Enter select",
    };
    let keys_text = match &app.mode {
        Mode::Normal => format!(
            "  > {focus_label}  |  {nav_keys}  |  / commands  Ctrl+F find  ? help  q quit"
        ),
        _ => " Esc cancel".into(),
    };
    let keys = Paragraph::new(keys_text).style(Style::default().add_modifier(Modifier::DIM));
    f.render_widget(keys, chunks[1]);
}
