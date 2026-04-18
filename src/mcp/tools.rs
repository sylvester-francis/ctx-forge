//! MCP tool implementations. Each function returns a serde_json::Value
//! that becomes the `result.content` of the tool call response.

use crate::bundle::{Bundle, Item};
use crate::format::{self, Format};
use crate::memory;
use crate::models;
use crate::paths::{self, CtxforgeRoot};
use crate::profile;
use crate::resolve;
use crate::source::{FileSource, Source};
#[cfg(feature = "extract")]
use crate::source::{FuncSource, TypeSource};
use crate::tokens;
use crate::walk;
use serde_json::{Value, json};

/// List of tools this server exposes.
pub fn tool_list() -> Value {
    json!({
        "tools": [
            // ── Memory ─────────────────────────────────────────────
            {
                "name": "ctxforge_recall",
                "description": "Search memory notes by tag, keyword, or recency.",
                "annotations": { "readOnlyHint": true },
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "tag": { "type": "string", "description": "Only notes with this tag" },
                        "search": { "type": "string", "description": "Search string (case-insensitive)" },
                        "limit": { "type": "integer", "description": "Max notes to return", "default": 20 }
                    }
                }
            },
            {
                "name": "ctxforge_note",
                "description": "Write a memory note that persists across sessions.",
                "annotations": { "destructiveHint": false },
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "body": { "type": "string", "description": "The note text" },
                        "tag": { "type": "string", "description": "Optional tag (e.g. 'auth', 'tls')" }
                    },
                    "required": ["body"]
                }
            },
            // ── Profiles ───────────────────────────────────────────
            {
                "name": "ctxforge_load_bundle",
                "description": "Load a saved profile's bundle by name.",
                "annotations": { "destructiveHint": true },
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "profile": { "type": "string", "description": "Profile name to load" }
                    },
                    "required": ["profile"]
                }
            },
            {
                "name": "ctxforge_save_bundle",
                "description": "Save the current bundle as a named profile for later reuse.",
                "annotations": { "destructiveHint": false },
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string", "description": "Profile name" }
                    },
                    "required": ["name"]
                }
            },
            {
                "name": "ctxforge_list_profiles",
                "description": "List all saved bundle profiles.",
                "annotations": { "readOnlyHint": true },
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            },
            // ── Status / inspection ────────────────────────────────
            {
                "name": "ctxforge_status",
                "description": "Check the current bundle's token budget against the model window.",
                "annotations": { "readOnlyHint": true },
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            },
            {
                "name": "ctxforge_list_items",
                "description": "List items in the current bundle with their paths, types, and token counts.",
                "annotations": { "readOnlyHint": true },
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            },
            // ── Context assembly ───────────────────────────────────
            {
                "name": "ctxforge_add_files",
                "description": "Add files, globs, or line ranges to the current context bundle.",
                "annotations": { "destructiveHint": false },
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "patterns": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "File paths, glob patterns, or path:start-end ranges"
                        }
                    },
                    "required": ["patterns"]
                }
            },
            {
                "name": "ctxforge_add_function",
                "description": "Add a function by name using tree-sitter extraction.",
                "annotations": { "destructiveHint": false },
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string", "description": "Function name to find and add" },
                        "file": { "type": "string", "description": "File path to search in" }
                    },
                    "required": ["name", "file"]
                }
            },
            {
                "name": "ctxforge_add_type",
                "description": "Add a type, struct, or class by name using tree-sitter extraction.",
                "annotations": { "destructiveHint": false },
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string", "description": "Type/struct/class name to find and add" },
                        "file": { "type": "string", "description": "File path to search in" }
                    },
                    "required": ["name", "file"]
                }
            },
            {
                "name": "ctxforge_remove",
                "description": "Remove items from the current bundle by file path or index.",
                "annotations": { "destructiveHint": true },
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "targets": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "File paths or numeric indices to remove"
                        }
                    },
                    "required": ["targets"]
                }
            },
            {
                "name": "ctxforge_clear",
                "description": "Remove all items from the current context bundle.",
                "annotations": { "destructiveHint": true },
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            },
            // ── Export ─────────────────────────────────────────────
            {
                "name": "ctxforge_export",
                "description": "Export the current bundle content in the specified format.",
                "annotations": { "readOnlyHint": true },
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "format": {
                            "type": "string",
                            "enum": ["markdown", "xml", "json"],
                            "description": "Output format (default: markdown)"
                        },
                        "include_memory": {
                            "type": "boolean",
                            "description": "Include memory notes in export (default: true)"
                        }
                    }
                }
            },
            // ── Templates ──────────────────────────────────────────
            {
                "name": "ctxforge_list_templates",
                "description": "List available prompt templates (built-in and custom).",
                "annotations": { "readOnlyHint": true },
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            },
            {
                "name": "ctxforge_apply_template",
                "description": "Render a prompt template, filling {{bundle}} with current bundle content and {{task}} with the provided task description.",
                "annotations": { "readOnlyHint": true },
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "template": { "type": "string", "description": "Template name" },
                        "task": { "type": "string", "description": "Task description to fill {{task}} placeholder" }
                    },
                    "required": ["template"]
                }
            },
            // ── Network sources ────────────────────────────────────
            {
                "name": "ctxforge_add_url",
                "description": "Add a URL as a context source. Content is fetched and cached with provenance tracking.",
                "annotations": { "destructiveHint": false },
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "url": { "type": "string", "description": "HTTPS URL to fetch" }
                    },
                    "required": ["url"]
                }
            },
            {
                "name": "ctxforge_refresh",
                "description": "Refresh cached URL sources. Omit `uri` to refresh all stale items; pass `all: true` to refresh even fresh entries.",
                "annotations": { "destructiveHint": false },
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "uri": { "type": "string", "description": "Canonical URI to refresh (e.g. url://host/path)" },
                        "all": { "type": "boolean", "description": "Refresh all cached items, not just stale" }
                    }
                }
            },
            {
                "name": "ctxforge_list_sources",
                "description": "List bundle items grouped by scheme with freshness status for cacheable sources.",
                "annotations": { "readOnlyHint": true },
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "scheme": { "type": "string", "description": "Filter by scheme (file, range, func, type, url)" },
                        "stale_only": { "type": "boolean", "description": "Only show stale items" }
                    }
                }
            },
            {
                "name": "ctxforge_docs_detect",
                "description": "Detect deps in the project and attach canonical doc URLs to the bundle.",
                "annotations": { "destructiveHint": false },
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "all": { "type": "boolean", "description": "Include Library-tier deps" },
                        "path": { "type": "string", "description": "Explicit path to a project / manifest" }
                    }
                }
            },
            {
                "name": "ctxforge_docs_add",
                "description": "Add a specific dep's documentation by name.",
                "annotations": { "destructiveHint": false },
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string" },
                        "ecosystem": { "type": "string", "description": "rust | js | python | go" }
                    },
                    "required": ["name"]
                }
            },
            {
                "name": "ctxforge_docs_list",
                "description": "List docs items currently attached to the bundle.",
                "annotations": { "readOnlyHint": true },
                "inputSchema": { "type": "object", "properties": {} }
            }
        ]
    })
}

