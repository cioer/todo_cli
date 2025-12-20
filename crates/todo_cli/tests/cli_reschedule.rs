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
fn reschedule_plain_text_output_includes_datetime() {
    let exe = env!("CARGO_BIN_EXE_todo_cli");
    let store_path = temp_path("cli-reschedule-plain.json");

    let content = serde_json::json!({
        "schema_version": 3,
        "tasks": [
            {
                "id": "task-1",
                "title": "demo",
                "status": "pending",
                "created_at": "2025-12-20T00:00:00Z",
                "scheduled_at": "2025-12-20T09:00:00Z"
            }
        ]
    });

    std::fs::write(&store_path, serde_json::to_string_pretty(&content).unwrap()).unwrap();

    let output = Command::new(exe)
        .args(["reschedule", "task-1", "2025-12-21T09:00:00Z"])
        .env("TODOAPP_STORE_PATH", &store_path)
        .output()
        .expect("failed to run reschedule command");

    std::fs::remove_file(&store_path).ok();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Rescheduled task: demo (task-1) at 2025-12-21T09:00:00Z"));
}

#[test]
fn reschedule_updates_task_and_persists() {
    let exe = env!("CARGO_BIN_EXE_todo_cli");
    let store_path = temp_path("cli-reschedule.json");

    let content = serde_json::json!({
        "schema_version": 3,
        "tasks": [
            {
                "id": "task-1",
                "title": "demo",
                "status": "pending",
                "created_at": "2025-12-20T00:00:00Z",
                "scheduled_at": "2025-12-20T09:00:00Z"
            }
        ]
    });

    std::fs::write(&store_path, serde_json::to_string_pretty(&content).unwrap()).unwrap();

    let output = Command::new(exe)
        .args(["--json", "reschedule", "task-1", "2025-12-21T09:00:00Z"])
        .env("TODOAPP_STORE_PATH", &store_path)
        .output()
        .expect("failed to run reschedule command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("json output");
    assert_eq!(parsed["scheduled_at"], "2025-12-21T09:00:00Z");

    let stored: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&store_path).unwrap())
            .expect("stored json");
    assert_eq!(
        stored["tasks"][0]["scheduled_at"],
        serde_json::Value::String("2025-12-21T09:00:00Z".to_string())
    );

    std::fs::remove_file(&store_path).ok();
}

#[test]
fn reschedule_rejects_invalid_datetime() {
    let exe = env!("CARGO_BIN_EXE_todo_cli");
    let store_path = temp_path("cli-reschedule-invalid.json");

    let content = serde_json::json!({
        "schema_version": 3,
        "tasks": [
            {
                "id": "task-1",
                "title": "demo",
                "status": "pending",
                "created_at": "2025-12-20T00:00:00Z",
                "scheduled_at": "2025-12-20T09:00:00Z"
            }
        ]
    });

    std::fs::write(&store_path, serde_json::to_string_pretty(&content).unwrap()).unwrap();

    let output = Command::new(exe)
        .args(["reschedule", "task-1", "bad-date"])
        .env("TODOAPP_STORE_PATH", &store_path)
        .output()
        .expect("failed to run reschedule command");

    std::fs::remove_file(&store_path).ok();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ERROR: invalid_input"));
}

#[test]
fn reschedule_rejects_missing_id() {
    let exe = env!("CARGO_BIN_EXE_todo_cli");
    let store_path = temp_path("cli-reschedule-missing-id.json");

    let content = serde_json::json!({
        "schema_version": 3,
        "tasks": [
            {
                "id": "task-1",
                "title": "demo",
                "status": "pending",
                "created_at": "2025-12-20T00:00:00Z",
                "scheduled_at": "2025-12-20T09:00:00Z"
            }
        ]
    });

    std::fs::write(&store_path, serde_json::to_string_pretty(&content).unwrap()).unwrap();

    let output = Command::new(exe)
        .args(["reschedule"])
        .env("TODOAPP_STORE_PATH", &store_path)
        .output()
        .expect("failed to run reschedule command");

    std::fs::remove_file(&store_path).ok();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ERROR: invalid_input"));
}

#[test]
fn reschedule_rejects_unknown_id() {
    let exe = env!("CARGO_BIN_EXE_todo_cli");
    let store_path = temp_path("cli-reschedule-missing.json");

    let content = serde_json::json!({
        "schema_version": 3,
        "tasks": [
            {
                "id": "task-1",
                "title": "demo",
                "status": "pending",
                "created_at": "2025-12-20T00:00:00Z",
                "scheduled_at": "2025-12-20T09:00:00Z"
            }
        ]
    });

    std::fs::write(&store_path, serde_json::to_string_pretty(&content).unwrap()).unwrap();

    let output = Command::new(exe)
        .args(["reschedule", "task-2", "2025-12-21T09:00:00Z"])
        .env("TODOAPP_STORE_PATH", &store_path)
        .output()
        .expect("failed to run reschedule command");

    std::fs::remove_file(&store_path).ok();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ERROR: invalid_input"));
    assert!(stderr.contains("task not found"));
}

#[test]
fn reschedule_rejects_unscheduled_task() {
    let exe = env!("CARGO_BIN_EXE_todo_cli");
    let store_path = temp_path("cli-reschedule-unscheduled.json");

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
        .args(["reschedule", "task-1", "2025-12-21T09:00:00Z"])
        .env("TODOAPP_STORE_PATH", &store_path)
        .output()
        .expect("failed to run reschedule command");

    std::fs::remove_file(&store_path).ok();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ERROR: invalid_input"));
    assert!(stderr.contains("task is not scheduled"));
}

#[test]
fn reschedule_updates_list_filters() {
    let exe = env!("CARGO_BIN_EXE_todo_cli");
    let store_path = temp_path("cli-reschedule-list.json");
    let (today, tomorrow) = local_now_strings();

    let content = serde_json::json!({
        "schema_version": 3,
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
        .args(["reschedule", "task-2", &today])
        .env("TODOAPP_STORE_PATH", &store_path)
        .output()
        .expect("failed to run reschedule command");

    assert!(output.status.success());

    let today_output = Command::new(exe)
        .args(["list", "today"])
        .env("TODOAPP_STORE_PATH", &store_path)
        .output()
        .expect("failed to run list today command");

    let backlog_output = Command::new(exe)
        .args(["--json", "list", "backlog"])
        .env("TODOAPP_STORE_PATH", &store_path)
        .output()
        .expect("failed to run list backlog command");

    std::fs::remove_file(&store_path).ok();
    assert!(today_output.status.success());
    let today_stdout = String::from_utf8_lossy(&today_output.stdout);
    assert!(today_stdout.contains("today task"));
    assert!(today_stdout.contains("future task"));

    assert!(backlog_output.status.success());
    let backlog_stdout = String::from_utf8_lossy(&backlog_output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&backlog_stdout).expect("json output");
    let tasks = parsed.as_array().expect("json array");
    assert!(tasks.is_empty());
}
