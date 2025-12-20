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
        "schema_version": 2,
        "tasks": tasks
    });
    std::fs::write(path, serde_json::to_string_pretty(&content).unwrap()).unwrap();
}

#[test]
fn edit_command_updates_title() {
    let exe = env!("CARGO_BIN_EXE_todo_cli");
    let store_path = temp_path("cli-edit.json");

    write_store(
        &store_path,
        serde_json::json!([
            {
                "id": "task-1",
                "title": "old",
                "status": "pending",
                "created_at": "2025-12-20T00:00:00Z",
                "scheduled_at": null
            }
        ]),
    );

    let output = Command::new(exe)
        .args(["edit", "task-1", "new title"])
        .env("TODOAPP_STORE_PATH", &store_path)
        .output()
        .expect("failed to run edit command");

    assert!(output.status.success());

    let stored: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&store_path).unwrap()).unwrap();
    std::fs::remove_file(&store_path).ok();

    assert_eq!(stored["tasks"][0]["title"], "new title");
}

#[test]
fn delete_command_removes_task() {
    let exe = env!("CARGO_BIN_EXE_todo_cli");
    let store_path = temp_path("cli-delete.json");

    write_store(
        &store_path,
        serde_json::json!([
            {
                "id": "task-1",
                "title": "old",
                "status": "pending",
                "created_at": "2025-12-20T00:00:00Z",
                "scheduled_at": null
            }
        ]),
    );

    let output = Command::new(exe)
        .args(["delete", "task-1"])
        .env("TODOAPP_STORE_PATH", &store_path)
        .output()
        .expect("failed to run delete command");

    assert!(output.status.success());

    let stored: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&store_path).unwrap()).unwrap();
    std::fs::remove_file(&store_path).ok();

    assert!(stored["tasks"].as_array().unwrap().is_empty());
}

#[test]
fn edit_command_reports_missing_id() {
    let exe = env!("CARGO_BIN_EXE_todo_cli");
    let store_path = temp_path("cli-edit-missing.json");

    write_store(&store_path, serde_json::json!([]));

    let output = Command::new(exe)
        .args(["edit", "task-1", "new title"])
        .env("TODOAPP_STORE_PATH", &store_path)
        .output()
        .expect("failed to run edit command");

    std::fs::remove_file(&store_path).ok();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ERROR: invalid_input"));
}

#[test]
fn delete_command_reports_missing_id() {
    let exe = env!("CARGO_BIN_EXE_todo_cli");
    let store_path = temp_path("cli-delete-missing.json");

    write_store(&store_path, serde_json::json!([]));

    let output = Command::new(exe)
        .args(["delete", "task-1"])
        .env("TODOAPP_STORE_PATH", &store_path)
        .output()
        .expect("failed to run delete command");

    std::fs::remove_file(&store_path).ok();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ERROR: invalid_input"));
}

#[test]
fn edit_command_plain_text_output() {
    let exe = env!("CARGO_BIN_EXE_todo_cli");
    let store_path = temp_path("cli-edit-text.json");

    write_store(
        &store_path,
        serde_json::json!([
            {
                "id": "task-1",
                "title": "old",
                "status": "pending",
                "created_at": "2025-12-20T00:00:00Z",
                "scheduled_at": null
            }
        ]),
    );

    let output = Command::new(exe)
        .args(["edit", "task-1", "new title"])
        .env("TODOAPP_STORE_PATH", &store_path)
        .output()
        .expect("failed to run edit command");

    std::fs::remove_file(&store_path).ok();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Updated task:"));
}

#[test]
fn delete_command_plain_text_output() {
    let exe = env!("CARGO_BIN_EXE_todo_cli");
    let store_path = temp_path("cli-delete-text.json");

    write_store(
        &store_path,
        serde_json::json!([
            {
                "id": "task-1",
                "title": "old",
                "status": "pending",
                "created_at": "2025-12-20T00:00:00Z",
                "scheduled_at": null
            }
        ]),
    );

    let output = Command::new(exe)
        .args(["delete", "task-1"])
        .env("TODOAPP_STORE_PATH", &store_path)
        .output()
        .expect("failed to run delete command");

    std::fs::remove_file(&store_path).ok();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Deleted task:"));
}

#[test]
fn edit_command_json_includes_fields() {
    let exe = env!("CARGO_BIN_EXE_todo_cli");
    let store_path = temp_path("cli-edit-json.json");

    write_store(
        &store_path,
        serde_json::json!([
            {
                "id": "task-1",
                "title": "old",
                "status": "pending",
                "created_at": "2025-12-20T00:00:00Z",
                "scheduled_at": "2025-12-21T10:00:00Z"
            }
        ]),
    );

    let output = Command::new(exe)
        .args(["--json", "edit", "task-1", "new title"])
        .env("TODOAPP_STORE_PATH", &store_path)
        .output()
        .expect("failed to run edit command");

    std::fs::remove_file(&store_path).ok();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("json output");

    assert_eq!(parsed["id"], "task-1");
    assert_eq!(parsed["title"], "new title");
    assert_eq!(parsed["status"], "pending");
    assert_eq!(parsed["created_at"], "2025-12-20T00:00:00Z");
    assert_eq!(parsed["scheduled_at"], "2025-12-21T10:00:00Z");
}

#[test]
fn delete_command_json_includes_fields() {
    let exe = env!("CARGO_BIN_EXE_todo_cli");
    let store_path = temp_path("cli-delete-json.json");

    write_store(
        &store_path,
        serde_json::json!([
            {
                "id": "task-1",
                "title": "old",
                "status": "pending",
                "created_at": "2025-12-20T00:00:00Z",
                "scheduled_at": "2025-12-21T10:00:00Z"
            }
        ]),
    );

    let output = Command::new(exe)
        .args(["--json", "delete", "task-1"])
        .env("TODOAPP_STORE_PATH", &store_path)
        .output()
        .expect("failed to run delete command");

    std::fs::remove_file(&store_path).ok();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("json output");

    assert_eq!(parsed["id"], "task-1");
    assert_eq!(parsed["title"], "old");
    assert_eq!(parsed["status"], "pending");
    assert_eq!(parsed["created_at"], "2025-12-20T00:00:00Z");
    assert_eq!(parsed["scheduled_at"], "2025-12-21T10:00:00Z");
}