pub fn call_tool(root: &CtxforgeRoot, name: &str, args: &Value) -> Result<Value, String> {
    match name {
        "ctxforge_recall" => tool_recall(root, args),
        "ctxforge_note" => tool_note(root, args),
        "ctxforge_load_bundle" => tool_load_bundle(root, args),
        "ctxforge_save_bundle" => tool_save_bundle(root, args),
        "ctxforge_list_profiles" => tool_list_profiles(root),
        "ctxforge_status" => tool_status(root),
        "ctxforge_list_items" => tool_list_items(root),
        "ctxforge_add_files" => tool_add_files(root, args),
        "ctxforge_add_function" => tool_add_function(root, args),
        "ctxforge_add_type" => tool_add_type(root, args),
        "ctxforge_remove" => tool_remove(root, args),
        "ctxforge_clear" => tool_clear(root),
        "ctxforge_export" => tool_export(root, args),
        "ctxforge_list_templates" => tool_list_templates(root),
        "ctxforge_apply_template" => tool_apply_template(root, args),
        "ctxforge_add_url" => tool_add_url(root, args),
        "ctxforge_refresh" => tool_refresh(root, args),
        "ctxforge_list_sources" => tool_list_sources(root, args),
        "ctxforge_docs_detect" => tool_docs_detect(root, args),
        "ctxforge_docs_add" => tool_docs_add(root, args),
        "ctxforge_docs_list" => tool_docs_list(root),
        _ => Err(format!("unknown tool: {name}")),
    }
}

