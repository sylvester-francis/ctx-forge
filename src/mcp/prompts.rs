//! MCP prompt implementations: expose built-in templates as MCP prompts.

use crate::bundle::Bundle;
use crate::format::{self, Format};
use crate::memory;
use crate::paths::CtxforgeRoot;
use crate::resolve;
use serde_json::{Value, json};

struct PromptDef {
    name: &'static str,
    description: &'static str,
    template_name: &'static str,
    task_required: bool,
}

static PROMPTS: &[PromptDef] = &[
    PromptDef {
        name: "ctxforge_bugfix",
        description: "Investigate and fix a bug using the current context bundle.",
        template_name: "bugfix",
        task_required: true,
    },
    PromptDef {
        name: "ctxforge_code_review",
        description: "Review code for bugs, clarity, and complexity.",
        template_name: "code-review",
        task_required: false,
    },
    PromptDef {
        name: "ctxforge_explain",
        description: "Explain how code works to a skilled engineer.",
        template_name: "explain",
        task_required: false,
    },
    PromptDef {
        name: "ctxforge_refactor",
        description: "Propose concrete refactoring changes.",
        template_name: "refactor",
        task_required: true,
    },
    PromptDef {
        name: "ctxforge_migrate",
        description: "Create a step-by-step migration plan.",
        template_name: "migrate",
        task_required: true,
    },
];

/// List available prompts.
pub fn prompt_list() -> Value {
    let prompts: Vec<Value> = PROMPTS
        .iter()
        .map(|p| {
            json!({
                "name": p.name,
                "description": p.description,
                "arguments": [{
                    "name": "task",
                    "description": "What to do with the code",
                    "required": p.task_required,
                }],
            })
        })
        .collect();

    json!({ "prompts": prompts })
}

/// Get a rendered prompt by name.
pub fn get_prompt(root: &CtxforgeRoot, name: &str, args: &Value) -> Result<Value, String> {
    let prompt_def = PROMPTS
        .iter()
        .find(|p| p.name == name)
        .ok_or_else(|| format!("unknown prompt: {name}"))?;

    let task = args.get("task").and_then(|v| v.as_str()).unwrap_or("");

    if prompt_def.task_required && task.is_empty() {
        return Err(format!(
            "prompt '{}' requires a 'task' argument",
            prompt_def.name
        ));
    }

    // Render the bundle
    let bundle = Bundle::load_or_default(root).map_err(|e| e.to_string())?;
    let resolved =
        resolve::resolve_all(&bundle.items, root.project_root()).map_err(|e| e.to_string())?;
    let memory_notes =
        memory::collect_for_attach(root, false, None, 20).map_err(|e| e.to_string())?;
    let bundle_rendered = format::render(Format::Markdown, &resolved, &memory_notes);

    // Try to apply the template; if the template file doesn't exist, use a
    // simple fallback that still includes the bundle and task.
    let content = match crate::template::apply_template(
        root,
        prompt_def.template_name,
        &bundle_rendered,
        Some(task),
    ) {
        Ok(rendered) => rendered,
        Err(_) => {
            format!(
                "# {}\n\nTask: {}\n\n## Context\n\n{}",
                prompt_def.description, task, bundle_rendered
            )
        }
    };

    Ok(json!({
        "description": prompt_def.description,
        "messages": [{
            "role": "user",
            "content": {
                "type": "text",
                "text": content
            }
        }]
    }))
}
