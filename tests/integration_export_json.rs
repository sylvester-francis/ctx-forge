use assert_cmd::Command;
use serde_json::Value;
use std::fs;
use tempfile::TempDir;

fn ctxforge() -> Command {
    Command::cargo_bin("ctxforge").unwrap()
}

fn run_and_parse(dir: &std::path::Path, args: &[&str]) -> Value {
    let out = ctxforge()
        .current_dir(dir)
        .args(args)
        .output()
        .expect("command should run");
    assert!(
        out.status.success(),
        "ctxforge failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("output should be valid JSON")
}

#[test]
fn export_json_flag_produces_valid_json() {
    let td = TempDir::new().unwrap();
    fs::write(td.path().join("main.rs"), "fn main() {}\n").unwrap();
    ctxforge()
        .current_dir(td.path())
        .args(["add", "main.rs"])
        .assert()
        .success();

    let v = run_and_parse(td.path(), &["export", "--json"]);
    assert_eq!(v["schema_version"], 1);
    assert_eq!(v["items_count"], 1);
    let item = &v["items"][0];
    assert_eq!(item["path"], "main.rs");
    assert_eq!(item["language"], "rust");
    assert_eq!(item["kind"], "file");
    assert_eq!(item["content"], "fn main() {}\n");
}

#[test]
fn export_format_json_equivalent_to_json_shortcut() {
    let td = TempDir::new().unwrap();
    fs::write(td.path().join("a.rs"), "").unwrap();
    ctxforge()
        .current_dir(td.path())
        .args(["add", "a.rs"])
        .assert()
        .success();

    let v = run_and_parse(td.path(), &["export", "--format", "json"]);
    assert_eq!(v["items_count"], 1);
}

#[test]
fn export_json_includes_memory() {
    let td = TempDir::new().unwrap();
    fs::write(td.path().join("a.rs"), "").unwrap();
    ctxforge()
        .current_dir(td.path())
        .args(["add", "a.rs"])
        .assert()
        .success();
    ctxforge()
        .current_dir(td.path())
        .args(["note", "--tag", "auth", "JWT in header"])
        .assert()
        .success();

    let v = run_and_parse(td.path(), &["export", "--json"]);
    let memory = v["memory"].as_array().unwrap();
    assert_eq!(memory.len(), 1);
    assert_eq!(memory[0]["tag"], "auth");
    assert_eq!(memory[0]["body"], "JWT in header");
    assert!(memory[0]["timestamp"].as_str().unwrap().contains('T'));
}

#[test]
fn export_json_no_memory_flag_omits_notes() {
    let td = TempDir::new().unwrap();
    fs::write(td.path().join("a.rs"), "").unwrap();
    ctxforge()
        .current_dir(td.path())
        .args(["add", "a.rs"])
        .assert()
        .success();
    ctxforge()
        .current_dir(td.path())
        .args(["note", "anything"])
        .assert()
        .success();

    let v = run_and_parse(td.path(), &["export", "--json", "--no-memory"]);
    assert_eq!(v["memory"].as_array().unwrap().len(), 0);
}

#[test]
fn export_json_range_item_has_lines_object() {
    let td = TempDir::new().unwrap();
    fs::write(td.path().join("a.rs"), "1\n2\n3\n4\n5\n").unwrap();
    ctxforge()
        .current_dir(td.path())
        .args(["add", "a.rs:2-4"])
        .assert()
        .success();

    let v = run_and_parse(td.path(), &["export", "--json"]);
    let item = &v["items"][0];
    assert_eq!(item["kind"], "range");
    assert_eq!(item["lines"]["start"], 2);
    assert_eq!(item["lines"]["end"], 4);
    assert_eq!(item["content"], "2\n3\n4");
}