fn tool_docs_detect(root: &CtxforgeRoot, args: &Value) -> Result<Value, String> {
    let all = args.get("all").and_then(|v| v.as_bool()).unwrap_or(false);
    let path = args
        .get("path")
        .and_then(|v| v.as_str())
        .map(std::path::PathBuf::from);
    crate::commands::docs_cmd::detect(root, all, path).map_err(|e| e.to_string())?;
    Ok(json!({
        "type": "text",
        "text": "docs detect complete"
    }))
}

fn tool_docs_add(root: &CtxforgeRoot, args: &Value) -> Result<Value, String> {
    let name = args
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or("missing required 'name' argument")?
        .to_string();
    let ecosystem = args
        .get("ecosystem")
        .and_then(|v| v.as_str())
        .map(String::from);
    crate::commands::docs_cmd::add(root, name.clone(), ecosystem).map_err(|e| e.to_string())?;
    Ok(json!({
        "type": "text",
        "text": format!("added docs: {name}")
    }))
}

fn tool_docs_list(root: &CtxforgeRoot) -> Result<Value, String> {
    let bundle = Bundle::load_or_default(root).map_err(|e| e.to_string())?;
    let items: Vec<serde_json::Value> = bundle
        .items
        .iter()
        .filter_map(|item| match &item.source {
            Source::Docs(d) => serde_json::to_value(d).ok(),
            _ => None,
        })
        .collect();
    Ok(json!({
        "type": "text",
        "text": serde_json::to_string_pretty(&items).unwrap_or_default()
    }))
}

fn tool_add_url(root: &CtxforgeRoot, args: &Value) -> Result<Value, String> {
    let url = args
        .get("url")
        .and_then(|v| v.as_str())
        .ok_or("missing required 'url' argument")?;
    crate::source::url::validate_url(url, false, false)?;
    let mut bundle = Bundle::load_or_default(root).map_err(|e| e.to_string())?;
    bundle.add(Item {
        source: Source::Url(crate::source::UrlSource { url: url.into() }),
        label: None,
    });
    bundle.save(root).map_err(|e| e.to_string())?;
    Ok(json!({
        "type": "text",
        "text": format!("Added URL source `{url}`; bundle now has {} item(s)", bundle.len()),
    }))
}

fn tool_refresh(root: &CtxforgeRoot, args: &Value) -> Result<Value, String> {
    let uri = args.get("uri").and_then(|v| v.as_str());
    let all = args.get("all").and_then(|v| v.as_bool()).unwrap_or(false);
    crate::commands::refresh::run(root, uri, all).map_err(|e| e.to_string())?;
    Ok(json!({
        "type": "text",
        "text": "refresh complete",
    }))
}

