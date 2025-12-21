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
fn schedule_updates_task_and_persists() {
    let exe = env!("CARGO_BIN_EXE_todo_cli");
    let store_path = temp_path("cli-schedule.json");

    let content = serde_json::json!({
        "schema_version": 3,
        "tasks": [
            {
                "id": "task-1",
                "title": "demo",
                "status": "pending",
                "created_at": "2025-12-20T00:00:00Z",
                "scheduled_at": null
            }
        ]
    });

    std::fs::write(&store_path, serde_json::to_string_pretty(&content).unwrap()).unwrap();

    let output = Command::new(exe)
        .args(["--json", "schedule", "task-1", "2025-12-21T09:00:00Z"])
        .env("TODOAPP_STORE_PATH", &store_path)
        .output()
        .expect("failed to run schedule command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("json output");
    assert_eq!(parsed["scheduled_at"], "2025-12-21T09:00:00Z");

    let stored: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&store_path).unwrap()).expect("stored json");
    assert_eq!(
        stored["tasks"][0]["scheduled_at"],
        serde_json::Value::String("2025-12-21T09:00:00Z".to_string())
    );

    std::fs::remove_file(&store_path).ok();
}

#[test]
fn schedule_rejects_invalid_datetime() {
    let exe = env!("CARGO_BIN_EXE_todo_cli");
    let store_path = temp_path("cli-schedule-invalid.json");

    let content = serde_json::json!({
        "schema_version": 3,
        "tasks": [
            {
                "id": "task-1",
                "title": "demo",
                "status": "pending",
                "created_at": "2025-12-20T00:00:00Z",
                "scheduled_at": null
            }
        ]
    });

    std::fs::write(&store_path, serde_json::to_string_pretty(&content).unwrap()).unwrap();

    let output = Command::new(exe)
        .args(["schedule", "task-1", "bad-date"])
        .env("TODOAPP_STORE_PATH", &store_path)
        .output()
        .expect("failed to run schedule command");

    std::fs::remove_file(&store_path).ok();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ERROR: invalid_input"));
}

#[test]
fn schedule_rejects_missing_id() {
    let exe = env!("CARGO_BIN_EXE_todo_cli");
    let store_path = temp_path("cli-schedule-missing-id.json");

    let content = serde_json::json!({
        "schema_version": 3,
        "tasks": [
            {
                "id": "task-1",
                "title": "demo",
                "status": "pending",
                "created_at": "2025-12-20T00:00:00Z",
                "scheduled_at": null
            }
        ]
    });

    std::fs::write(&store_path, serde_json::to_string_pretty(&content).unwrap()).unwrap();

    let output = Command::new(exe)
        .args(["schedule"])
        .env("TODOAPP_STORE_PATH", &store_path)
        .output()
        .expect("failed to run schedule command");

    std::fs::remove_file(&store_path).ok();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ERROR: invalid_input"));
}

#[test]
fn schedule_rejects_unknown_id() {
    let exe = env!("CARGO_BIN_EXE_todo_cli");
    let store_path = temp_path("cli-schedule-missing.json");

    let content = serde_json::json!({
        "schema_version": 3,
        "tasks": [
            {
                "id": "task-1",
                "title": "demo",
                "status": "pending",
                "created_at": "2025-12-20T00:00:00Z",
                "scheduled_at": null
            }
        ]
    });

    std::fs::write(&store_path, serde_json::to_string_pretty(&content).unwrap()).unwrap();

    let output = Command::new(exe)
        .args(["schedule", "task-2", "2025-12-21T09:00:00Z"])
        .env("TODOAPP_STORE_PATH", &store_path)
        .output()
        .expect("failed to run schedule command");

    std::fs::remove_file(&store_path).ok();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ERROR: invalid_input"));
    assert!(stderr.contains("task not found"));
}
