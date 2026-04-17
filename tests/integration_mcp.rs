use std::io::Write;
use std::process::Stdio;
use tempfile::TempDir;

/// Helper: send a JSON-RPC request string to the MCP server and return raw stdout.
fn mcp_request(dir: &std::path::Path, request: &str) -> String {
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_ctxforge"))
        .arg("mcp")
        .current_dir(dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to start ctxforge mcp");

    let stdin = child.stdin.as_mut().unwrap();
    writeln!(stdin, "{}", request).unwrap();
    drop(child.stdin.take());

    let output = child.wait_with_output().unwrap();
    String::from_utf8(output.stdout).unwrap()
}

/// Helper: send multiple JSON-RPC requests and return all parsed responses.
fn mcp_requests(dir: &std::path::Path, requests: &[&str]) -> Vec<serde_json::Value> {
    let combined = requests.join("\n");
    let raw = mcp_request(dir, &combined);
    raw.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).expect("invalid JSON"))
        .collect()
}

// ── Protocol ───────────────────────────────────────────────────────────

#[test]
fn initialize_returns_updated_protocol_and_capabilities() {
    let td = TempDir::new().unwrap();
    let responses = mcp_requests(
        td.path(),
        &[r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#],
    );
    let result = &responses[0]["result"];
    assert_eq!(result["protocolVersion"], "2025-03-26");
    assert!(result["capabilities"]["tools"].is_object());
    assert!(result["capabilities"]["resources"].is_object());
    assert!(result["capabilities"]["prompts"].is_object());
    assert_eq!(result["serverInfo"]["name"], "ctxforge");
}

// ── Safety annotations ─────────────────────────────────────────────────

#[test]
fn tools_list_includes_annotations() {
    let td = TempDir::new().unwrap();
    let responses = mcp_requests(
        td.path(),
        &[
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#,
        ],
    );
    let tools = responses[1]["result"]["tools"].as_array().unwrap();

    let recall = tools
        .iter()
        .find(|t| t["name"] == "ctxforge_recall")
        .unwrap();
    assert_eq!(recall["annotations"]["readOnlyHint"], true);

    let note = tools.iter().find(|t| t["name"] == "ctxforge_note").unwrap();
    assert_eq!(note["annotations"]["destructiveHint"], false);

    let load = tools
        .iter()
        .find(|t| t["name"] == "ctxforge_load_bundle")
        .unwrap();
    assert_eq!(load["annotations"]["destructiveHint"], true);

    let status = tools
        .iter()
        .find(|t| t["name"] == "ctxforge_status")
        .unwrap();
    assert_eq!(status["annotations"]["readOnlyHint"], true);
}

// ── Context assembly tools ─────────────────────────────────────────────

#[test]
fn tool_add_files_adds_to_bundle() {
    let td = TempDir::new().unwrap();
    std::fs::write(td.path().join("hello.rs"), "fn main() {}").unwrap();

    let responses = mcp_requests(
        td.path(),
        &[
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"ctxforge_add_files","arguments":{"patterns":["hello.rs"]}}}"#,
        ],
    );
    let text = responses[1]["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    assert!(text.contains("1") && text.contains("Added"), "got: {text}");
}

#[test]
fn tool_remove_removes_from_bundle() {
    let td = TempDir::new().unwrap();
    std::fs::write(td.path().join("a.rs"), "fn a() {}").unwrap();
    std::fs::write(td.path().join("b.rs"), "fn b() {}").unwrap();

    let responses = mcp_requests(
        td.path(),
        &[
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"ctxforge_add_files","arguments":{"patterns":["a.rs","b.rs"]}}}"#,
            r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"ctxforge_remove","arguments":{"targets":["a.rs"]}}}"#,
        ],
    );
    let text = responses[2]["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    assert!(
        text.contains("Removed") && text.contains("1"),
        "got: {text}"
    );
}

#[test]
fn tool_clear_empties_bundle() {
    let td = TempDir::new().unwrap();
    std::fs::write(td.path().join("a.rs"), "fn a() {}").unwrap();

    let responses = mcp_requests(
        td.path(),
        &[
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"ctxforge_add_files","arguments":{"patterns":["a.rs"]}}}"#,
            r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"ctxforge_clear","arguments":{}}}"#,
        ],
    );
    let text = responses[2]["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    assert!(text.contains("Cleared"), "got: {text}");
}