fn tool_list_sources(root: &CtxforgeRoot, args: &Value) -> Result<Value, String> {
    let scheme_filter = args.get("scheme").and_then(|v| v.as_str());
    let stale_only = args
        .get("stale_only")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let bundle = Bundle::load_or_default(root).map_err(|e| e.to_string())?;
    let cache =
        crate::paths::global_cache_dir().and_then(|d| crate::cache::ContentCache::open(d).ok());

    let mut out = Vec::new();
    for (i, item) in bundle.items.iter().enumerate() {
        let scheme = item.source.scheme_name();
        if let Some(s) = scheme_filter {
            if scheme != s {
                continue;
            }
        }
        let (fresh, stale_flag) =
            if let (true, Some(c)) = (item.source.is_cacheable(), cache.as_ref()) {
                let key = item.source.cache_key();
                match c.get(&key).unwrap_or(crate::cache::CacheRead::Miss) {
                    crate::cache::CacheRead::Fresh { .. } => (Some(true), false),
                    crate::cache::CacheRead::Stale { .. } => (Some(false), true),
                    crate::cache::CacheRead::Miss => (None, false),
                }
            } else {
                (None, false)
            };
        if stale_only && !stale_flag {
            continue;
        }
        out.push(json!({
            "index": i + 1,
            "uri": item.source.to_uri().to_string(),
            "scheme": scheme,
            "fresh": fresh,
            "stale": stale_flag,
            "label": item.label,
        }));
    }
    Ok(json!({
        "type": "text",
        "text": serde_json::to_string_pretty(&out).unwrap_or_default(),
    }))
}

// ── Memory ─────────────────────────────────────────────────────────────

fn tool_recall(root: &CtxforgeRoot, args: &Value) -> Result<Value, String> {
    let all = memory::index::read_all(root).map_err(|e| e.to_string())?;
    let filter = memory::RecallFilter {
        tag: args.get("tag").and_then(|v| v.as_str()).map(String::from),
        search: args
            .get("search")
            .and_then(|v| v.as_str())
            .map(String::from),
        since: None,
        limit: args
            .get("limit")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize),
    };
    let notes = memory::run_recall(&all, &filter);
    let items: Vec<Value> = notes
        .iter()
        .map(|n| {
            json!({
                "timestamp": n.timestamp.to_rfc3339(),
                "tag": n.tag,
                "body": n.body,
            })
        })
        .collect();
    Ok(json!({
        "type": "text",
        "text": serde_json::to_string_pretty(&items).unwrap_or_default()
    }))
}

fn tool_note(root: &CtxforgeRoot, args: &Value) -> Result<Value, String> {
    let body = args
        .get("body")
        .and_then(|v| v.as_str())
        .ok_or("missing required 'body' argument")?;
    let tag = args.get("tag").and_then(|v| v.as_str()).map(String::from);
    let note = memory::write_note(root, body, tag).map_err(|e| e.to_string())?;
    let ts = note.timestamp.format("%Y-%m-%d %H:%M UTC");
    Ok(json!({
        "type": "text",
        "text": format!("Noted: [{ts}] {}", note.body)
    }))
}

// ── Profiles ───────────────────────────────────────────────────────────

fn tool_load_bundle(root: &CtxforgeRoot, args: &Value) -> Result<Value, String> {
    let name = args
        .get("profile")
        .and_then(|v| v.as_str())
        .ok_or("missing required 'profile' argument")?;
    let bundle = profile::load(root, name).map_err(|e| e.to_string())?;
    bundle.save(root).map_err(|e| e.to_string())?;
    Ok(json!({
        "type": "text",
        "text": format!("Loaded profile '{}' ({} items)", name, bundle.len())
    }))
}

fn tool_save_bundle(root: &CtxforgeRoot, args: &Value) -> Result<Value, String> {
    let name = args
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or("missing required 'name' argument")?;
    let bundle = Bundle::load_or_default(root).map_err(|e| e.to_string())?;
    profile::save(root, name, &bundle).map_err(|e| e.to_string())?;
    Ok(json!({
        "type": "text",
        "text": format!("Saved profile '{}' ({} items)", name, bundle.len())
    }))
}

