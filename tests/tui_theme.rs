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

#[test]
fn registry_has_four_themes() {
    let names: Vec<_> = registry::all_themes().iter().map(|t| t.name).collect();
    assert_eq!(names, vec!["ctxforge", "zinc", "tokyo-night", "gruvbox"]);
}

#[test]
fn every_theme_has_readable_accent_against_bg() {
    // WCAG AA for large text / UI chrome: contrast ratio >= 3.0.
    // Plain text (fg vs bg) should clear 4.5 where possible; Color::Reset
    // is treated as the terminal's default so we don't enforce that.
    for theme in registry::all_themes() {
        let ratio = contrast(theme.accent, theme.bg);
        assert!(
            ratio >= 3.0,
            "{} accent/bg contrast {:.2} < 3.0 (WCAG AA large)",
            theme.name,
            ratio
        );
        let ratio = contrast(theme.border_focused, theme.bg);
        assert!(
            ratio >= 3.0,
            "{} border_focused/bg contrast {:.2} < 3.0",
            theme.name,
            ratio
        );
    }
}

fn contrast(a: Color, b: Color) -> f64 {
    let la = luminance(a);
    let lb = luminance(b);
    let (l1, l2) = if la > lb { (la, lb) } else { (lb, la) };
    (l1 + 0.05) / (l2 + 0.05)
}

#[test]
fn config_round_trips_theme() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("config.toml");

    let config = ctxforge::tui::theme::config::Config {
        theme: "tokyo-night".to_string(),
        default_send: None,
    };
    ctxforge::tui::theme::config::save_to(&path, &config).unwrap();

    let loaded = ctxforge::tui::theme::config::load_from(&path).unwrap();
    assert_eq!(loaded.theme, "tokyo-night");
}

#[test]
fn config_missing_file_returns_defaults() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("does-not-exist.toml");
    let config = ctxforge::tui::theme::config::load_from(&path).unwrap();
    assert_eq!(config.theme, "ctxforge");
}

#[test]
fn set_theme_by_name_switches_app_theme() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().to_path_buf();
    let root = ctxforge::paths::CtxforgeRoot::find_or_create(tmp.path()).unwrap();
    // Confine the global config path to the tempdir so the test doesn't
    // touch the developer's real ~/.config/ctxforge/config.toml.
    temp_env::with_var("XDG_CONFIG_HOME", Some(home.as_os_str()), || {
        let mut app = ctxforge::tui::app::App::new(root);
        assert_eq!(app.theme.name, "ctxforge");
        app.set_theme_by_name("gruvbox");
        assert_eq!(app.theme.name, "gruvbox");
        // Persisted — reload resolves to gruvbox.
        let path = ctxforge::paths::config_file_path().unwrap();
        let name = ctxforge::tui::theme::config::resolve_theme_name(&path);
        assert_eq!(name, "gruvbox");
    });
}

#[test]
fn set_theme_by_name_rejects_unknown() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().to_path_buf();
    let root = ctxforge::paths::CtxforgeRoot::find_or_create(tmp.path()).unwrap();
    temp_env::with_var("XDG_CONFIG_HOME", Some(home.as_os_str()), || {
        let mut app = ctxforge::tui::app::App::new(root);
        app.set_theme_by_name("nonexistent");
        assert_eq!(app.theme.name, "ctxforge", "theme should not change");
        assert!(app.status_message.contains("unknown theme"));
    });
}

#[test]
fn env_override_beats_config() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("config.toml");
    std::fs::write(&path, "[ui]\ntheme = \"gruvbox\"\n").unwrap();

    // temp_env ensures we don't leak env between tests that run in parallel.
    let name = temp_env::with_var("CTXFORGE_THEME", Some("zinc"), || {
        ctxforge::tui::theme::config::resolve_theme_name(&path)
    });
    assert_eq!(name, "zinc");
}

fn luminance(c: Color) -> f64 {
    let (r, g, b) = match c {
        Color::Rgb(r, g, b) => (r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0),
        Color::Reset | Color::White => (1.0, 1.0, 1.0),
        Color::Black => (0.0, 0.0, 0.0),
        Color::Red => (0.5, 0.0, 0.0),
        Color::Green => (0.0, 0.5, 0.0),
        Color::Yellow => (0.5, 0.5, 0.0),
        Color::Blue => (0.0, 0.0, 0.5),
        Color::Magenta => (0.5, 0.0, 0.5),
        Color::Cyan => (0.0, 0.5, 0.5),
        Color::Gray => (0.7, 0.7, 0.7),
        Color::DarkGray => (0.4, 0.4, 0.4),
        _ => (0.5, 0.5, 0.5),
    };
    let lin = |c: f64| {
        if c <= 0.03928 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b)
}
