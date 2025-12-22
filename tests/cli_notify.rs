use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use time::format_description::well_known::Rfc3339;
use time::{Duration, OffsetDateTime};

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
fn notify_command_plain_text_outputs_notified_tasks() {
    let exe = env!("CARGO_BIN_EXE_todo_cli");
    let store_path = temp_path("cli-notify-text.json");
    let now = OffsetDateTime::now_utc();
    let past = (now - Duration::days(1)).format(&Rfc3339).unwrap();

    write_store(
        &store_path,
        serde_json::json!([
            {
                "id": "task-1",
                "title": "overdue",
                "status": "pending",
                "created_at": "2025-12-20T00:00:00Z",
                "scheduled_at": past,
                "urgent": false
            },
            {
                "id": "task-2",
                "title": "urgent",
                "status": "pending",
                "created_at": "2025-12-20T00:00:00Z",
                "scheduled_at": null,
                "urgent": true
            },
            {
                "id": "task-3",
                "title": "completed",
                "status": "completed",
                "created_at": "2025-12-20T00:00:00Z",
                "scheduled_at": past,
                "urgent": true,
                "completed_at": "2025-12-21T10:00:00Z",
                "completion_history": []
            }
        ]),
    );

    let output = Command::new(exe)
        .args(["notify"])
        .env("TODOAPP_STORE_PATH", &store_path)
        .env("TODOAPP_DISABLE_NOTIFICATIONS", "1")
        .output()
        .expect("failed to run notify command");

    std::fs::remove_file(&store_path).ok();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Notified task: overdue (task-1)"));
    assert!(stdout.contains("Notified task: urgent (task-2)"));
    assert!(!stdout.contains("task-3"));
}

#[test]
fn notify_command_json_outputs_tasks() {
    let exe = env!("CARGO_BIN_EXE_todo_cli");
    let store_path = temp_path("cli-notify-json.json");
    let now = OffsetDateTime::now_utc();
    let past = (now - Duration::days(1)).format(&Rfc3339).unwrap();

    write_store(
        &store_path,
        serde_json::json!([
            {
                "id": "task-1",
                "title": "overdue",
                "status": "pending",
                "created_at": "2025-12-20T00:00:00Z",
                "scheduled_at": past,
                "urgent": false
            },
            {
                "id": "task-2",
                "title": "urgent",
                "status": "pending",
                "created_at": "2025-12-20T00:00:00Z",
                "scheduled_at": null,
                "urgent": true
            }
        ]),
    );

    let output = Command::new(exe)
        .args(["--json", "notify"])
        .env("TODOAPP_STORE_PATH", &store_path)
        .env("TODOAPP_DISABLE_NOTIFICATIONS", "1")
        .output()
        .expect("failed to run notify command");

    std::fs::remove_file(&store_path).ok();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("json output");
    let tasks = parsed.as_array().expect("json array");
    assert_eq!(tasks.len(), 2);
    assert_eq!(tasks[0]["id"], "task-1");
    assert_eq!(tasks[1]["id"], "task-2");
}