fn tool_list_profiles(root: &CtxforgeRoot) -> Result<Value, String> {
    let names = profile::list(root).map_err(|e| e.to_string())?;
    if names.is_empty() {
        return Ok(json!({
            "type": "text",
            "text": "No profiles saved yet."
        }));
    }
    let items: Vec<Value> = names
        .iter()
        .map(|n| {
            let count = profile::load(root, n).map(|b| b.len()).unwrap_or(0);
            json!({ "name": n, "item_count": count })
        })
        .collect();
    Ok(json!({
        "type": "text",
        "text": serde_json::to_string_pretty(&items).unwrap_or_default()
    }))
}

// ── Status / inspection ────────────────────────────────────────────────

fn tool_status(root: &CtxforgeRoot) -> Result<Value, String> {
    let bundle = Bundle::load_or_default(root).map_err(|e| e.to_string())?;
    if bundle.is_empty() {
        return Ok(json!({
            "type": "text",
            "text": "Bundle is empty. Use ctxforge add or ctxforge_load_bundle to add items."
        }));
    }

    let model_name = bundle.model.as_deref().unwrap_or(models::DEFAULT_MODEL);
    let model = models::lookup(model_name);

    let resolved =
        resolve::resolve_all(&bundle.items, root.project_root()).map_err(|e| e.to_string())?;
    let total: usize = resolved
        .iter()
        .map(|r| tokens::count(&r.content, &model).tokens)
        .sum();
    let pct = (total as f64 / model.window as f64) * 100.0;

    Ok(json!({
        "type": "text",
        "text": format!(
            "{} items, {} tokens ({}, {} window, {:.1}% used)",
            bundle.len(), total, model.name, model.window, pct
        )
    }))
}

fn tool_list_items(root: &CtxforgeRoot) -> Result<Value, String> {
    let bundle = Bundle::load_or_default(root).map_err(|e| e.to_string())?;
    if bundle.is_empty() {
        return Ok(json!({
            "type": "text",
            "text": "Bundle is empty."
        }));
    }

    let model_name = bundle.model.as_deref().unwrap_or(models::DEFAULT_MODEL);
    let model = models::lookup(model_name);
    let resolved =
        resolve::resolve_all(&bundle.items, root.project_root()).map_err(|e| e.to_string())?;

    let items: Vec<Value> = resolved
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let tc = tokens::count(&r.content, &model);
            let pct = (tc.tokens as f64 / model.window as f64) * 100.0;
            json!({
                "index": i + 1,
                "path": r.item.display(),
                "kind": r.item.source.scheme_name(),
                "tokens": tc.tokens,
                "percentage": format!("{:.1}%", pct),
            })
        })
        .collect();

    Ok(json!({
        "type": "text",
        "text": serde_json::to_string_pretty(&items).unwrap_or_default()
    }))
}

// ── Context assembly ───────────────────────────────────────────────────

fn is_uri_pattern(pat: &str) -> bool {
    pat.starts_with("https://")
        || pat.starts_with("http://")
        || pat.starts_with("url:")
        || pat.starts_with("file://")
        || pat.starts_with("range://")
        || pat.starts_with("func:")
        || pat.starts_with("type:")
}

fn tool_add_files(root: &CtxforgeRoot, args: &Value) -> Result<Value, String> {
    let patterns = args
        .get("patterns")
        .and_then(|v| v.as_array())
        .ok_or("missing required 'patterns' argument (array of strings)")?;

    let project_root = root.project_root().to_path_buf();
    let mut bundle = Bundle::load_or_default(root).map_err(|e| e.to_string())?;
    let mut added_count: usize = 0;

    for pat_val in patterns {
        let pat = pat_val.as_str().ok_or("each pattern must be a string")?;

        // URI-like patterns route through parse_add_argument. URL sources
        // get validated against the default (strict) policy — MCP clients
        // that want looser policy should use ctxforge_add_url explicitly.
        if is_uri_pattern(pat) {
            let item = Item::parse_add_argument(pat).map_err(|e| e.to_string())?;
            if let Source::Url(u) = &item.source {
                crate::source::url::validate_url(&u.url, false, false)?;
            }
            bundle.add(item);
            added_count += 1;
            continue;
        }

        // Check for range syntax (path:start-end)
        if let Some((_path_part, range_part)) = pat.rsplit_once(':') {
            if range_part.contains('-')
                && range_part.chars().all(|c| c.is_ascii_digit() || c == '-')
            {
                let item = Item::parse_add_argument(pat).map_err(|e| e.to_string())?;
                bundle.add(item);
                added_count += 1;
                continue;
            }
        }

        // Expand via walker (handles literal files, dirs, globs)
        let paths = walk::expand(pat, &project_root, &[]).map_err(|e| e.to_string())?;
        for p in paths {
            bundle.add(Item {
                source: Source::File(FileSource { path: p }),
                label: None,
            });
            added_count += 1;
        }
    }

    bundle.save(root).map_err(|e| e.to_string())?;

    Ok(json!({
        "type": "text",
        "text": format!("Added {} item(s); bundle now has {} item(s)", added_count, bundle.len())
    }))
}

