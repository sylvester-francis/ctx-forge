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

#[test]
fn add_nonexistent_file_suggests_close_match() {
    use assert_cmd::Command;
    use tempfile::TempDir;

    let td = TempDir::new().unwrap();
    std::fs::write(td.path().join("main.rs"), "fn main() {}").unwrap();
    std::fs::write(td.path().join("lib.rs"), "").unwrap();

    let mut cmd = Command::cargo_bin("ctxforge").unwrap();
    cmd.current_dir(td.path()).arg("add").arg("mian.rs"); // typo

    let output = cmd.output().expect("run ctxforge");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "should exit non-zero on missing file"
    );
    assert!(
        stderr.contains("file not found"),
        "stderr did not contain 'file not found': {stderr}"
    );
    assert!(
        stderr.contains("main.rs"),
        "should suggest main.rs (close to mian.rs); stderr: {stderr}"
    );
}

#[test]
fn add_brace_glob_expands_to_matching_files() {
    use assert_cmd::Command;
    use tempfile::TempDir;

    let td = TempDir::new().unwrap();
    std::fs::write(td.path().join("main.rs"), "fn main() {}").unwrap();
    std::fs::write(td.path().join("lib.rs"), "").unwrap();
    std::fs::write(td.path().join("other.txt"), "").unwrap();

    // `{main,lib}.rs` is a globset brace-alternation pattern that should
    // match `main.rs` and `lib.rs` (but not `other.txt`).
    let output = Command::cargo_bin("ctxforge")
        .unwrap()
        .current_dir(td.path())
        .args(["add", "{main,lib}.rs"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "brace glob should succeed (not error as 'file not found'): stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify the bundle now has 2 items (main.rs + lib.rs).
    let status = Command::cargo_bin("ctxforge")
        .unwrap()
        .current_dir(td.path())
        .arg("status")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&status.stdout);
    assert!(
        stdout.contains("main.rs"),
        "status should list main.rs: {stdout}"
    );
    assert!(
        stdout.contains("lib.rs"),
        "status should list lib.rs: {stdout}"
    );
}

#[test]
fn add_glob_matching_nothing_does_not_error() {
    use assert_cmd::Command;
    use tempfile::TempDir;

    let td = TempDir::new().unwrap();
    std::fs::write(td.path().join("main.rs"), "fn main() {}").unwrap();

    // A glob that matches no files should NOT produce a NotFound error.
    // It should warn (or silently succeed) and exit 0.
    let output = Command::cargo_bin("ctxforge")
        .unwrap()
        .current_dir(td.path())
        .args(["add", "*.nonexistent_extension"])
        .output()
        .unwrap();

    // Should NOT be a fatal error — globs that match nothing are
    // warnings, not errors. Exit code should be 0.
    assert!(
        output.status.success(),
        "glob matching nothing should exit 0; got exit {}: stderr={}",
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stderr)
    );
}