// ── Export and list_items ──────────────────────────────────────────────

#[test]
fn tool_export_returns_bundle_content() {
    let td = TempDir::new().unwrap();
    std::fs::write(td.path().join("hello.rs"), "fn main() {}\n").unwrap();

    let responses = mcp_requests(
        td.path(),
        &[
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"ctxforge_add_files","arguments":{"patterns":["hello.rs"]}}}"#,
            r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"ctxforge_export","arguments":{"format":"markdown"}}}"#,
        ],
    );
    let text = responses[2]["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    assert!(
        text.contains("fn main()"),
        "export should contain file content, got: {text}"
    );
}

#[test]
fn tool_list_items_returns_item_info() {
    let td = TempDir::new().unwrap();
    std::fs::write(td.path().join("a.rs"), "fn a() {}\n").unwrap();

    let responses = mcp_requests(
        td.path(),
        &[
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"ctxforge_add_files","arguments":{"patterns":["a.rs"]}}}"#,
            r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"ctxforge_list_items","arguments":{}}}"#,
        ],
    );
    let text = responses[2]["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    assert!(
        text.contains("a.rs"),
        "list_items should show file path, got: {text}"
    );
}

// ── Profile tools ──────────────────────────────────────────────────────

#[test]
fn tool_save_and_list_profiles() {
    let td = TempDir::new().unwrap();
    std::fs::write(td.path().join("a.rs"), "fn a() {}").unwrap();

    let responses = mcp_requests(
        td.path(),
        &[
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"ctxforge_add_files","arguments":{"patterns":["a.rs"]}}}"#,
            r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"ctxforge_save_bundle","arguments":{"name":"my-profile"}}}"#,
            r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"ctxforge_list_profiles","arguments":{}}}"#,
        ],
    );
    let save_text = responses[2]["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    assert!(save_text.contains("my-profile"), "got: {save_text}");

    let list_text = responses[3]["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    assert!(list_text.contains("my-profile"), "got: {list_text}");
}

// ── Template tools ─────────────────────────────────────────────────────

#[test]
fn tool_list_templates_shows_available() {
    let td = TempDir::new().unwrap();
    let tpl_dir = td.path().join(".ctxforge").join("templates");
    std::fs::create_dir_all(&tpl_dir).unwrap();
    std::fs::write(
        tpl_dir.join("my-review.md"),
        "Review: {{bundle}}\nTask: {{task}}",
    )
    .unwrap();

    let responses = mcp_requests(
        td.path(),
        &[
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"ctxforge_list_templates","arguments":{}}}"#,
        ],
    );
    let text = responses[1]["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    assert!(text.contains("my-review"), "got: {text}");
}

#[test]
fn tool_apply_template_renders_content() {
    let td = TempDir::new().unwrap();
    let tpl_dir = td.path().join(".ctxforge").join("templates");
    std::fs::create_dir_all(&tpl_dir).unwrap();
    std::fs::write(
        tpl_dir.join("simple.md"),
        "Task: {{task}}\n\nContext:\n{{bundle}}",
    )
    .unwrap();
    std::fs::write(td.path().join("main.rs"), "fn main() {}\n").unwrap();

    let responses = mcp_requests(
        td.path(),
        &[
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"ctxforge_add_files","arguments":{"patterns":["main.rs"]}}}"#,
            r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"ctxforge_apply_template","arguments":{"template":"simple","task":"fix the bug"}}}"#,
        ],
    );
    let text = responses[2]["result"]["content"][0]["text"]
        .as_str()
        .unwrap();
    assert!(
        text.contains("fix the bug"),
        "should contain task, got: {text}"
    );
    assert!(
        text.contains("fn main()"),
        "should contain bundle content, got: {text}"
    );
}

// ── Resources ──────────────────────────────────────────────────────────

#[test]
fn resources_list_returns_resources() {
    let td = TempDir::new().unwrap();
    let responses = mcp_requests(
        td.path(),
        &[
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
            r#"{"jsonrpc":"2.0","id":2,"method":"resources/list","params":{}}"#,
        ],
    );
    let resources = responses[1]["result"]["resources"].as_array().unwrap();
    let uris: Vec<&str> = resources.iter().filter_map(|r| r["uri"].as_str()).collect();
    assert!(uris.contains(&"ctxforge://bundle"), "got: {uris:?}");
    assert!(uris.contains(&"ctxforge://bundle/items"), "got: {uris:?}");
    assert!(uris.contains(&"ctxforge://memory"), "got: {uris:?}");
}

