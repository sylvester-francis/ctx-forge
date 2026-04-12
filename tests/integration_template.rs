//! Integration tests for the `ctxforge templates` subcommand and the
//! `--template` / `--task` flags on copy/export/pipe.

use assert_cmd::Command;
use tempfile::TempDir;

fn ctxforge() -> Command {
    Command::cargo_bin("ctxforge").unwrap()
}

#[test]
fn templates_list_empty_when_no_templates() {
    let td = TempDir::new().unwrap();
    let mut cmd = ctxforge();
    cmd.current_dir(td.path()).arg("templates");
    let output = cmd.output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("no templates yet"));
}

#[test]
fn templates_new_scaffolds_file() {
    let td = TempDir::new().unwrap();
    let mut cmd = ctxforge();
    cmd.current_dir(td.path())
        .arg("templates")
        .arg("new")
        .arg("bugfix");
    let output = cmd.output().unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let template_path = td
        .path()
        .join(".ctxforge")
        .join("templates")
        .join("bugfix.md");
    assert!(template_path.exists());
    let content = std::fs::read_to_string(&template_path).unwrap();
    assert!(content.contains("{{task}}"));
    assert!(content.contains("{{bundle}}"));
}

#[test]
fn templates_new_refuses_duplicate() {
    let td = TempDir::new().unwrap();
    ctxforge()
        .current_dir(td.path())
        .args(["templates", "new", "bugfix"])
        .assert()
        .success();

    let output = ctxforge()
        .current_dir(td.path())
        .args(["templates", "new", "bugfix"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("already exists"));
}

#[test]
fn templates_rm_deletes_project_template() {
    let td = TempDir::new().unwrap();
    ctxforge()
        .current_dir(td.path())
        .args(["templates", "new", "bugfix"])
        .assert()
        .success();

    let template_path = td
        .path()
        .join(".ctxforge")
        .join("templates")
        .join("bugfix.md");
    assert!(template_path.exists());

    ctxforge()
        .current_dir(td.path())
        .args(["templates", "rm", "bugfix"])
        .assert()
        .success();
    assert!(!template_path.exists());
}

#[test]
fn templates_list_shows_created_template() {
    let td = TempDir::new().unwrap();
    ctxforge()
        .current_dir(td.path())
        .args(["templates", "new", "bugfix"])
        .assert()
        .success();

    let output = ctxforge()
        .current_dir(td.path())
        .arg("templates")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("bugfix"));
    assert!(stdout.contains("PROJECT"));
}

#[test]
fn templates_new_rejects_path_separators() {
    let td = TempDir::new().unwrap();
    let output = ctxforge()
        .current_dir(td.path())
        .args(["templates", "new", "foo/bar"])
        .output()
        .unwrap();
    assert!(!output.status.success());
}

// --- Task 12: --template/--task flags on copy/export/pipe ---

fn create_test_template(td: &TempDir, name: &str, body: &str) {
    let dir = td.path().join(".ctxforge").join("templates");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(format!("{name}.md")), body).unwrap();
}

#[test]
fn export_with_template_wraps_bundle_in_stdout() {
    let td = TempDir::new().unwrap();
    std::fs::write(td.path().join("main.rs"), "fn main() {}\n").unwrap();
    create_test_template(&td, "wrap", "PREFIX\n{{bundle}}\nSUFFIX");
    ctxforge()
        .current_dir(td.path())
        .args(["add", "main.rs"])
        .assert()
        .success();
    let output = ctxforge()
        .current_dir(td.path())
        .args(["export", "--template", "wrap"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.starts_with("PREFIX"), "stdout was: {stdout}");
    assert!(stdout.contains("fn main()"));
    assert!(stdout.trim_end().ends_with("SUFFIX"));
}

#[test]
fn export_with_template_and_task_substitutes_both() {
    let td = TempDir::new().unwrap();
    std::fs::write(td.path().join("main.rs"), "fn main() {}\n").unwrap();
    create_test_template(&td, "wrap", "TASK={{task}}\nBUNDLE:\n{{bundle}}");
    ctxforge()
        .current_dir(td.path())
        .args(["add", "main.rs"])
        .assert()
        .success();
    let output = ctxforge()
        .current_dir(td.path())
        .args(["export", "--template", "wrap", "--task", "fix the bug"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("TASK=fix the bug"));
    assert!(stdout.contains("fn main()"));
}

#[test]
fn export_with_template_missing_required_task_errors() {
    let td = TempDir::new().unwrap();
    std::fs::write(td.path().join("main.rs"), "fn main() {}\n").unwrap();
    create_test_template(&td, "wrap", "TASK={{task}}\nBUNDLE:\n{{bundle}}");
    ctxforge()
        .current_dir(td.path())
        .args(["add", "main.rs"])
        .assert()
        .success();
    let output = ctxforge()
        .current_dir(td.path())
        .args(["export", "--template", "wrap"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("requires --task"));
}

#[test]
fn export_with_unknown_template_errors() {
    let td = TempDir::new().unwrap();
    std::fs::write(td.path().join("main.rs"), "fn main() {}\n").unwrap();
    ctxforge()
        .current_dir(td.path())
        .args(["add", "main.rs"])
        .assert()
        .success();
    let output = ctxforge()
        .current_dir(td.path())
        .args(["export", "--template", "nonexistent"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not found"));
}

// --- Task 11: Starter library tests ---

#[test]
fn templates_starters_lists_all_five() {
    let td = TempDir::new().unwrap();
    let output = ctxforge()
        .current_dir(td.path())
        .args(["templates", "starters"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    for name in ["bugfix", "code-review", "explain", "refactor", "migrate"] {
        assert!(
            stdout.contains(name),
            "starter '{name}' missing from output: {stdout}"
        );
    }
    assert!(stdout.contains("BUILT-IN STARTERS"));
}

#[test]
fn templates_new_from_starter_copies_content() {
    let td = TempDir::new().unwrap();
    let output = ctxforge()
        .current_dir(td.path())
        .args(["templates", "new", "my-bugfix", "--from", "bugfix"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let template_path = td
        .path()
        .join(".ctxforge")
        .join("templates")
        .join("my-bugfix.md");
    assert!(template_path.exists());
    let content = std::fs::read_to_string(&template_path).unwrap();
    // Starter-specific phrase
    assert!(content.contains("debugging a specific issue"));
    assert!(content.contains("{{task}}"));
    assert!(content.contains("{{bundle}}"));
}

#[test]
fn templates_new_from_unknown_starter_errors() {
    let td = TempDir::new().unwrap();
    let output = ctxforge()
        .current_dir(td.path())
        .args(["templates", "new", "my-thing", "--from", "nonexistent"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unknown starter"));
    assert!(
        stderr.contains("bugfix"),
        "error should list available starters"
    );
}

#[test]
fn templates_new_from_refuses_overwrite() {
    let td = TempDir::new().unwrap();
    ctxforge()
        .current_dir(td.path())
        .args(["templates", "new", "dup", "--from", "bugfix"])
        .assert()
        .success();
    let output = ctxforge()
        .current_dir(td.path())
        .args(["templates", "new", "dup", "--from", "code-review"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("already exists"));
}
