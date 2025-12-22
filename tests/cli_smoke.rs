use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::json;
use todo_core::storage::SCHEMA_VERSION;

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

#[test]
fn cli_reports_invalid_config_and_uses_defaults() {
    let exe = env!("CARGO_BIN_EXE_todo_cli");
    let config_path = temp_path("cli-invalid-config.json");
    let store_path = temp_path("cli-config-store.json");

    std::fs::write(&config_path, "{ invalid json ").unwrap();

    let output = Command::new(exe)
        .args(["list", "today"])
        .env("TODOAPP_CONFIG_PATH", &config_path)
        .env("TODOAPP_STORE_PATH", &store_path)
        .output()
        .expect("failed to run list command with invalid config");

    std::fs::remove_file(&config_path).ok();
    std::fs::remove_file(&store_path).ok();

    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ERROR: invalid_data"));
}

#[test]
fn cli_config_override_alias_applies_without_mutating_config() {
    let exe = env!("CARGO_BIN_EXE_todo_cli");
    let config_path = temp_path("cli-override-config.json");
    let store_path = temp_path("cli-override-store.json");
    let initial_config = r#"{"theme":"vanilla","aliases":{}}"#;
    std::fs::write(&config_path, initial_config).unwrap();

    let missing_alias = Command::new(exe)
        .args(["ls"])
        .env("TODOAPP_CONFIG_PATH", &config_path)
        .env("TODOAPP_STORE_PATH", &store_path)
        .output()
        .expect("failed to run ls without override");

    assert!(!missing_alias.status.success());
    let stderr = String::from_utf8_lossy(&missing_alias.stderr);
    assert!(stderr.contains("invalid_input"));

    let override_output = Command::new(exe)
        .args(["ls", "--config-override=aliases.ls=list today"])
        .env("TODOAPP_CONFIG_PATH", &config_path)
        .env("TODOAPP_STORE_PATH", &store_path)
        .output()
        .expect("failed to run ls with override");

    assert!(override_output.status.success());
    let stderr = String::from_utf8_lossy(&override_output.stderr);
    assert!(stderr.trim().is_empty());

    let stored_config = std::fs::read_to_string(&config_path).unwrap();
    assert_eq!(stored_config, initial_config);

    std::fs::remove_file(&config_path).ok();
    std::fs::remove_file(&store_path).ok();
}

#[test]
fn cli_config_override_invalid_flag_reports_error_and_preserves_files() {
    let exe = env!("CARGO_BIN_EXE_todo_cli");
    let config_path = temp_path("cli-invalid-override-config.json");
    let store_path = temp_path("cli-invalid-override-store.json");
    let initial_config = r#"{"theme":null,"aliases":{}}"#;
    std::fs::write(&config_path, initial_config).unwrap();
    let initial_store = json!({
        "schema_version": SCHEMA_VERSION,
        "tasks": [],
        "focused_task_id": null,
    });
    std::fs::write(&store_path, serde_json::to_string_pretty(&initial_store).unwrap()).unwrap();

    let output = Command::new(exe)
        .args(["list", "today", "--config-override=aliases.=show"])
        .env("TODOAPP_CONFIG_PATH", &config_path)
        .env("TODOAPP_STORE_PATH", &store_path)
        .output()
        .expect("failed to run list command with invalid override");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ERROR: invalid_input"));
    assert!(stderr.contains("aliases override requires an alias name"));

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.trim().is_empty());

    let stored_config = std::fs::read_to_string(&config_path).unwrap();
    assert_eq!(stored_config, initial_config);

    let stored: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&store_path).unwrap()).unwrap();
    assert_eq!(stored, initial_store);

    std::fs::remove_file(&config_path).ok();
    std::fs::remove_file(&store_path).ok();
}

#[test]
fn cli_config_override_does_not_corrupt_store_schema_version() {
    let exe = env!("CARGO_BIN_EXE_todo_cli");
    let config_path = temp_path("cli-override-schema-config.json");
    let store_path = temp_path("cli-override-schema-store.json");
    let config_content = r#"{"theme":null,"aliases":{}}"#;
    std::fs::write(&config_path, config_content).unwrap();
    let initial_store = json!({
        "schema_version": SCHEMA_VERSION,
        "tasks": [],
        "focused_task_id": null,
    });
    std::fs::write(&store_path, serde_json::to_string_pretty(&initial_store).unwrap()).unwrap();

    let output = Command::new(exe)
        .args(["list", "backlog", "--config-override=aliases.backlog=list"])
        .env("TODOAPP_CONFIG_PATH", &config_path)
        .env("TODOAPP_STORE_PATH", &store_path)
        .output()
        .expect("failed to run list command with override");

    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.trim().is_empty());

    let stored: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&store_path).unwrap()).unwrap();
    assert_eq!(stored["schema_version"], json!(SCHEMA_VERSION));
    assert!(stored["tasks"].is_array());
    assert!(stored["focused_task_id"].is_null());

    std::fs::remove_file(&config_path).ok();
    std::fs::remove_file(&store_path).ok();
}

