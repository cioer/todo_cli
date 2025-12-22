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

fn write_store(path: &PathBuf, tasks: serde_json::Value) {
    let content = serde_json::json!({
        "schema_version": 5,
        "tasks": tasks
    });
    std::fs::write(path, serde_json::to_string_pretty(&content).unwrap()).unwrap();
}

#[test]
fn show_command_plain_text_outputs_task() {
    let exe = env!("CARGO_BIN_EXE_todo_cli");
    let store_path = temp_path("cli-show-text.json");

    write_store(
        &store_path,
        serde_json::json!([
            {
                "id": "task-1",
                "title": "show me",
                "status": "pending",
                "created_at": "2025-12-20T00:00:00Z",
                "scheduled_at": null,
                "urgent": false
            }
        ]),
    );

    let output = Command::new(exe)
        .args(["show", "task-1"])
        .env("TODOAPP_STORE_PATH", &store_path)
        .output()
        .expect("failed to run show command");

    std::fs::remove_file(&store_path).ok();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("task-1"));
    assert!(stdout.contains("show me"));
}

#[test]
fn show_command_json_outputs_task() {
    let exe = env!("CARGO_BIN_EXE_todo_cli");
    let store_path = temp_path("cli-show-json.json");

    write_store(
        &store_path,
        serde_json::json!([
            {
                "id": "task-1",
                "title": "show me",
                "status": "pending",
                "created_at": "2025-12-20T00:00:00Z",
                "scheduled_at": null,
                "urgent": false
            }
        ]),
    );

    let output = Command::new(exe)
        .args(["--json", "show", "task-1"])
        .env("TODOAPP_STORE_PATH", &store_path)
        .output()
        .expect("failed to run show command");

    std::fs::remove_file(&store_path).ok();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("json output");

    assert_eq!(parsed["id"], "task-1");
    assert_eq!(parsed["title"], "show me");
    assert_eq!(parsed["status"], "pending");
    assert_eq!(parsed["created_at"], "2025-12-20T00:00:00Z");
    assert!(parsed["scheduled_at"].is_null());
}

#[test]
fn show_command_reports_missing_task() {
    let exe = env!("CARGO_BIN_EXE_todo_cli");
    let store_path = temp_path("cli-show-missing.json");

    write_store(&store_path, serde_json::json!([]));

    let output = Command::new(exe)
        .args(["show", "task-1"])
        .env("TODOAPP_STORE_PATH", &store_path)
        .output()
        .expect("failed to run show command");

    std::fs::remove_file(&store_path).ok();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ERROR: invalid_input - task not found"));
}