fn tool_add_function(root: &CtxforgeRoot, args: &Value) -> Result<Value, String> {
    #[cfg(not(feature = "extract"))]
    {
        let _ = (root, args);
        Err(
            "tree-sitter extraction not available — rebuild ctxforge with `--features extract`"
                .into(),
        )
    }

    #[cfg(feature = "extract")]
    {
        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or("missing required 'name' argument")?;
        let file = args
            .get("file")
            .and_then(|v| v.as_str())
            .ok_or("missing required 'file' argument")?;

        let mut bundle = Bundle::load_or_default(root).map_err(|e| e.to_string())?;
        let item = Item {
            source: Source::Func(FuncSource {
                path: std::path::PathBuf::from(file),
                name: name.to_string(),
            }),
            label: None,
        };
        bundle.add(item);
        bundle.save(root).map_err(|e| e.to_string())?;

        Ok(json!({
            "type": "text",
            "text": format!("Added function '{}' from {}; bundle now has {} item(s)", name, file, bundle.len())
        }))
    }
}

fn tool_add_type(root: &CtxforgeRoot, args: &Value) -> Result<Value, String> {
    #[cfg(not(feature = "extract"))]
    {
        let _ = (root, args);
        Err(
            "tree-sitter extraction not available — rebuild ctxforge with `--features extract`"
                .into(),
        )
    }

    #[cfg(feature = "extract")]
    {
        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or("missing required 'name' argument")?;
        let file = args
            .get("file")
            .and_then(|v| v.as_str())
            .ok_or("missing required 'file' argument")?;

        let mut bundle = Bundle::load_or_default(root).map_err(|e| e.to_string())?;
        let item = Item {
            source: Source::Type(TypeSource {
                path: std::path::PathBuf::from(file),
                name: name.to_string(),
            }),
            label: None,
        };
        bundle.add(item);
        bundle.save(root).map_err(|e| e.to_string())?;

        Ok(json!({
            "type": "text",
            "text": format!("Added type '{}' from {}; bundle now has {} item(s)", name, file, bundle.len())
        }))
    }
}

fn tool_remove(root: &CtxforgeRoot, args: &Value) -> Result<Value, String> {
    let targets = args
        .get("targets")
        .and_then(|v| v.as_array())
        .ok_or("missing required 'targets' argument (array of strings)")?;

    let mut bundle = Bundle::load_or_default(root).map_err(|e| e.to_string())?;
    let mut removed_count: usize = 0;

    for target_val in targets {
        let target = target_val.as_str().ok_or("each target must be a string")?;

        // Try index first
        if let Ok(idx) = target.parse::<usize>() {
            if bundle.remove_by_index(idx).is_ok() {
                removed_count += 1;
                continue;
            }
        }

        // Otherwise path-based removal
        let count = bundle.remove_by_path(std::path::Path::new(target));
        removed_count += count;
    }

    bundle.save(root).map_err(|e| e.to_string())?;

    Ok(json!({
        "type": "text",
        "text": format!("Removed {} item(s); {} remaining", removed_count, bundle.len())
    }))
}

