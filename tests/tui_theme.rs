//! Tests for the TUI theme registry.

use ctxforge::tui::theme::registry;
use ratatui::style::Color;

#[test]
fn ctxforge_theme_has_expected_fields() {
    let theme = registry::by_name("ctxforge").expect("ctxforge theme must exist");
    assert_eq!(theme.name, "ctxforge");
    assert_eq!(theme.bg, Color::Rgb(10, 14, 22));
    assert_eq!(theme.accent, Color::Cyan);
}

#[test]
fn registry_default_is_ctxforge() {
    let theme = registry::default_theme();
    assert_eq!(theme.name, "ctxforge");
}

#[test]
fn registry_lists_at_least_one_theme() {
    assert!(!registry::all_themes().is_empty());
}

#[test]
fn compat_helpers_match_default_theme() {
    let t = registry::default_theme();
    assert_eq!(ctxforge::tui::theme::selected_color(), t.accent);
    assert_eq!(ctxforge::tui::theme::dir_color(), t.dir);
    assert_eq!(ctxforge::tui::theme::hotspot_color(), t.hotspot);
}

#[test]
fn gauge_color_buckets_are_stable() {
    use ctxforge::tui::theme::gauge_color;
    assert_eq!(gauge_color(10.0), Color::Green);
    assert_eq!(gauge_color(50.0), Color::Yellow);
    assert_eq!(gauge_color(80.0), Color::Rgb(255, 165, 0));
    assert_eq!(gauge_color(95.0), Color::Red);
}

#[test]
fn app_has_default_theme() {
    let tmp = tempfile::tempdir().unwrap();
    let root = ctxforge::paths::CtxforgeRoot::find_or_create(tmp.path()).unwrap();
    let app = ctxforge::tui::app::App::new(root);
    assert_eq!(app.theme.name, "ctxforge");
}
