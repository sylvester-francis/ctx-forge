use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

fn ctxforge() -> Command {
    Command::cargo_bin("ctxforge").unwrap()
}

#[test]
fn export_to_stdout() {
    let td = TempDir::new().unwrap();
    fs::write(td.path().join("a.rs"), "fn main() {}\n").unwrap();

    ctxforge()
        .current_dir(td.path())
        .args(["add", "a.rs"])
        .assert()
        .success();

    ctxforge()
        .current_dir(td.path())
        .args(["export"])
        .assert()
        .success()
        .stdout(predicates::str::contains("## `a.rs`"))
        .stdout(predicates::str::contains("```rust"))
        .stdout(predicates::str::contains("fn main()"));
}

#[test]
fn export_to_file() {
    let td = TempDir::new().unwrap();
    fs::write(td.path().join("a.rs"), "fn main() {}\n").unwrap();

    ctxforge()
        .current_dir(td.path())
        .args(["add", "a.rs"])
        .assert()
        .success();

    let out = td.path().join("out.md");
    ctxforge()
        .current_dir(td.path())
        .args(["export", "-o"])
        .arg(&out)
        .assert()
        .success();

    let contents = fs::read_to_string(&out).unwrap();
    assert!(contents.contains("## `a.rs`"));
    assert!(contents.contains("fn main()"));
}
