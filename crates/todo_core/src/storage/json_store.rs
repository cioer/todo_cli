use crate::error::AppError;
use crate::model::Task;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const SCHEMA_VERSION: u32 = 5;
const STORE_FILE_NAME: &str = "tasks.json";

#[derive(Debug, Serialize, Deserialize)]
struct StoredTasks {
    schema_version: u32,
    tasks: Vec<Task>,
    #[serde(default)]
    focused_task_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskState {
    pub tasks: Vec<Task>,
    pub focused_task_id: Option<String>,
}

pub fn store_path() -> Result<PathBuf, AppError> {
    if let Ok(path) = std::env::var("TODOAPP_STORE_PATH")
        && !path.trim().is_empty()
    {
        return Ok(PathBuf::from(path));
    }

    if cfg!(windows) {
        let appdata =
            std::env::var("APPDATA").map_err(|_| AppError::invalid_data("APPDATA is not set"))?;
        Ok(PathBuf::from(appdata).join("todoapp").join(STORE_FILE_NAME))
    } else {
        let home = std::env::var("HOME").map_err(|_| AppError::invalid_data("HOME is not set"))?;
        Ok(PathBuf::from(home)
            .join(".config")
            .join("todoapp")
            .join(STORE_FILE_NAME))
    }
}

pub fn load_tasks(path: &Path) -> Result<Vec<Task>, AppError> {
    Ok(load_state(path)?.tasks)
}

pub fn load_state(path: &Path) -> Result<TaskState, AppError> {
    if !path.exists() {
        return Ok(TaskState {
            tasks: Vec::new(),
            focused_task_id: None,
        });
    }

    let content = std::fs::read_to_string(path).map_err(|err| AppError::io(err.to_string()))?;
    let stored: StoredTasks =
        serde_json::from_str(&content).map_err(|err| AppError::invalid_data(err.to_string()))?;

    if !(1..=SCHEMA_VERSION).contains(&stored.schema_version) {
        return Err(AppError::invalid_data("schema_version mismatch"));
    }

    if let Some(focused_task_id) = stored.focused_task_id.as_deref() {
        let exists = stored.tasks.iter().any(|task| task.id == focused_task_id);
        if !exists {
            return Err(AppError::invalid_data("focused_task_id not found"));
        }
    }

    Ok(TaskState {
        tasks: stored.tasks,
        focused_task_id: stored.focused_task_id,
    })
}

pub fn save_tasks(path: &Path, tasks: &[Task]) -> Result<(), AppError> {
    let focused_task_id = if path.exists() {
        load_state(path)?.focused_task_id
    } else {
        None
    };
    let state = TaskState {
        tasks: tasks.to_vec(),
        focused_task_id,
    };
    save_state(path, &state)
}

pub fn save_state(path: &Path, state: &TaskState) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| AppError::io(err.to_string()))?;
    }

    let stored = StoredTasks {
        schema_version: SCHEMA_VERSION,
        tasks: state.tasks.to_vec(),
        focused_task_id: state.focused_task_id.clone(),
    };
    let content = serde_json::to_string_pretty(&stored)
        .map_err(|err| AppError::invalid_data(err.to_string()))?;
    std::fs::write(path, content).map_err(|err| AppError::io(err.to_string()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let permissions = std::fs::Permissions::from_mode(0o600);
        std::fs::set_permissions(path, permissions).map_err(|err| AppError::io(err.to_string()))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{SCHEMA_VERSION, TaskState, load_state, load_tasks, save_state, save_tasks};
    use crate::model::{Task, TaskStatus};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_path(file_name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("todoapp-{nanos}-{file_name}"))
    }

    #[test]
    fn save_and_load_round_trip() {
        let path = temp_path("tasks.json");
        let task = Task {
            id: "task-1".to_string(),
            title: "demo".to_string(),
            status: TaskStatus::Pending,
            created_at: "2025-12-20T00:00:00Z".to_string(),
            scheduled_at: None,
            completed_at: None,
            completion_history: Vec::new(),
            urgent: false,
        };

        save_tasks(&path, std::slice::from_ref(&task)).unwrap();
        let loaded = load_tasks(&path).unwrap();
        fs::remove_file(&path).ok();

        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0], task);
    }

    #[test]
    fn save_and_load_state_preserves_focus() {
        let path = temp_path("state.json");
        let task = Task {
            id: "task-1".to_string(),
            title: "demo".to_string(),
            status: TaskStatus::Pending,
            created_at: "2025-12-20T00:00:00Z".to_string(),
            scheduled_at: None,
            completed_at: None,
            completion_history: Vec::new(),
            urgent: false,
        };
        let state = TaskState {
            tasks: vec![task.clone()],
            focused_task_id: Some(task.id.clone()),
        };

        save_state(&path, &state).unwrap();
        let loaded = load_state(&path).unwrap();
        fs::remove_file(&path).ok();

        assert_eq!(loaded.tasks.len(), 1);
        assert_eq!(loaded.tasks[0], task);
        assert_eq!(loaded.focused_task_id, Some("task-1".to_string()));
    }

    #[test]
    fn accepts_v1_schema_without_scheduled_at() {
        let path = temp_path("v1-schema.json");
        let content = "{\n  \"schema_version\": 1,\n  \"tasks\": [\n    {\n      \"id\": \"task-1\",\n      \"title\": \"demo\",\n      \"status\": \"pending\",\n      \"created_at\": \"2025-12-20T00:00:00Z\"\n    }\n  ]\n}";
        fs::write(&path, content).unwrap();

        let loaded = load_tasks(&path).unwrap();
        fs::remove_file(&path).ok();

        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].scheduled_at, None);
        assert_eq!(loaded[0].completed_at, None);
        assert!(loaded[0].completion_history.is_empty());
        assert!(!loaded[0].urgent);
    }

    #[test]
    fn accepts_v2_schema_without_completion_fields() {
        let path = temp_path("v2-schema.json");
        let content = "{\n  \"schema_version\": 2,\n  \"tasks\": [\n    {\n      \"id\": \"task-1\",\n      \"title\": \"demo\",\n      \"status\": \"pending\",\n      \"created_at\": \"2025-12-20T00:00:00Z\",\n      \"scheduled_at\": \"2025-12-21T09:00:00Z\"\n    }\n  ]\n}";
        fs::write(&path, content).unwrap();

        let loaded = load_tasks(&path).unwrap();
        fs::remove_file(&path).ok();

        assert_eq!(loaded.len(), 1);
        assert_eq!(
            loaded[0].scheduled_at,
            Some("2025-12-21T09:00:00Z".to_string())
        );
        assert_eq!(loaded[0].completed_at, None);
        assert!(loaded[0].completion_history.is_empty());
        assert!(!loaded[0].urgent);
    }

    #[test]
    fn rejects_non_boolean_urgent_field() {
        let path = temp_path("bad-urgent.json");
        let content = "{\n  \"schema_version\": 5,\n  \"tasks\": [\n    {\n      \"id\": \"task-1\",\n      \"title\": \"demo\",\n      \"status\": \"pending\",\n      \"created_at\": \"2025-12-20T00:00:00Z\",\n      \"urgent\": \"yes\"\n    }\n  ]\n}";
        fs::write(&path, content).unwrap();

        let err = load_tasks(&path).unwrap_err();
        fs::remove_file(&path).ok();

        assert_eq!(err.code(), "invalid_data");
    }

    #[test]
    fn rejects_unknown_focused_task_id() {
        let path = temp_path("bad-focus.json");
        let content = "{\n  \"schema_version\": 4,\n  \"focused_task_id\": \"task-missing\",\n  \"tasks\": [\n    {\n      \"id\": \"task-1\",\n      \"title\": \"demo\",\n      \"status\": \"pending\",\n      \"created_at\": \"2025-12-20T00:00:00Z\"\n    }\n  ]\n}";
        fs::write(&path, content).unwrap();

        let err = load_state(&path).unwrap_err();
        fs::remove_file(&path).ok();

        assert_eq!(err.code(), "invalid_data");
    }

    #[test]
    fn schema_version_must_match() {
        let path = temp_path("bad-schema.json");
        let bad = format!(
            "{{\n  \"schema_version\": {},\n  \"tasks\": []\n}}",
            SCHEMA_VERSION + 1
        );
        fs::write(&path, bad).unwrap();

        let err = load_tasks(&path).unwrap_err();
        fs::remove_file(&path).ok();

        assert_eq!(err.code(), "invalid_data");
    }
}
