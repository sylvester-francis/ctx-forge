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
