use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

fn ctxforge() -> Command {
    Command::cargo_bin("ctxforge").unwrap()
}

#[test]
fn export_xml_flag_produces_xml_output() {
    let td = TempDir::new().unwrap();
    fs::write(td.path().join("main.rs"), "fn main() {}\n").unwrap();

    ctxforge()
        .current_dir(td.path())
        .args(["add", "main.rs"])
        .assert()
        .success();

    ctxforge()
        .current_dir(td.path())
        .args(["export", "--xml"])
        .assert()
        .success()
        .stdout(predicates::str::contains("<context items=\"1\">"))
        .stdout(predicates::str::contains(
            "<source path=\"main.rs\" language=\"rust\"",
        ))
        .stdout(predicates::str::contains("<![CDATA[fn main() {}"))
        .stdout(predicates::str::contains("</context>"));
}

#[test]
fn export_format_xml_equivalent_to_xml_shortcut() {
    let td = TempDir::new().unwrap();
    fs::write(td.path().join("main.rs"), "fn main() {}\n").unwrap();
    ctxforge()
        .current_dir(td.path())
        .args(["add", "main.rs"])
        .assert()
        .success();

    ctxforge()
        .current_dir(td.path())
        .args(["export", "--format", "xml"])
        .assert()
        .success()
        .stdout(predicates::str::contains("<context"));
}

#[test]
fn export_xml_includes_memory_when_notes_exist() {
    let td = TempDir::new().unwrap();
    fs::write(td.path().join("main.rs"), "fn main() {}\n").unwrap();
    ctxforge()
        .current_dir(td.path())
        .args(["add", "main.rs"])
        .assert()
        .success();
    ctxforge()
        .current_dir(td.path())
        .args(["note", "--tag", "auth", "JWT in header"])
        .assert()
        .success();

    ctxforge()
        .current_dir(td.path())
        .args(["export", "--xml"])
        .assert()
        .success()
        .stdout(predicates::str::contains("<memory count=\"1\">"))
        .stdout(predicates::str::contains("tag=\"auth\""))
        .stdout(predicates::str::contains("JWT in header"));
}

#[test]
fn export_xml_treats_markdown_as_documentation() {
    let td = TempDir::new().unwrap();
    fs::write(td.path().join("README.md"), "# Project\n").unwrap();
    ctxforge()
        .current_dir(td.path())
        .args(["add", "README.md"])
        .assert()
        .success();

    ctxforge()
        .current_dir(td.path())
        .args(["export", "--xml"])
        .assert()
        .success()
        .stdout(predicates::str::contains(
            "<documentation path=\"README.md\"",
        ))
        .stdout(predicates::str::contains("<source").not());
}

#[test]
fn export_format_rejects_unknown_value() {
    let td = TempDir::new().unwrap();
    ctxforge()
        .current_dir(td.path())
        .args(["export", "--format", "yaml"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("invalid --format"));
}
