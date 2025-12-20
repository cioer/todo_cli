use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

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
fn done_command_marks_completed_and_records_history() {
    let exe = env!("CARGO_BIN_EXE_todo_cli");
    let store_path = temp_path("cli-done.json");

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
        .args(["done", "task-1", "ship it"])
        .env("TODOAPP_STORE_PATH", &store_path)
        .output()
        .expect("failed to run done command");

    assert!(output.status.success());

    let stored: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&store_path).unwrap()).unwrap();
    std::fs::remove_file(&store_path).ok();

    assert_eq!(stored["tasks"][0]["status"], "completed");
    assert!(stored["tasks"][0]["completed_at"].is_string());
    let history = stored["tasks"][0]["completion_history"]
        .as_array()
        .expect("history array");
    assert_eq!(history.len(), 1);
    assert_eq!(history[0]["message"], "ship it");
    assert!(history[0]["completed_at"].is_string());
    OffsetDateTime::parse(
        history[0]["completed_at"].as_str().expect("history completed_at string"),
        &Rfc3339,
    )
    .expect("history completed_at rfc3339");
}

#[test]
fn done_command_rejects_already_completed() {
    let exe = env!("CARGO_BIN_EXE_todo_cli");
    let store_path = temp_path("cli-done-completed.json");

    write_store(
        &store_path,
        serde_json::json!([
            {
                "id": "task-1",
                "title": "old",
                "status": "completed",
                "created_at": "2025-12-20T00:00:00Z",
                "scheduled_at": null,
                "completed_at": "2025-12-21T10:00:00Z",
                "completion_history": [
                    {
                        "message": "done",
                        "completed_at": "2025-12-21T10:00:00Z"
                    }
                ]
            }
        ]),
    );

    let output = Command::new(exe)
        .args(["done", "task-1"])
        .env("TODOAPP_STORE_PATH", &store_path)
        .output()
        .expect("failed to run done command");

    std::fs::remove_file(&store_path).ok();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ERROR: invalid_input"));
}

#[test]
fn done_command_reports_missing_id() {
    let exe = env!("CARGO_BIN_EXE_todo_cli");
    let store_path = temp_path("cli-done-missing.json");

    write_store(&store_path, serde_json::json!([]));

    let output = Command::new(exe)
        .args(["done", "task-1"])
        .env("TODOAPP_STORE_PATH", &store_path)
        .output()
        .expect("failed to run done command");

    std::fs::remove_file(&store_path).ok();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ERROR: invalid_input"));
}

#[test]
fn done_command_rejects_blank_message() {
    let exe = env!("CARGO_BIN_EXE_todo_cli");
    let store_path = temp_path("cli-done-blank-message.json");

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
        .args(["done", "task-1", "   "])
        .env("TODOAPP_STORE_PATH", &store_path)
        .output()
        .expect("failed to run done command");

    std::fs::remove_file(&store_path).ok();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ERROR: invalid_input"));
}

#[test]
fn done_command_plain_text_output() {
    let exe = env!("CARGO_BIN_EXE_todo_cli");
    let store_path = temp_path("cli-done-text.json");

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
        .args(["done", "task-1"])
        .env("TODOAPP_STORE_PATH", &store_path)
        .output()
        .expect("failed to run done command");

    std::fs::remove_file(&store_path).ok();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Completed task:"));
}

#[test]
fn done_command_json_includes_fields() {
    let exe = env!("CARGO_BIN_EXE_todo_cli");
    let store_path = temp_path("cli-done-json.json");

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
        .args(["--json", "done", "task-1", "finished"])
        .env("TODOAPP_STORE_PATH", &store_path)
        .output()
        .expect("failed to run done command");

    std::fs::remove_file(&store_path).ok();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("json output");

    assert_eq!(parsed["id"], "task-1");
    assert_eq!(parsed["title"], "old");
    assert_eq!(parsed["status"], "completed");
    assert_eq!(parsed["created_at"], "2025-12-20T00:00:00Z");
    assert_eq!(parsed["scheduled_at"], "2025-12-21T10:00:00Z");
    assert!(parsed["completed_at"].is_string());
    OffsetDateTime::parse(
        parsed["completed_at"].as_str().expect("completed_at string"),
        &Rfc3339,
    )
    .expect("completed_at rfc3339");
    let history = parsed["completion_history"]
        .as_array()
        .expect("history array");
    assert_eq!(history.len(), 1);
    assert_eq!(history[0]["message"], "finished");
    assert!(history[0]["completed_at"].is_string());
    OffsetDateTime::parse(
        history[0]["completed_at"].as_str().expect("history completed_at string"),
        &Rfc3339,
    )
    .expect("history completed_at rfc3339");
}


