use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

fn ctxforge() -> Command {
    Command::cargo_bin("ctxforge").unwrap()
}

#[test]
fn save_and_load_roundtrip() {
    let td = TempDir::new().unwrap();
    fs::write(td.path().join("a.rs"), "").unwrap();

    ctxforge()
        .current_dir(td.path())
        .args(["add", "a.rs"])
        .assert()
        .success();
    ctxforge()
        .current_dir(td.path())
        .args(["save", "feature-a"])
        .assert()
        .success();
    ctxforge()
        .current_dir(td.path())
        .args(["clear"])
        .assert()
        .success();
    ctxforge()
        .current_dir(td.path())
        .args(["load", "feature-a"])
        .assert()
        .success()
        .stdout(predicates::str::contains(
            "loaded profile `feature-a` (1 items)",
        ));
}

#[test]
fn profiles_list_shows_saved() {
    let td = TempDir::new().unwrap();
    fs::write(td.path().join("a.rs"), "").unwrap();
    ctxforge()
        .current_dir(td.path())
        .args(["add", "a.rs"])
        .assert()
        .success();
    ctxforge()
        .current_dir(td.path())
        .args(["save", "zeta"])
        .assert()
        .success();
    ctxforge()
        .current_dir(td.path())
        .args(["save", "alpha"])
        .assert()
        .success();

    ctxforge()
        .current_dir(td.path())
        .args(["profiles"])
        .assert()
        .success()
        .stdout(predicates::str::contains("alpha"))
        .stdout(predicates::str::contains("zeta"));
}

#[test]
fn profiles_rm_removes() {
    let td = TempDir::new().unwrap();
    fs::write(td.path().join("a.rs"), "").unwrap();
    ctxforge()
        .current_dir(td.path())
        .args(["add", "a.rs"])
        .assert()
        .success();
    ctxforge()
        .current_dir(td.path())
        .args(["save", "doomed"])
        .assert()
        .success();
    ctxforge()
        .current_dir(td.path())
        .args(["profiles", "rm", "doomed"])
        .assert()
        .success();
    ctxforge()
        .current_dir(td.path())
        .args(["load", "doomed"])
        .assert()
        .failure();
}
