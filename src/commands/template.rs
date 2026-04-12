//! `ctxforge templates` — list, scaffold, and remove prompt templates.

use crate::cli::TemplatesAction;
use crate::error::{CtxforgeError, Result};
use crate::output;
use crate::paths::{self, CtxforgeRoot};

/// A single starter template shipped with ctxforge.
struct Starter {
    name: &'static str,
    description: &'static str,
    content: &'static str,
}

static STARTERS: &[Starter] = &[
    Starter {
        name: "bugfix",
        description: "Debug a specific issue and propose a minimal fix",
        content: include_str!("../../templates/starters/bugfix.md"),
    },
    Starter {
        name: "code-review",
        description: "Review code for bugs, clarity, complexity",
        content: include_str!("../../templates/starters/code-review.md"),
    },
    Starter {
        name: "explain",
        description: "Explain how code works to a skilled engineer",
        content: include_str!("../../templates/starters/explain.md"),
    },
    Starter {
        name: "refactor",
        description: "Propose concrete refactoring changes",
        content: include_str!("../../templates/starters/refactor.md"),
    },
    Starter {
        name: "migrate",
        description: "Step-by-step migration plan",
        content: include_str!("../../templates/starters/migrate.md"),
    },
];

fn find_starter(name: &str) -> Option<&'static Starter> {
    STARTERS.iter().find(|s| s.name == name)
}

pub fn run(root: &CtxforgeRoot, action: Option<TemplatesAction>) -> Result<()> {
    match action {
        None => list(root),
        Some(TemplatesAction::New { name, from }) => new(root, &name, from.as_deref()),
        Some(TemplatesAction::Rm { name }) => rm(root, &name),
        Some(TemplatesAction::Starters) => starters(),
    }
}

fn list(root: &CtxforgeRoot) -> Result<()> {
    let project_templates = scan_dir(&root.templates_dir());
    let global_dir = paths::global_templates_dir();
    let global_templates = global_dir.as_deref().map(scan_dir).unwrap_or_default();

    if project_templates.is_empty() && global_templates.is_empty() {
        println!("  (no templates yet)");
        println!("  Tip: create one with `ctxforge templates new <name>`");
        return Ok(());
    }

    if !project_templates.is_empty() {
        println!("  PROJECT");
        for name in &project_templates {
            let p = root.templates_dir().join(format!("{name}.md"));
            let shadows = global_templates.contains(name);
            let suffix = if shadows { " (shadows global)" } else { "" };
            println!("    {:<14} {}{}", name, p.display(), suffix);
        }
    }
    if !global_templates.is_empty() {
        if !project_templates.is_empty() {
            println!();
        }
        println!("  GLOBAL");
        if let Some(g) = global_dir.as_deref() {
            for name in &global_templates {
                let p = g.join(format!("{name}.md"));
                let shadowed = project_templates.contains(name);
                let suffix = if shadowed { " (shadowed)" } else { "" };
                println!("    {:<14} {}{}", name, p.display(), suffix);
            }
        }
    }
    Ok(())
}

fn new(root: &CtxforgeRoot, name: &str, from: Option<&str>) -> Result<()> {
    if name.is_empty() {
        return Err(CtxforgeError::Msg("template name required".into()));
    }
    if name.contains('/') || name.contains('\\') {
        return Err(CtxforgeError::Msg(format!(
            "template name must not contain path separators (got '{name}')"
        )));
    }
    let dir = root.templates_dir();
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{name}.md"));
    if path.exists() {
        return Err(CtxforgeError::Msg(format!(
            "template '{name}' already exists at {}\n  Tip: edit it with '$EDITOR {}' or delete it first with 'ctxforge templates rm {name}'",
            path.display(),
            path.display()
        )));
    }

    let content = match from {
        None => {
            // Blank scaffold (existing behavior).
            format!(
                "# Template: {name}\n\n\
You are an expert software engineer. Below is the relevant code and notes.\n\n\
## Task\n\n{{{{task}}}}\n\n\
## Code and notes\n\n{{{{bundle}}}}\n"
            )
        }
        Some(starter_name) => {
            let starter = find_starter(starter_name).ok_or_else(|| {
                let available: Vec<&str> = STARTERS.iter().map(|s| s.name).collect();
                CtxforgeError::Msg(format!(
                    "unknown starter '{starter_name}'\n  available: {}\n  Tip: run 'ctxforge templates starters' for descriptions",
                    available.join(", ")
                ))
            })?;
            starter.content.to_string()
        }
    };

    std::fs::write(&path, content)?;
    let source_label = match from {
        Some(s) => format!(" from starter '{s}'"),
        None => String::new(),
    };
    output::success(&format!(
        "created template '{name}'{source_label} at {}",
        path.display()
    ));
    Ok(())
}

fn rm(root: &CtxforgeRoot, name: &str) -> Result<()> {
    if name.contains('/') || name.contains('\\') {
        return Err(CtxforgeError::Msg(format!(
            "template name must not contain path separators (got '{name}')"
        )));
    }
    let stem = name.strip_suffix(".md").unwrap_or(name);
    let project_path = root.template_path(stem);
    if !project_path.exists() {
        // Check if it exists in global — refuse to delete.
        if let Some(g) = paths::global_templates_dir() {
            let global_path = g.join(format!("{stem}.md"));
            if global_path.exists() {
                return Err(CtxforgeError::Msg(format!(
                    "refused to delete global template at {}\n  Tip: remove it with 'rm {}' directly, or override it with a project-local version.",
                    global_path.display(),
                    global_path.display()
                )));
            }
        }
        return Err(CtxforgeError::Msg(format!(
            "template '{stem}' not found at {}",
            project_path.display()
        )));
    }
    std::fs::remove_file(&project_path)?;
    output::success(&format!("deleted template '{stem}'"));
    Ok(())
}

fn starters() -> Result<()> {
    println!("  BUILT-IN STARTERS");
    for s in STARTERS {
        println!("    {:<14} {}", s.name, s.description);
    }
    println!();
    println!("  Instantiate with:  ctxforge templates new <project-name> --from <starter>");
    Ok(())
}

fn scan_dir(dir: &std::path::Path) -> Vec<String> {
    if !dir.is_dir() {
        return Vec::new();
    }
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type().map(|t| t.is_file()).unwrap_or(false)
                && e.path().extension().and_then(|x| x.to_str()) == Some("md")
        })
        .filter_map(|e| {
            e.path()
                .file_stem()
                .and_then(|s| s.to_str())
                .map(String::from)
        })
        .collect();
    names.sort();
    names
}
