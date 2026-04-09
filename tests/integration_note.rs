use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

fn ctxforge() -> Command {
    Command::cargo_bin("ctxforge").unwrap()
}

#[test]
fn note_creates_index_and_tag_file() {
    let td = TempDir::new().unwrap();

    ctxforge()
        .current_dir(td.path())
        .args([
            "note",
            "--tag",
            "auth",
            "JWT",
            "goes",
            "in",
            "Authorization",
            "header",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("noted"))
        .stdout(predicates::str::contains("[auth]"));

    let index = td.path().join(".ctxforge/memory/_index.jsonl");
    assert!(index.exists(), "index should exist");
    let idx_contents = fs::read_to_string(&index).unwrap();
    assert!(idx_contents.contains("JWT goes in Authorization header"));
    assert!(idx_contents.contains("\"tag\":\"auth\""));

    let tag_file = td.path().join(".ctxforge/memory/auth.md");
    assert!(tag_file.exists(), "tag file should exist");
    let tag_contents = fs::read_to_string(&tag_file).unwrap();
    assert!(tag_contents.contains("# Memory — auth"));
    assert!(tag_contents.contains("JWT goes in Authorization header"));
}

#[test]
fn note_without_tag_goes_to_decisions() {
    let td = TempDir::new().unwrap();

    ctxforge()
        .current_dir(td.path())
        .args(["note", "general", "decision", "about", "architecture"])
        .assert()
        .success();

    let decisions = td.path().join(".ctxforge/memory/decisions.md");
    assert!(decisions.exists(), "decisions.md should exist");
    let contents = fs::read_to_string(&decisions).unwrap();
    assert!(contents.contains("general decision about architecture"));
}

#[test]
fn note_without_body_errors() {
    let td = TempDir::new().unwrap();
    ctxforge()
        .current_dir(td.path())
        .args(["note"])
        .assert()
        .failure();
}
