//! Basic CLI tests for confers-cli

/// Helper to run cargo in the project directory
fn run_cargo(args: &[&str]) -> std::process::Command {
    // Use cargo from the current Rust toolchain
    let mut cmd = std::process::Command::new("cargo");
    cmd.args(args);
    cmd
}

#[test]
fn test_cli_compiles() {
    // Test that the CLI compiles successfully
    let output = run_cargo(&["build", "-p", "confers-cli"])
        .output()
        .expect("Failed to execute cargo build");

    assert!(
        output.status.success(),
        "CLI failed to compile: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
