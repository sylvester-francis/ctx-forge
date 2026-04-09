use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

fn ctxforge() -> Command {
    Command::cargo_bin("ctxforge").unwrap()
}

#[test]
fn add_single_file() {
    let td = TempDir::new().unwrap();
    fs::write(td.path().join("foo.rs"), "fn foo() {}\n").unwrap();

    ctxforge()
        .current_dir(td.path())
        .args(["add", "foo.rs"])
        .assert()
        .success()
        .stdout(predicates::str::contains("added 1 item"));

    assert!(td.path().join(".ctxforge/bundle.json").exists());
}

#[test]
fn add_glob_expands() {
    let td = TempDir::new().unwrap();
    fs::create_dir_all(td.path().join("src")).unwrap();
    fs::write(td.path().join("src/a.rs"), "").unwrap();
    fs::write(td.path().join("src/b.rs"), "").unwrap();
    fs::write(td.path().join("src/c.go"), "").unwrap();

    ctxforge()
        .current_dir(td.path())
        .args(["add", "src/*.rs"])
        .assert()
        .success()
        .stdout(predicates::str::contains("added 2 item"));
}

#[test]
fn add_exclude_filters() {
    let td = TempDir::new().unwrap();
    fs::create_dir_all(td.path().join("src")).unwrap();
    fs::write(td.path().join("src/main.rs"), "").unwrap();
    fs::write(td.path().join("src/main_test.rs"), "").unwrap();

    ctxforge()
        .current_dir(td.path())
        .args(["add", "src/*.rs", "--exclude", "*_test.rs"])
        .assert()
        .success()
        .stdout(predicates::str::contains("added 1 item"));
}

#[test]
fn add_with_range() {
    let td = TempDir::new().unwrap();
    fs::write(td.path().join("a.rs"), "1\n2\n3\n4\n5\n").unwrap();

    ctxforge()
        .current_dir(td.path())
        .args(["add", "a.rs:2-4"])
        .assert()
        .success()
        .stdout(predicates::str::contains("added 1 item"));
}

#[test]
fn add_with_no_args_errors() {
    let td = TempDir::new().unwrap();
    ctxforge()
        .current_dir(td.path())
        .args(["add"])
        .assert()
        .failure();
}

#[test]
fn rm_by_path() {
    let td = TempDir::new().unwrap();
    fs::write(td.path().join("a.rs"), "").unwrap();
    fs::write(td.path().join("b.rs"), "").unwrap();

    ctxforge()
        .current_dir(td.path())
        .args(["add", "a.rs", "b.rs"])
        .assert()
        .success();

    ctxforge()
        .current_dir(td.path())
        .args(["rm", "a.rs"])
        .assert()
        .success()
        .stdout(predicates::str::contains("removed 1 item"));
}

#[test]
fn rm_by_index() {
    let td = TempDir::new().unwrap();
    fs::write(td.path().join("a.rs"), "").unwrap();
    fs::write(td.path().join("b.rs"), "").unwrap();

    ctxforge()
        .current_dir(td.path())
        .args(["add", "a.rs", "b.rs"])
        .assert()
        .success();

    ctxforge()
        .current_dir(td.path())
        .args(["rm", "1"])
        .assert()
        .success()
        .stdout(predicates::str::contains("removed #1"));
}

#[test]
fn clear_empties_bundle() {
    let td = TempDir::new().unwrap();
    fs::write(td.path().join("a.rs"), "").unwrap();

    ctxforge()
        .current_dir(td.path())
        .args(["add", "a.rs"])
        .assert()
        .success();

    ctxforge()
        .current_dir(td.path())
        .args(["clear"])
        .assert()
        .success()
        .stdout(predicates::str::contains("cleared 1 item"));

    ctxforge()
        .current_dir(td.path())
        .args(["clear"])
        .assert()
        .success()
        .stdout(predicates::str::contains("cleared 0 item"));
}
