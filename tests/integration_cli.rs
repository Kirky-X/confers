//! CLI integration tests.

#![cfg(feature = "cli")]

use std::path::PathBuf;
use std::process::Command;

fn get_cli_binary() -> PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let target_dir = PathBuf::from(manifest_dir).join("target/debug");
    target_dir.join("confers-cli")
}

#[test]
fn test_cli_help() {
    let binary = get_cli_binary();
    if !binary.exists() {
        eprintln!("CLI binary not found, skipping test");
        return;
    }

    let output = Command::new(&binary)
        .arg("--help")
        .output()
        .expect("Failed to execute CLI");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("confers"));
}

#[test]
fn test_cli_inspect_help() {
    let binary = get_cli_binary();
    if !binary.exists() {
        eprintln!("CLI binary not found, skipping test");
        return;
    }

    let output = Command::new(&binary)
        .args(["inspect", "--help"])
        .output()
        .expect("Failed to execute CLI");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("inspect"));
}

#[test]
fn test_cli_validate_help() {
    let binary = get_cli_binary();
    if !binary.exists() {
        eprintln!("CLI binary not found, skipping test");
        return;
    }

    let output = Command::new(&binary)
        .args(["validate", "--help"])
        .output()
        .expect("Failed to execute CLI");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("validate"));
}

#[test]
fn test_cli_export_help() {
    let binary = get_cli_binary();
    if !binary.exists() {
        eprintln!("CLI binary not found, skipping test");
        return;
    }

    let output = Command::new(&binary)
        .args(["export", "--help"])
        .output()
        .expect("Failed to execute CLI");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("export"));
}

#[test]
fn test_cli_snapshot_help() {
    let binary = get_cli_binary();
    if !binary.exists() {
        eprintln!("CLI binary not found, skipping test");
        return;
    }

    let output = Command::new(&binary)
        .args(["snapshot", "--help"])
        .output()
        .expect("Failed to execute CLI");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("snapshot"));
}

#[test]
fn test_cli_diff_help() {
    let binary = get_cli_binary();
    if !binary.exists() {
        eprintln!("CLI binary not found, skipping test");
        return;
    }

    let output = Command::new(&binary)
        .args(["diff", "--help"])
        .output()
        .expect("Failed to execute CLI");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("diff"));
}
