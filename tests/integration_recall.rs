use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn ctxforge() -> Command {
    Command::cargo_bin("ctxforge").unwrap()
}

fn seed_notes(dir: &std::path::Path) {
    ctxforge()
        .current_dir(dir)
        .args(["note", "--tag", "auth", "JWT in header"])
        .assert()
        .success();
    ctxforge()
        .current_dir(dir)
        .args(["note", "--tag", "tls", "abandoned rustls 0.22"])
        .assert()
        .success();
    ctxforge()
        .current_dir(dir)
        .args(["note", "plain note with no tag"])
        .assert()
        .success();
}

#[test]
fn recall_with_no_notes_prints_empty_message() {
    let td = TempDir::new().unwrap();
    ctxforge()
        .current_dir(td.path())
        .args(["recall"])
        .assert()
        .success()
        .stdout(predicates::str::contains("no notes yet"));
}

#[test]
fn recall_lists_all_notes_by_default() {
    let td = TempDir::new().unwrap();
    seed_notes(td.path());

    ctxforge()
        .current_dir(td.path())
        .args(["recall"])
        .assert()
        .success()
        .stdout(predicates::str::contains("JWT in header"))
        .stdout(predicates::str::contains("rustls"))
        .stdout(predicates::str::contains("plain note"))
        .stdout(predicates::str::contains("3 of 3 note(s)"));
}

#[test]
fn recall_filters_by_tag() {
    let td = TempDir::new().unwrap();
    seed_notes(td.path());

    ctxforge()
        .current_dir(td.path())
        .args(["recall", "--tag", "auth"])
        .assert()
        .success()
        .stdout(predicates::str::contains("JWT in header"))
        .stdout(predicates::str::contains("1 of 3 note(s)"))
        .stdout(predicates::str::contains("rustls").not());
}

#[test]
fn recall_filters_by_search() {
    let td = TempDir::new().unwrap();
    seed_notes(td.path());

    ctxforge()
        .current_dir(td.path())
        .args(["recall", "--search", "rustls"])
        .assert()
        .success()
        .stdout(predicates::str::contains("rustls"))
        .stdout(predicates::str::contains("JWT").not());
}

#[test]
fn recall_rejects_bad_since_value() {
    let td = TempDir::new().unwrap();
    seed_notes(td.path());
    ctxforge()
        .current_dir(td.path())
        .args(["recall", "--since", "bogus"])
        .assert()
        .failure();
}
