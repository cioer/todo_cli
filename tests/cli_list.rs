use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use time::format_description::well_known::Rfc3339;
use time::{Duration, OffsetDateTime, UtcOffset};

fn temp_path(file_name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("todoapp-{nanos}-{file_name}"))
}

fn local_now_strings() -> (String, String) {
    let offset = UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC);
    let now = OffsetDateTime::now_utc().to_offset(offset);
    let tomorrow = now + Duration::days(1);
    (
        now.format(&Rfc3339).expect("format today"),
        tomorrow.format(&Rfc3339).expect("format tomorrow"),
    )
}

#[test]
fn list_today_plain_text_filters_tasks() {
    let exe = env!("CARGO_BIN_EXE_todo_cli");
    let store_path = temp_path("cli-list-today.json");
    let (today, tomorrow) = local_now_strings();

    let content = serde_json::json!({
        "schema_version": 2,
        "tasks": [
            {
                "id": "task-1",
                "title": "today task",
                "status": "pending",
                "created_at": "2025-12-20T00:00:00Z",
                "scheduled_at": today
            },
            {
                "id": "task-2",
                "title": "future task",
                "status": "pending",
                "created_at": "2025-12-20T00:00:00Z",
                "scheduled_at": tomorrow
            }
        ]
    });

    std::fs::write(&store_path, serde_json::to_string_pretty(&content).unwrap()).unwrap();

    let output = Command::new(exe)
        .args(["list", "today"])
        .env("TODOAPP_STORE_PATH", &store_path)
        .output()
        .expect("failed to run list today command");

    std::fs::remove_file(&store_path).ok();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("today task"));
    assert!(!stdout.contains("future task"));
}

#[test]
fn list_backlog_json_filters_tasks() {
    let exe = env!("CARGO_BIN_EXE_todo_cli");
    let store_path = temp_path("cli-list-backlog.json");
    let (today, tomorrow) = local_now_strings();

    let content = serde_json::json!({
        "schema_version": 2,
        "tasks": [
            {
                "id": "task-1",
                "title": "today task",
                "status": "pending",
                "created_at": "2025-12-20T00:00:00Z",
                "scheduled_at": today
            },
            {
                "id": "task-2",
                "title": "future task",
                "status": "pending",
                "created_at": "2025-12-20T00:00:00Z",
                "scheduled_at": tomorrow
            }
        ]
    });

    std::fs::write(&store_path, serde_json::to_string_pretty(&content).unwrap()).unwrap();

    let output = Command::new(exe)
        .args(["--json", "list", "backlog"])
        .env("TODOAPP_STORE_PATH", &store_path)
        .output()
        .expect("failed to run list backlog command");

    std::fs::remove_file(&store_path).ok();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("json output");
    let tasks = parsed.as_array().expect("json array");
    assert_eq!(tasks.len(), 1);
    let task = &tasks[0];
    assert_eq!(task["id"], "task-2");
    assert_eq!(task["title"], "future task");
    assert_eq!(task["status"], "pending");
    assert_eq!(task["created_at"], "2025-12-20T00:00:00Z");
    assert_eq!(task["scheduled_at"], tomorrow);
}

#[test]
fn list_reports_invalid_scheduled_at() {
    let exe = env!("CARGO_BIN_EXE_todo_cli");
    let store_path = temp_path("cli-list-invalid.json");

    let content = serde_json::json!({
        "schema_version": 2,
        "tasks": [
            {
                "id": "task-1",
                "title": "bad",
                "status": "pending",
                "created_at": "2025-12-20T00:00:00Z",
                "scheduled_at": "not-a-date"
            }
        ]
    });

    std::fs::write(&store_path, serde_json::to_string_pretty(&content).unwrap()).unwrap();

    let output = Command::new(exe)
        .args(["list", "today"])
        .env("TODOAPP_STORE_PATH", &store_path)
        .output()
        .expect("failed to run list today command");

    std::fs::remove_file(&store_path).ok();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ERROR: invalid_data"));
}

#[test]
fn list_today_places_focused_task_first_with_prefix() {
    let exe = env!("CARGO_BIN_EXE_todo_cli");
    let store_path = temp_path("cli-list-focus.json");
    let (today, tomorrow) = local_now_strings();

    let content = serde_json::json!({
        "schema_version": 4,
        "focused_task_id": "task-2",
        "tasks": [
            {
                "id": "task-1",
                "title": "today task",
                "status": "pending",
                "created_at": "2025-12-20T00:00:00Z",
                "scheduled_at": today
            },
            {
                "id": "task-2",
                "title": "focused task",
                "status": "pending",
                "created_at": "2025-12-20T00:00:00Z",
                "scheduled_at": today
            },
            {
                "id": "task-3",
                "title": "future task",
                "status": "pending",
                "created_at": "2025-12-20T00:00:00Z",
                "scheduled_at": tomorrow
            }
        ]
    });

    std::fs::write(&store_path, serde_json::to_string_pretty(&content).unwrap()).unwrap();

    let output = Command::new(exe)
        .args(["list", "today"])
        .env("TODOAPP_STORE_PATH", &store_path)
        .output()
        .expect("failed to run list today command");

    std::fs::remove_file(&store_path).ok();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut lines = stdout.lines();
    let first_line = lines.next().unwrap_or("");
    assert!(first_line.starts_with("[FOCUS] "));
    assert!(first_line.contains("task-2"));
    assert!(stdout.contains("today task"));
}

#[test]
fn list_today_does_not_show_focus_prefix_when_focused_task_missing() {
    let exe = env!("CARGO_BIN_EXE_todo_cli");
    let store_path = temp_path("cli-list-focus-missing.json");
    let (today, tomorrow) = local_now_strings();

    let content = serde_json::json!({
        "schema_version": 4,
        "focused_task_id": "task-2",
        "tasks": [
            {
                "id": "task-1",
                "title": "today task",
                "status": "pending",
                "created_at": "2025-12-20T00:00:00Z",
                "scheduled_at": today
            },
            {
                "id": "task-2",
                "title": "future task",
                "status": "pending",
                "created_at": "2025-12-20T00:00:00Z",
                "scheduled_at": tomorrow
            }
        ]
    });

    std::fs::write(&store_path, serde_json::to_string_pretty(&content).unwrap()).unwrap();

    let output = Command::new(exe)
        .args(["list", "today"])
        .env("TODOAPP_STORE_PATH", &store_path)
        .output()
        .expect("failed to run list today command");

    std::fs::remove_file(&store_path).ok();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("[FOCUS]"));
    assert!(stdout.contains("today task"));
}
