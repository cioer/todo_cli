use std::process::Command;

#[test]
fn cli_smoke_help() {
    let exe = env!("CARGO_BIN_EXE_todo_cli");
    let output = Command::new(exe)
        .arg("--help")
        .output()
        .expect("failed to run todo_cli --help");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.trim().is_empty());
}
