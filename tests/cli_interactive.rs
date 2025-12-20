use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_path(file_name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("todoapp-{nanos}-{file_name}"))
}

fn run_interactive(input: &str) -> std::process::Output {
    let exe = env!("CARGO_BIN_EXE_todo_cli");
    let store_path = temp_path("cli-interactive.json");

    let mut child = Command::new(exe)
        .env("TODOAPP_STORE_PATH", &store_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn interactive session");

    {
        let stdin = child.stdin.as_mut().expect("stdin");
        stdin
            .write_all(input.as_bytes())
            .expect("failed to write to stdin");
    }

    let output = child
        .wait_with_output()
        .expect("failed to read interactive output");

    std::fs::remove_file(&store_path).ok();
    output
}

#[test]
fn interactive_help_shows_usage() {
    let output = run_interactive("help\nexit\n");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage") || stdout.contains("USAGE"));
}

#[test]
fn interactive_question_mark_shows_usage() {
    let output = run_interactive("?\nexit\n");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage") || stdout.contains("USAGE"));
}

#[test]
fn interactive_invalid_command_prints_error() {
    let output = run_interactive("nope\nexit\n");
    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ERROR: invalid_input"));
}

#[test]
fn interactive_add_command_succeeds() {
    let output = run_interactive("add \"demo task\"\nexit\n");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Added task:"));
}
