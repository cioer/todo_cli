use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_path(file_name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("todoapp-{nanos}-{file_name}"))
}

#[test]
fn add_command_succeeds() {
    let exe = env!("CARGO_BIN_EXE_todo_cli");
    let store_path = temp_path("cli-add.json");
    let output = Command::new(exe)
        .args(["add", "demo task"])
        .env("TODOAPP_STORE_PATH", &store_path)
        .output()
        .expect("failed to run add command");

    std::fs::remove_file(&store_path).ok();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Added task:"));
}

#[test]
fn add_command_rejects_missing_title() {
    let exe = env!("CARGO_BIN_EXE_todo_cli");
    let store_path = temp_path("cli-add-missing.json");
    let output = Command::new(exe)
        .args(["add"])
        .env("TODOAPP_STORE_PATH", &store_path)
        .output()
        .expect("failed to run add command");

    std::fs::remove_file(&store_path).ok();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ERROR: invalid_input"));
}
