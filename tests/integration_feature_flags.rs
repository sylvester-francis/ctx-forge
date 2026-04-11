//! Verify the project builds under all feature flag combinations.
//!
//! Each test runs `cargo check --no-default-features [--features ...]` and
//! asserts the build succeeds. Run with `cargo test --test integration_feature_flags`.

use std::process::Command;

fn cargo_check(features: &[&str]) {
    let mut cmd = Command::new("cargo");
    cmd.arg("check");
    cmd.arg("--no-default-features");
    if !features.is_empty() {
        cmd.arg("--features");
        cmd.arg(features.join(","));
    }
    let output = cmd.output().expect("failed to run cargo check");
    assert!(
        output.status.success(),
        "cargo check failed for features {:?}:\n{}",
        features,
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn builds_with_no_features() {
    cargo_check(&[]);
}

#[test]
fn builds_with_tui_only() {
    cargo_check(&["tui"]);
}

#[test]
fn builds_with_mcp_only() {
    cargo_check(&["mcp"]);
}

#[test]
fn builds_with_default_features() {
    cargo_check(&["tui", "mcp"]);
}

#[test]
fn builds_with_all_features() {
    cargo_check(&["tui", "mcp", "extract"]);
}
