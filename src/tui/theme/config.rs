//! User config for theming, loaded from `~/.config/ctxforge/config.toml`.
//!
//! Precedence (highest first):
//!   1. `CTXFORGE_THEME` environment variable
//!   2. `[ui] theme = "..."` from the config file
//!   3. Built-in default (`ctxforge`)
//!
//! Absent file = defaults. Parse errors are non-fatal — they fall back to
//! defaults and surface via the returned Result for callers that want to
//! log.

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    #[serde(default = "default_theme_name")]
    pub theme: String,
    #[serde(default)]
    pub default_send: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: default_theme_name(),
            default_send: None,
        }
    }
}

fn default_theme_name() -> String {
    "ctxforge".to_string()
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
struct FileLayout {
    #[serde(default)]
    ui: Ui,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Ui {
    #[serde(default = "default_theme_name")]
    theme: String,
    #[serde(default)]
    default_send: Option<String>,
}

impl Default for Ui {
    fn default() -> Self {
        Self {
            theme: default_theme_name(),
            default_send: None,
        }
    }
}

pub fn load_from(path: &Path) -> Result<Config, String> {
    if !path.exists() {
        return Ok(Config::default());
    }
    let body =
        std::fs::read_to_string(path).map_err(|e| format!("read {}: {}", path.display(), e))?;
    let file: FileLayout =
        toml::from_str(&body).map_err(|e| format!("parse {}: {}", path.display(), e))?;
    Ok(Config {
        theme: file.ui.theme,
        default_send: file.ui.default_send,
    })
}

pub fn save_to(path: &Path, config: &Config) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("mkdir {}: {}", parent.display(), e))?;
    }
    let file = FileLayout {
        ui: Ui {
            theme: config.theme.clone(),
            default_send: config.default_send.clone(),
        },
    };
    let body = toml::to_string_pretty(&file).map_err(|e| format!("encode: {e}"))?;
    let tmp = path.with_extension("toml.tmp");
    std::fs::write(&tmp, body).map_err(|e| format!("write {}: {}", tmp.display(), e))?;
    std::fs::rename(&tmp, path)
        .map_err(|e| format!("rename {} -> {}: {}", tmp.display(), path.display(), e))?;
    Ok(())
}

/// Determines the active theme name, honouring env override + config + default.
pub fn resolve_theme_name(path: &Path) -> String {
    if let Ok(name) = std::env::var("CTXFORGE_THEME") {
        if !name.is_empty() {
            return name;
        }
    }
    load_from(path)
        .map(|c| c.theme)
        .unwrap_or_else(|_| default_theme_name())
}
