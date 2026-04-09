use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

fn ctxforge() -> Command {
    Command::cargo_bin("ctxforge").unwrap()
}

#[test]
fn resume_empty_project_prints_empty_bundle_and_no_notes() {
    let td = TempDir::new().unwrap();
    ctxforge()
        .current_dir(td.path())
        .args(["resume"])
        .assert()
        .success()
        .stdout(predicates::str::contains("(empty"))
        .stdout(predicates::str::contains("no notes yet"));
}

#[test]
fn resume_shows_bundle_items_and_recent_notes() {
    let td = TempDir::new().unwrap();
    fs::write(td.path().join("a.rs"), "fn main() {}\n").unwrap();

    ctxforge()
        .current_dir(td.path())
        .args(["add", "a.rs"])
        .assert()
        .success();
    ctxforge()
        .current_dir(td.path())
        .args(["note", "--tag", "tls", "abandoned rustls"])
        .assert()
        .success();

    ctxforge()
        .current_dir(td.path())
        .args(["resume"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Bundle: 1 item"))
        .stdout(predicates::str::contains("a.rs"))
        .stdout(predicates::str::contains("Recent notes"))
        .stdout(predicates::str::contains("abandoned rustls"))
        .stdout(predicates::str::contains("ctxforge copy"));
}

#[test]
fn resume_respects_memory_limit() {
    let td = TempDir::new().unwrap();
    for i in 0..5 {
        ctxforge()
            .current_dir(td.path())
            .args(["note", "--tag", "t", &format!("note {i}")])
            .assert()
            .success();
    }

    ctxforge()
        .current_dir(td.path())
        .args(["resume", "--memory-limit", "2"])
        .assert()
        .success()
        .stdout(predicates::str::contains("2 most recent"));
}
