//! MCP tool implementations. Each function returns a serde_json::Value
//! that becomes the `result.content` of the tool call response.

use crate::bundle::Bundle;
use crate::memory;
use crate::models;
use crate::paths::CtxforgeRoot;
use crate::resolve;
use crate::tokens;
use serde_json::{Value, json};

/// List of tools this server exposes.
pub fn tool_list() -> Value {
    json!({
        "tools": [
            {
                "name": "ctxforge_recall",
                "description": "Search memory notes by tag, keyword, or recency.",
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
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "body": { "type": "string", "description": "The note text" },
                        "tag": { "type": "string", "description": "Optional tag (e.g. 'auth', 'tls')" }
                    },
                    "required": ["body"]
                }
            },
            {
                "name": "ctxforge_load_bundle",
                "description": "Load a saved profile's bundle by name.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "profile": { "type": "string", "description": "Profile name to load" }
                    },
                    "required": ["profile"]
                }
            },
            {
                "name": "ctxforge_status",
                "description": "Check the current bundle's token budget against the model window.",
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            }
        ]
    })
}

pub fn call_tool(root: &CtxforgeRoot, name: &str, args: &Value) -> Result<Value, String> {
    match name {
        "ctxforge_recall" => tool_recall(root, args),
        "ctxforge_note" => tool_note(root, args),
        "ctxforge_load_bundle" => tool_load_bundle(root, args),
        "ctxforge_status" => tool_status(root),
        _ => Err(format!("unknown tool: {name}")),
    }
}

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

fn tool_load_bundle(root: &CtxforgeRoot, args: &Value) -> Result<Value, String> {
    let profile = args
        .get("profile")
        .and_then(|v| v.as_str())
        .ok_or("missing required 'profile' argument")?;
    let bundle = crate::profile::load(root, profile).map_err(|e| e.to_string())?;
    bundle.save(root).map_err(|e| e.to_string())?;
    Ok(json!({
        "type": "text",
        "text": format!("Loaded profile '{}' ({} items)", profile, bundle.len())
    }))
}

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
