use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

fn ctxforge() -> Command {
    Command::cargo_bin("ctxforge").unwrap()
}

fn setup(td: &TempDir) {
    fs::write(td.path().join("a.rs"), "fn main() {}\n").unwrap();
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
    ctxforge()
        .current_dir(td.path())
        .args(["note", "--tag", "tls", "abandoned rustls 0.22"])
        .assert()
        .success();
}

#[test]
fn export_attaches_memory_by_default() {
    let td = TempDir::new().unwrap();
    setup(&td);

    ctxforge()
        .current_dir(td.path())
        .args(["export"])
        .assert()
        .success()
        .stdout(predicates::str::contains("## Memory"))
        .stdout(predicates::str::contains("JWT in header"))
        .stdout(predicates::str::contains("rustls 0.22"))
        .stdout(predicates::str::contains("## `a.rs`"));
}

#[test]
fn export_no_memory_flag_omits_section() {
    let td = TempDir::new().unwrap();
    setup(&td);

    ctxforge()
        .current_dir(td.path())
        .args(["export", "--no-memory"])
        .assert()
        .success()
        .stdout(predicates::str::contains("## Memory").not())
        .stdout(predicates::str::contains("JWT").not())
        .stdout(predicates::str::contains("## `a.rs`"));
}

#[test]
fn export_memory_tag_filters_to_one() {
    let td = TempDir::new().unwrap();
    setup(&td);

    ctxforge()
        .current_dir(td.path())
        .args(["export", "--memory-tag", "auth"])
        .assert()
        .success()
        .stdout(predicates::str::contains("## Memory"))
        .stdout(predicates::str::contains("JWT in header"))
        .stdout(predicates::str::contains("rustls").not());
}

#[test]
fn export_memory_limit_truncates() {
    let td = TempDir::new().unwrap();
    fs::write(td.path().join("a.rs"), "").unwrap();
    ctxforge()
        .current_dir(td.path())
        .args(["add", "a.rs"])
        .assert()
        .success();
    for i in 0..5 {
        ctxforge()
            .current_dir(td.path())
            .args(["note", "--tag", "t", &format!("note {i}")])
            .assert()
            .success();
    }

    ctxforge()
        .current_dir(td.path())
        .args(["export", "--memory-limit", "2"])
        .assert()
        .success()
        .stdout(predicates::str::contains("note 4"))
        .stdout(predicates::str::contains("note 3"))
        .stdout(predicates::str::contains("note 0").not());
}
