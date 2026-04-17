//! MCP resource implementations: bundle and memory resources.

use crate::bundle::Bundle;
use crate::format::{self, Format};
use crate::memory;
use crate::models;
use crate::paths::CtxforgeRoot;
use crate::resolve;
use crate::tokens;
use serde_json::{Value, json};

/// List available resources and resource templates.
pub fn resource_list() -> Value {
    json!({
        "resources": [
            {
                "uri": "ctxforge://bundle",
                "name": "Current Bundle",
                "description": "Full resolved bundle content in markdown format.",
                "mimeType": "text/markdown"
            },
            {
                "uri": "ctxforge://bundle/items",
                "name": "Bundle Items",
                "description": "Item list with paths, kinds, and token counts.",
                "mimeType": "application/json"
            },
            {
                "uri": "ctxforge://memory",
                "name": "All Memory Notes",
                "description": "Complete memory note index.",
                "mimeType": "application/json"
            }
        ],
        "resourceTemplates": [
            {
                "uriTemplate": "ctxforge://memory/{tag}",
                "name": "Memory by Tag",
                "description": "Memory notes filtered by tag.",
                "mimeType": "application/json"
            }
        ]
    })
}

/// Read a specific resource by URI.
pub fn read_resource(root: &CtxforgeRoot, uri: &str) -> Result<Value, String> {
    match uri {
        "ctxforge://bundle" => read_bundle(root),
        "ctxforge://bundle/items" => read_bundle_items(root),
        "ctxforge://memory" => read_memory(root, None),
        _ if uri.starts_with("ctxforge://memory/") => {
            let tag = uri.strip_prefix("ctxforge://memory/").unwrap();
            if tag.is_empty() {
                return Err("tag cannot be empty in ctxforge://memory/{tag}".into());
            }
            read_memory(root, Some(tag))
        }
        _ => Err(format!("unknown resource: {uri}")),
    }
}

fn read_bundle(root: &CtxforgeRoot) -> Result<Value, String> {
    let bundle = Bundle::load_or_default(root).map_err(|e| e.to_string())?;
    let resolved =
        resolve::resolve_all(&bundle.items, root.project_root()).map_err(|e| e.to_string())?;
    let memory_notes =
        memory::collect_for_attach(root, false, None, 20).map_err(|e| e.to_string())?;
    let rendered = format::render(Format::Markdown, &resolved, &memory_notes);

    Ok(json!({
        "contents": [{
            "uri": "ctxforge://bundle",
            "mimeType": "text/markdown",
            "text": rendered
        }]
    }))
}

fn read_bundle_items(root: &CtxforgeRoot) -> Result<Value, String> {
    let bundle = Bundle::load_or_default(root).map_err(|e| e.to_string())?;
    let model_name = bundle.model.as_deref().unwrap_or(models::DEFAULT_MODEL);
    let model = models::lookup(model_name);
    let resolved =
        resolve::resolve_all(&bundle.items, root.project_root()).map_err(|e| e.to_string())?;

    let items: Vec<Value> = resolved
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let tc = tokens::count(&r.content, &model);
            json!({
                "index": i + 1,
                "path": r.item.display(),
                "kind": r.item.source.scheme_name(),
                "tokens": tc.tokens,
            })
        })
        .collect();

    Ok(json!({
        "contents": [{
            "uri": "ctxforge://bundle/items",
            "mimeType": "application/json",
            "text": serde_json::to_string_pretty(&items).unwrap_or_default()
        }]
    }))
}

fn read_memory(root: &CtxforgeRoot, tag: Option<&str>) -> Result<Value, String> {
    let all = memory::index::read_all(root).map_err(|e| e.to_string())?;
    let filter = memory::RecallFilter {
        tag: tag.map(String::from),
        ..Default::default()
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

    let uri = match tag {
        Some(t) => format!("ctxforge://memory/{t}"),
        None => "ctxforge://memory".to_string(),
    };

    Ok(json!({
        "contents": [{
            "uri": uri,
            "mimeType": "application/json",
            "text": serde_json::to_string_pretty(&items).unwrap_or_default()
        }]
    }))
}