#[test]
fn cli_theme_override_emits_palette_code_and_preserves_config() {
    let exe = env!("CARGO_BIN_EXE_todo_cli");
    let config_path = temp_path("cli-theme-config.json");
    let store_path = temp_path("cli-theme-store.json");
    let initial_config = r#"{"theme":null,"aliases":{}}"#;
    std::fs::write(&config_path, initial_config).unwrap();

    let output = Command::new(exe)
        .args(["add", "theme task", "--config-override=theme=noir"])
        .env("TODOAPP_CONFIG_PATH", &config_path)
        .env("TODOAPP_STORE_PATH", &store_path)
        .output()
        .expect("failed to run add command with theme override");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\x1b[38;5;208m"));

    let stored_config = std::fs::read_to_string(&config_path).unwrap();
    assert_eq!(stored_config, initial_config);

    std::fs::remove_file(&config_path).ok();
    std::fs::remove_file(&store_path).ok();
}

#[test]
fn cli_alias_cycle_is_rejected_before_execution() {
    let exe = env!("CARGO_BIN_EXE_todo_cli");
    let config_path = temp_path("cli-alias-cycle-config.json");
    let store_path = temp_path("cli-alias-cycle-store.json");
    let config_content = r#"{"theme":null,"aliases":{"ls":"list today","list":"ls today"}}"#;
    std::fs::write(&config_path, config_content).unwrap();

    let output = Command::new(exe)
        .args(["list", "today"])
        .env("TODOAPP_CONFIG_PATH", &config_path)
        .env("TODOAPP_STORE_PATH", &store_path)
        .output()
        .expect("failed to run list command with cyclic alias");

    std::fs::remove_file(&config_path).ok();
    std::fs::remove_file(&store_path).ok();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ERROR: invalid_input"));
    assert!(stderr.contains("introduces a cycle"));
}

#[test]
fn cli_add_task_with_urgent_flag_sets_task() {
    let exe = env!("CARGO_BIN_EXE_todo_cli");
    let store_path = temp_path("cli-add-urgent.json");

    let output = Command::new(exe)
        .args(["add", "extra urgent", "--urgent"])
        .env("TODOAPP_STORE_PATH", &store_path)
        .output()
        .expect("failed to run add command with urgent flag");

    assert!(output.status.success());
    let stored: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&store_path).unwrap()).unwrap();
    std::fs::remove_file(&store_path).ok();

    let tasks = stored["tasks"].as_array().expect("tasks array");
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0]["urgent"], json!(true));
}

#[test]
fn cli_urgent_command_marks_and_clears_flag() {
    let exe = env!("CARGO_BIN_EXE_todo_cli");
    let store_path = temp_path("cli-urgent-command.json");
    let initial_store = json!({
        "schema_version": SCHEMA_VERSION,
        "focused_task_id": null,
        "tasks": [
            {
                "id": "task-urgent",
                "title": "urgent job",
                "status": "pending",
                "created_at": "2025-12-20T00:00:00Z",
                "scheduled_at": null,
                "urgent": false
            }
        ]
    });
    std::fs::write(&store_path, serde_json::to_string_pretty(&initial_store).unwrap()).unwrap();

    let mark_output = Command::new(exe)
        .args(["urgent", "task-urgent"])
        .env("TODOAPP_STORE_PATH", &store_path)
        .output()
        .expect("failed to run urgent command");

    assert!(mark_output.status.success());
    let marked: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&store_path).unwrap()).unwrap();
    assert_eq!(marked["tasks"][0]["urgent"], json!(true));

    let clear_output = Command::new(exe)
        .args(["urgent", "task-urgent", "--clear"])
        .env("TODOAPP_STORE_PATH", &store_path)
        .output()
        .expect("failed to run urgent command --clear");

    assert!(clear_output.status.success());
    let cleared: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&store_path).unwrap()).unwrap();

    std::fs::remove_file(&store_path).ok();

    assert_eq!(cleared["tasks"][0]["urgent"], json!(false));
}
