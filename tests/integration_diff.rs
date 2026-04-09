use assert_cmd::Command;
use std::fs;
use std::process::Command as StdCommand;
use tempfile::TempDir;

fn ctxforge() -> Command {
    Command::cargo_bin("ctxforge").unwrap()
}

fn git(dir: &std::path::Path, args: &[&str]) {
    let status = StdCommand::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?} failed");
}

#[test]
fn add_diff_branch_pulls_changed_files() {
    let td = TempDir::new().unwrap();
    let p = td.path();

    git(p, &["init", "-q", "-b", "main"]);
    git(p, &["config", "user.email", "t@t"]);
    git(p, &["config", "user.name", "t"]);

    fs::write(p.join("a.rs"), "fn a() {}\n").unwrap();
    git(p, &["add", "a.rs"]);
    git(p, &["commit", "-q", "-m", "init"]);

    git(p, &["checkout", "-q", "-b", "feature"]);
    fs::write(p.join("b.rs"), "fn b() {}\n").unwrap();
    git(p, &["add", "b.rs"]);
    git(p, &["commit", "-q", "-m", "add b"]);

    ctxforge()
        .current_dir(p)
        .args(["add", "--diff", "main"])
        .assert()
        .success()
        .stdout(predicates::str::contains("added 1 item"));

    ctxforge()
        .current_dir(p)
        .args(["status"])
        .assert()
        .success()
        .stdout(predicates::str::contains("b.rs"));
}
