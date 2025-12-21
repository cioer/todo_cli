use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_path(file_name: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("todoapp-{nanos}-{file_name}"))
}

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

#[test]
fn cli_focus_sets_task_and_reports_errors() {
    let exe = env!("CARGO_BIN_EXE_todo_cli");
    let store_path = temp_path("cli-focus.json");

    let add_output = Command::new(exe)
        .args(["add", "demo"])
        .env("TODOAPP_STORE_PATH", &store_path)
        .output()
        .expect("failed to run add command");

    assert!(add_output.status.success());
    let add_stdout = String::from_utf8_lossy(&add_output.stdout);
    let id = add_stdout
        .trim()
        .rsplit('(')
        .next()
        .and_then(|value| value.strip_suffix(')'))
        .map(|value| value.trim())
        .expect("task id in output");

    let focus_output = Command::new(exe)
        .args(["focus", id])
        .env("TODOAPP_STORE_PATH", &store_path)
        .output()
        .expect("failed to run focus command");

    assert!(focus_output.status.success());
    let focus_stdout = String::from_utf8_lossy(&focus_output.stdout);
    assert!(focus_stdout.contains("Focused task:"));

    let missing_output = Command::new(exe)
        .args(["focus", "task-missing"])
        .env("TODOAPP_STORE_PATH", &store_path)
        .output()
        .expect("failed to run focus command for missing task");

    std::fs::remove_file(&store_path).ok();
    assert!(!missing_output.status.success());
    let stderr = String::from_utf8_lossy(&missing_output.stderr);
    assert!(stderr.contains("ERROR: invalid_input - task not found"));
}

#[test]
fn cli_focus_does_not_break_edit_flow() {
    let exe = env!("CARGO_BIN_EXE_todo_cli");
    let store_path = temp_path("cli-focus-edit.json");

    let add_output = Command::new(exe)
        .args(["add", "original"])
        .env("TODOAPP_STORE_PATH", &store_path)
        .output()
        .expect("failed to run add command");

    assert!(add_output.status.success());
    let add_stdout = String::from_utf8_lossy(&add_output.stdout);
    let id = add_stdout
        .trim()
        .rsplit('(')
        .next()
        .and_then(|value| value.strip_suffix(')'))
        .map(|value| value.trim())
        .expect("task id in output");

    let focus_output = Command::new(exe)
        .args(["focus", id])
        .env("TODOAPP_STORE_PATH", &store_path)
        .output()
        .expect("failed to run focus command");

    assert!(focus_output.status.success());

    let edit_output = Command::new(exe)
        .args(["edit", id, "updated"])
        .env("TODOAPP_STORE_PATH", &store_path)
        .output()
        .expect("failed to run edit command");

    assert!(edit_output.status.success());
    let stored: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&store_path).unwrap()).unwrap();
    std::fs::remove_file(&store_path).ok();

    assert_eq!(stored["tasks"][0]["title"], "updated");
}
