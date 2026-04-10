//! Render the TUI layout.

use crate::tui::app::{App, Focus};
use crate::tui::theme;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Gauge, List, ListItem, Paragraph};

pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header + gauge
            Constraint::Min(5),    // main panels
            Constraint::Length(2), // status + keybindings
        ])
        .split(f.area());

    draw_header(f, app, chunks[0]);
    draw_panels(f, app, chunks[1]);
    draw_footer(f, app, chunks[2]);
}

fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let pct = app.window_pct();
    let color = theme::gauge_color(pct);

    let token_str = if app.exact_tokens {
        format!("{}", app.total_tokens)
    } else {
        format!("~{}", app.total_tokens)
    };

    let label = format!(
        " ctxforge │ {} │ {} / {} ({:.1}%)",
        app.model_name, token_str, app.model_window, pct
    );

    let ratio = (pct / 100.0).min(1.0);

    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL))
        .gauge_style(Style::default().fg(color))
        .label(label)
        .ratio(ratio);

    f.render_widget(gauge, area);
}

fn draw_panels(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    draw_file_tree(f, app, chunks[0]);
    draw_bundle_list(f, app, chunks[1]);
}

fn draw_file_tree(f: &mut Frame, app: &App, area: Rect) {
    let title = format!(" files ({}) ", app.tree_entries.len());
    let border_style = if app.focus == Focus::FileTree {
        Style::default().fg(ratatui::style::Color::Cyan)
    } else {
        Style::default()
    };
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    let items: Vec<ListItem> = app
        .tree_entries
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let indent = "  ".repeat(entry.depth);
            let marker = if entry.is_dir {
                "▸ "
            } else if app.bundled_paths.contains(&entry.rel_path) {
                "■ "
            } else {
                "▫ "
            };

            let style = if i == app.tree_cursor && app.focus == Focus::FileTree {
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
                format!("{indent}{marker}{}", entry.name),
                style,
            )]))
        })
        .collect();

    let list = List::new(items).block(block);
    f.render_widget(list, area);
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
            let truncated = if display.len() > 30 {
                format!("{}…", &display[..29])
            } else {
                display
            };

            let text = format!(
                "{:>2}  {:<30} {:>6} {:>5.1}%",
                i + 1,
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
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(area);

    // Status message.
    let status = Paragraph::new(format!(" {}", app.status_message))
        .style(Style::default().add_modifier(Modifier::DIM));
    f.render_widget(status, chunks[0]);

    // Keybindings.
    let keys = Paragraph::new(" ␣ toggle  j/k move  ↹ switch panel  c copy  q quit")
        .style(Style::default().add_modifier(Modifier::DIM));
    f.render_widget(keys, chunks[1]);
}
