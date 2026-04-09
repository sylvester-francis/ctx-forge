use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

fn ctxforge() -> Command {
    Command::cargo_bin("ctxforge").unwrap()
}

#[test]
fn status_empty_bundle() {
    let td = TempDir::new().unwrap();
    ctxforge()
        .current_dir(td.path())
        .args(["status"])
        .assert()
        .success()
        .stdout(predicates::str::contains("empty bundle"));
}

#[test]
fn status_shows_counts_and_total() {
    let td = TempDir::new().unwrap();
    fs::write(td.path().join("a.rs"), "fn a() {}\n".repeat(5)).unwrap();

    ctxforge()
        .current_dir(td.path())
        .args(["add", "a.rs"])
        .assert()
        .success();

    ctxforge()
        .current_dir(td.path())
        .args(["status"])
        .assert()
        .success()
        .stdout(predicates::str::contains("a.rs"))
        .stdout(predicates::str::contains("Total:"))
        .stdout(predicates::str::contains("claude-sonnet-4"));
}

#[test]
fn status_respects_model_override() {
    let td = TempDir::new().unwrap();
    fs::write(td.path().join("a.rs"), "fn a() {}\n").unwrap();

    ctxforge()
        .current_dir(td.path())
        .args(["add", "a.rs"])
        .assert()
        .success();

    ctxforge()
        .current_dir(td.path())
        .args(["--model", "gpt-4o", "status"])
        .assert()
        .success()
        .stdout(predicates::str::contains("gpt-4o"))
        .stdout(predicates::str::contains("128,000"));
}