#[test]
fn resources_read_bundle_returns_content() {
    let td = TempDir::new().unwrap();
    std::fs::write(td.path().join("a.rs"), "fn a() {}\n").unwrap();

    let responses = mcp_requests(
        td.path(),
        &[
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"ctxforge_add_files","arguments":{"patterns":["a.rs"]}}}"#,
            r#"{"jsonrpc":"2.0","id":3,"method":"resources/read","params":{"uri":"ctxforge://bundle"}}"#,
        ],
    );
    let text = responses[2]["result"]["contents"][0]["text"]
        .as_str()
        .unwrap();
    assert!(text.contains("fn a()"), "got: {text}");
}

// ── Prompts ────────────────────────────────────────────────────────────

#[test]
fn prompts_list_returns_builtin_prompts() {
    let td = TempDir::new().unwrap();
    let responses = mcp_requests(
        td.path(),
        &[
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
            r#"{"jsonrpc":"2.0","id":2,"method":"prompts/list","params":{}}"#,
        ],
    );
    let prompts = responses[1]["result"]["prompts"].as_array().unwrap();
    let names: Vec<&str> = prompts.iter().filter_map(|p| p["name"].as_str()).collect();
    assert!(names.contains(&"ctxforge_bugfix"), "got: {names:?}");
    assert!(names.contains(&"ctxforge_code_review"), "got: {names:?}");
    assert!(names.contains(&"ctxforge_explain"), "got: {names:?}");
    assert!(names.contains(&"ctxforge_refactor"), "got: {names:?}");
    assert!(names.contains(&"ctxforge_migrate"), "got: {names:?}");
}

#[test]
fn prompts_get_renders_prompt() {
    let td = TempDir::new().unwrap();
    let tpl_dir = td.path().join(".ctxforge").join("templates");
    std::fs::create_dir_all(&tpl_dir).unwrap();
    std::fs::write(
        tpl_dir.join("bugfix.md"),
        "Fix this: {{task}}\nCode:\n{{bundle}}",
    )
    .unwrap();
    std::fs::write(td.path().join("bug.rs"), "fn broken() {}\n").unwrap();

    let responses = mcp_requests(
        td.path(),
        &[
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"ctxforge_add_files","arguments":{"patterns":["bug.rs"]}}}"#,
            r#"{"jsonrpc":"2.0","id":3,"method":"prompts/get","params":{"name":"ctxforge_bugfix","arguments":{"task":"null pointer in broken()"}}}"#,
        ],
    );
    let content = responses[2]["result"]["messages"][0]["content"]["text"]
        .as_str()
        .unwrap();
    assert!(content.contains("null pointer"), "got: {content}");
    assert!(content.contains("fn broken()"), "got: {content}");
}

// ── Comprehensive tool count ───────────────────────────────────────────

#[test]
fn tools_list_contains_all_18_tools() {
    let td = TempDir::new().unwrap();
    let responses = mcp_requests(
        td.path(),
        &[
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#,
        ],
    );
    let tools = responses[1]["result"]["tools"].as_array().unwrap();

    let expected = vec![
        "ctxforge_recall",
        "ctxforge_note",
        "ctxforge_load_bundle",
        "ctxforge_save_bundle",
        "ctxforge_list_profiles",
        "ctxforge_status",
        "ctxforge_list_items",
        "ctxforge_add_files",
        "ctxforge_add_function",
        "ctxforge_add_type",
        "ctxforge_remove",
        "ctxforge_clear",
        "ctxforge_export",
        "ctxforge_list_templates",
        "ctxforge_apply_template",
        "ctxforge_add_url",
        "ctxforge_refresh",
        "ctxforge_list_sources",
    ];

    let actual: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();

    for name in &expected {
        assert!(
            actual.contains(name),
            "missing tool: {name}, have: {actual:?}"
        );
    }
    assert_eq!(
        actual.len(),
        expected.len(),
        "unexpected tool count: {actual:?}"
    );

    // Verify every tool has annotations
    for tool in tools {
        let name = tool["name"].as_str().unwrap();
        assert!(
            tool["annotations"].is_object(),
            "tool '{name}' is missing annotations"
        );
    }
}