fn tool_clear(root: &CtxforgeRoot) -> Result<Value, String> {
    let mut bundle = Bundle::load_or_default(root).map_err(|e| e.to_string())?;
    let n = bundle.len();
    bundle.clear();
    bundle.save(root).map_err(|e| e.to_string())?;

    Ok(json!({
        "type": "text",
        "text": format!("Cleared {} item(s); bundle is now empty", n)
    }))
}

// ── Export ──────────────────────────────────────────────────────────────

fn tool_export(root: &CtxforgeRoot, args: &Value) -> Result<Value, String> {
    let bundle = Bundle::load_or_default(root).map_err(|e| e.to_string())?;
    if bundle.is_empty() {
        return Ok(json!({
            "type": "text",
            "text": "Bundle is empty. Add items first with ctxforge_add_files."
        }));
    }

    let fmt = match args.get("format").and_then(|v| v.as_str()) {
        Some("xml") => Format::Xml,
        Some("json") => Format::Json,
        _ => Format::Markdown,
    };

    let include_memory = args
        .get("include_memory")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let resolved =
        resolve::resolve_all(&bundle.items, root.project_root()).map_err(|e| e.to_string())?;
    let memory_notes = if include_memory {
        memory::collect_for_attach(root, false, None, 20).map_err(|e| e.to_string())?
    } else {
        Vec::new()
    };

    let rendered = format::render(fmt, &resolved, &memory_notes, false);

    Ok(json!({
        "type": "text",
        "text": rendered
    }))
}

// ── Templates ──────────────────────────────────────────────────────────

fn tool_list_templates(root: &CtxforgeRoot) -> Result<Value, String> {
    let project_dir = root.templates_dir();
    let global_dir = paths::global_templates_dir();

    let mut templates: Vec<Value> = Vec::new();

    // Scan project-local templates
    if project_dir.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&project_dir) {
            for entry in entries.flatten() {
                if entry.path().extension().and_then(|e| e.to_str()) == Some("md") {
                    if let Some(stem) = entry.path().file_stem().and_then(|s| s.to_str()) {
                        templates.push(json!({ "name": stem, "source": "project" }));
                    }
                }
            }
        }
    }

    // Scan global templates
    if let Some(ref g) = global_dir {
        if g.is_dir() {
            if let Ok(entries) = std::fs::read_dir(g) {
                for entry in entries.flatten() {
                    if entry.path().extension().and_then(|e| e.to_str()) == Some("md") {
                        if let Some(stem) = entry.path().file_stem().and_then(|s| s.to_str()) {
                            if !templates.iter().any(|t| t["name"] == stem) {
                                templates.push(json!({ "name": stem, "source": "global" }));
                            }
                        }
                    }
                }
            }
        }
    }

    if templates.is_empty() {
        return Ok(json!({
            "type": "text",
            "text": "No templates found. Create one with 'ctxforge templates new <name>'."
        }));
    }

    Ok(json!({
        "type": "text",
        "text": serde_json::to_string_pretty(&templates).unwrap_or_default()
    }))
}

fn tool_apply_template(root: &CtxforgeRoot, args: &Value) -> Result<Value, String> {
    let template_name = args
        .get("template")
        .and_then(|v| v.as_str())
        .ok_or("missing required 'template' argument")?;
    let task = args.get("task").and_then(|v| v.as_str()).unwrap_or("");

    let bundle = Bundle::load_or_default(root).map_err(|e| e.to_string())?;
    let resolved =
        resolve::resolve_all(&bundle.items, root.project_root()).map_err(|e| e.to_string())?;
    let memory_notes =
        memory::collect_for_attach(root, false, None, 20).map_err(|e| e.to_string())?;
    let bundle_rendered = format::render(Format::Markdown, &resolved, &memory_notes, false);

    let rendered =
        crate::template::apply_template(root, template_name, &bundle_rendered, Some(task))
            .map_err(|e| e.to_string())?;

    Ok(json!({
        "type": "text",
        "text": rendered
    }))
}
