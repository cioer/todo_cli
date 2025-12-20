use crate::error::AppError;
use crate::model::{CompletionEntry, Task, TaskStatus};
use crate::storage::json_store;
use std::path::Path;
use time::format_description::well_known::Rfc3339;
use time::{Date, OffsetDateTime, UtcOffset};

pub fn add_task(title: &str) -> Result<Task, AppError> {
    let path = json_store::store_path()?;
    add_task_with_path(&path, title)
}

fn add_task_with_path(path: &Path, title: &str) -> Result<Task, AppError> {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        return Err(AppError::invalid_input("title is required"));
    }

    let created_at = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|err| AppError::invalid_data(err.to_string()))?;
    let id = format!(
        "task-{}",
        OffsetDateTime::now_utc().unix_timestamp_nanos()
    );

    let task = Task {
        id,
        title: trimmed.to_string(),
        status: TaskStatus::Pending,
        created_at,
        scheduled_at: None,
        completed_at: None,
        completion_history: Vec::new(),
    };

    let mut tasks = json_store::load_tasks(path)?;
    tasks.push(task.clone());
    json_store::save_tasks(path, &tasks)?;

    Ok(task)
}

pub fn list_today() -> Result<Vec<Task>, AppError> {
    let path = json_store::store_path()?;
    list_today_with_path(&path)
}

pub fn list_backlog() -> Result<Vec<Task>, AppError> {
    let path = json_store::store_path()?;
    list_backlog_with_path(&path)
}

pub fn edit_task(id: &str, new_title: &str) -> Result<Task, AppError> {
    let path = json_store::store_path()?;
    edit_task_with_path(&path, id, new_title)
}

pub fn delete_task(id: &str) -> Result<Task, AppError> {
    let path = json_store::store_path()?;
    delete_task_with_path(&path, id)
}

pub fn complete_task(id: &str, message: Option<&str>) -> Result<Task, AppError> {
    let path = json_store::store_path()?;
    complete_task_with_path(&path, id, message)
}

fn list_today_with_path(path: &Path) -> Result<Vec<Task>, AppError> {
    let tasks = json_store::load_tasks(path)?;
    let local_offset = local_offset()?;
    let today = OffsetDateTime::now_utc().to_offset(local_offset).date();
    filter_tasks(&tasks, today, local_offset, ListMode::Today)
}

fn list_backlog_with_path(path: &Path) -> Result<Vec<Task>, AppError> {
    let tasks = json_store::load_tasks(path)?;
    let local_offset = local_offset()?;
    let today = OffsetDateTime::now_utc().to_offset(local_offset).date();
    filter_tasks(&tasks, today, local_offset, ListMode::Backlog)
}

fn local_offset() -> Result<UtcOffset, AppError> {
    Ok(UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC))
}

enum ListMode {
    Today,
    Backlog,
}

fn filter_tasks(
    tasks: &[Task],
    today: Date,
    local_offset: UtcOffset,
    mode: ListMode,
) -> Result<Vec<Task>, AppError> {
    let mut filtered = Vec::new();

    for task in tasks {
        let scheduled_at = match task.scheduled_at.as_deref() {
            Some(value) => value,
            None => continue,
        };

        let scheduled = OffsetDateTime::parse(scheduled_at, &Rfc3339)
            .map_err(|_| AppError::invalid_data("scheduled_at must be RFC3339"))?;
        let scheduled_date = scheduled.to_offset(local_offset).date();

        let matches = match mode {
            ListMode::Today => scheduled_date == today,
            ListMode::Backlog => scheduled_date > today,
        };

        if matches {
            filtered.push(task.clone());
        }
    }

    Ok(filtered)
}

fn edit_task_with_path(path: &Path, id: &str, new_title: &str) -> Result<Task, AppError> {
    let trimmed_id = id.trim();
    if trimmed_id.is_empty() {
        return Err(AppError::invalid_input("id is required"));
    }

    let trimmed_title = new_title.trim();
    if trimmed_title.is_empty() {
        return Err(AppError::invalid_input("title is required"));
    }

    let mut tasks = json_store::load_tasks(path)?;
    let mut updated_task = None;

    for task in &mut tasks {
        if task.id == trimmed_id {
            task.title = trimmed_title.to_string();
            updated_task = Some(task.clone());
            break;
        }
    }

    let updated = updated_task.ok_or_else(|| AppError::invalid_input("task not found"))?;
    json_store::save_tasks(path, &tasks)?;

    Ok(updated)
}

fn delete_task_with_path(path: &Path, id: &str) -> Result<Task, AppError> {
    let trimmed_id = id.trim();
    if trimmed_id.is_empty() {
        return Err(AppError::invalid_input("id is required"));
    }

    let mut tasks = json_store::load_tasks(path)?;
    let index = tasks
        .iter()
        .position(|task| task.id == trimmed_id)
        .ok_or_else(|| AppError::invalid_input("task not found"))?;

    let removed = tasks.remove(index);
    json_store::save_tasks(path, &tasks)?;

    Ok(removed)
}

fn complete_task_with_path(path: &Path, id: &str, message: Option<&str>) -> Result<Task, AppError> {
    let trimmed_id = id.trim();
    if trimmed_id.is_empty() {
        return Err(AppError::invalid_input("id is required"));
    }

    let mut tasks = json_store::load_tasks(path)?;
    let mut updated_task = None;

    for task in &mut tasks {
        if task.id == trimmed_id {
            if task.status == TaskStatus::Completed {
                return Err(AppError::invalid_input("task already completed"));
            }

            let completed_at = OffsetDateTime::now_utc()
                .format(&Rfc3339)
                .map_err(|err| AppError::invalid_data(err.to_string()))?;

            task.status = TaskStatus::Completed;
            task.completed_at = Some(completed_at.clone());

            if let Some(value) = message {
                let trimmed = value.trim();
                if trimmed.is_empty() {
                    return Err(AppError::invalid_input("message is required"));
                }
                task.completion_history.push(CompletionEntry {
                    message: trimmed.to_string(),
                    completed_at: completed_at.clone(),
                });
            }

            updated_task = Some(task.clone());
            break;
        }
    }

    let updated = updated_task.ok_or_else(|| AppError::invalid_input("task not found"))?;
    json_store::save_tasks(path, &tasks)?;

    Ok(updated)
}

#[cfg(test)]
mod tests {
    use super::{
        add_task_with_path, complete_task_with_path, delete_task_with_path, edit_task_with_path,
        filter_tasks, ListMode,
    };
    use crate::model::{CompletionEntry, Task, TaskStatus};
    use crate::storage::json_store;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};
    use time::format_description::well_known::Rfc3339;
    use time::{Date, Duration, Month, OffsetDateTime, UtcOffset};

    fn temp_path(file_name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("todoapp-{nanos}-{file_name}"))
    }

    #[test]
    fn add_task_rejects_blank_title() {
        let path = temp_path("blank-title.json");
        let err = add_task_with_path(&path, "  ").unwrap_err();
        std::fs::remove_file(&path).ok();

        assert_eq!(err.code(), "invalid_input");
    }

    #[test]
    fn add_task_writes_to_store() {
        let path = temp_path("add-task.json");
        let task = add_task_with_path(&path, "demo").unwrap();
        let loaded = json_store::load_tasks(&path).unwrap();
        std::fs::remove_file(&path).ok();

        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, task.id);
        assert_eq!(loaded[0].title, task.title);
        assert_eq!(loaded[0].scheduled_at, None);
        assert_eq!(loaded[0].completed_at, None);
        assert!(loaded[0].completion_history.is_empty());
    }

    #[test]
    fn filter_tasks_returns_today_and_backlog() {
        let today = Date::from_calendar_date(2025, Month::December, 20).unwrap();
        let offset = UtcOffset::UTC;
        let today_dt = today
            .with_hms(12, 0, 0)
            .unwrap()
            .assume_offset(offset);
        let tomorrow_dt = (today + Duration::days(1))
            .with_hms(9, 0, 0)
            .unwrap()
            .assume_offset(offset);

        let tasks = vec![
            Task {
                id: "task-1".to_string(),
                title: "today".to_string(),
                status: TaskStatus::Pending,
                created_at: "2025-12-01T00:00:00Z".to_string(),
                scheduled_at: Some(today_dt.format(&Rfc3339).unwrap()),
                completed_at: None,
                completion_history: Vec::new(),
            },
            Task {
                id: "task-2".to_string(),
                title: "future".to_string(),
                status: TaskStatus::Pending,
                created_at: "2025-12-01T00:00:00Z".to_string(),
                scheduled_at: Some(tomorrow_dt.format(&Rfc3339).unwrap()),
                completed_at: None,
                completion_history: Vec::new(),
            },
            Task {
                id: "task-3".to_string(),
                title: "unscheduled".to_string(),
                status: TaskStatus::Pending,
                created_at: "2025-12-01T00:00:00Z".to_string(),
                scheduled_at: None,
                completed_at: None,
                completion_history: Vec::new(),
            },
        ];

        let today_tasks = filter_tasks(&tasks, today, offset, ListMode::Today).unwrap();
        assert_eq!(today_tasks.len(), 1);
        assert_eq!(today_tasks[0].id, "task-1");

        let backlog_tasks = filter_tasks(&tasks, today, offset, ListMode::Backlog).unwrap();
        assert_eq!(backlog_tasks.len(), 1);
        assert_eq!(backlog_tasks[0].id, "task-2");
    }

    #[test]
    fn filter_tasks_reports_invalid_scheduled_at() {
        let today = Date::from_calendar_date(2025, Month::December, 20).unwrap();
        let offset = UtcOffset::UTC;
        let tasks = vec![Task {
            id: "task-1".to_string(),
            title: "bad".to_string(),
            status: TaskStatus::Pending,
            created_at: "2025-12-01T00:00:00Z".to_string(),
            scheduled_at: Some("not-a-date".to_string()),
            completed_at: None,
            completion_history: Vec::new(),
        }];

        let err = filter_tasks(&tasks, today, offset, ListMode::Today).unwrap_err();
        assert_eq!(err.code(), "invalid_data");
    }

    #[test]
    fn edit_task_updates_title() {
        let path = temp_path("edit-task.json");
        let original = Task {
            id: "task-1".to_string(),
            title: "old".to_string(),
            status: TaskStatus::Pending,
            created_at: "2025-12-01T00:00:00Z".to_string(),
            scheduled_at: Some("2025-12-22T09:00:00Z".to_string()),
            completed_at: None,
            completion_history: Vec::new(),
        };

        json_store::save_tasks(&path, &[original.clone()]).unwrap();

        let updated = edit_task_with_path(&path, "task-1", "new").unwrap();
        let loaded = json_store::load_tasks(&path).unwrap();
        std::fs::remove_file(&path).ok();

        assert_eq!(updated.title, "new");
        assert_eq!(updated.scheduled_at, original.scheduled_at);
        assert_eq!(loaded[0].title, "new");
        assert_eq!(loaded[0].scheduled_at, original.scheduled_at);
    }

    #[test]
    fn edit_task_rejects_blank_title() {
        let path = temp_path("edit-blank.json");
        let task = Task {
            id: "task-1".to_string(),
            title: "old".to_string(),
            status: TaskStatus::Pending,
            created_at: "2025-12-01T00:00:00Z".to_string(),
            scheduled_at: None,
            completed_at: None,
            completion_history: Vec::new(),
        };

        json_store::save_tasks(&path, &[task]).unwrap();

        let err = edit_task_with_path(&path, "task-1", "  ").unwrap_err();
        std::fs::remove_file(&path).ok();

        assert_eq!(err.code(), "invalid_input");
    }

    #[test]
    fn edit_task_rejects_missing_id() {
        let path = temp_path("edit-missing.json");
        let task = Task {
            id: "task-1".to_string(),
            title: "old".to_string(),
            status: TaskStatus::Pending,
            created_at: "2025-12-01T00:00:00Z".to_string(),
            scheduled_at: None,
            completed_at: None,
            completion_history: Vec::new(),
        };

        json_store::save_tasks(&path, &[task]).unwrap();

        let err = edit_task_with_path(&path, "task-2", "new").unwrap_err();
        std::fs::remove_file(&path).ok();

        assert_eq!(err.code(), "invalid_input");
    }

    #[test]
    fn edit_task_rejects_blank_id() {
        let path = temp_path("edit-blank-id.json");
        let task = Task {
            id: "task-1".to_string(),
            title: "old".to_string(),
            status: TaskStatus::Pending,
            created_at: "2025-12-01T00:00:00Z".to_string(),
            scheduled_at: None,
            completed_at: None,
            completion_history: Vec::new(),
        };

        json_store::save_tasks(&path, &[task]).unwrap();

        let err = edit_task_with_path(&path, "  ", "new").unwrap_err();
        std::fs::remove_file(&path).ok();

        assert_eq!(err.code(), "invalid_input");
    }

    #[test]
    fn complete_task_sets_completed_at_and_history() {
        let path = temp_path("complete-task.json");
        let task = Task {
            id: "task-1".to_string(),
            title: "demo".to_string(),
            status: TaskStatus::Pending,
            created_at: "2025-12-01T00:00:00Z".to_string(),
            scheduled_at: Some("2025-12-22T09:00:00Z".to_string()),
            completed_at: None,
            completion_history: Vec::new(),
        };

        json_store::save_tasks(&path, &[task.clone()]).unwrap();

        let updated = complete_task_with_path(&path, "task-1", Some("ship it")).unwrap();
        let loaded = json_store::load_tasks(&path).unwrap();
        std::fs::remove_file(&path).ok();

        assert_eq!(updated.status, TaskStatus::Completed);
        let completed_at = updated.completed_at.clone().expect("completed_at set");
        OffsetDateTime::parse(&completed_at, &Rfc3339).unwrap();
        assert_eq!(updated.scheduled_at, task.scheduled_at);
        assert_eq!(updated.completion_history.len(), 1);
        assert_eq!(updated.completion_history[0].message, "ship it");
        assert_eq!(updated.completion_history[0].completed_at, completed_at);

        assert_eq!(loaded[0].status, TaskStatus::Completed);
        assert_eq!(loaded[0].completed_at, Some(completed_at));
        assert_eq!(loaded[0].completion_history.len(), 1);
        assert_eq!(loaded[0].completion_history[0].message, "ship it");
    }

    #[test]
    fn complete_task_without_message_keeps_history_empty() {
        let path = temp_path("complete-no-message.json");
        let task = Task {
            id: "task-1".to_string(),
            title: "demo".to_string(),
            status: TaskStatus::Pending,
            created_at: "2025-12-01T00:00:00Z".to_string(),
            scheduled_at: None,
            completed_at: None,
            completion_history: Vec::new(),
        };

        json_store::save_tasks(&path, &[task]).unwrap();

        let updated = complete_task_with_path(&path, "task-1", None).unwrap();
        std::fs::remove_file(&path).ok();

        assert_eq!(updated.status, TaskStatus::Completed);
        assert!(updated.completed_at.is_some());
        assert!(updated.completion_history.is_empty());
    }

    #[test]
    fn complete_task_rejects_already_completed() {
        let path = temp_path("complete-already.json");
        let task = Task {
            id: "task-1".to_string(),
            title: "demo".to_string(),
            status: TaskStatus::Completed,
            created_at: "2025-12-01T00:00:00Z".to_string(),
            scheduled_at: None,
            completed_at: Some("2025-12-22T10:00:00Z".to_string()),
            completion_history: vec![CompletionEntry {
                message: "already".to_string(),
                completed_at: "2025-12-22T10:00:00Z".to_string(),
            }],
        };

        json_store::save_tasks(&path, &[task]).unwrap();

        let err = complete_task_with_path(&path, "task-1", Some("ship it")).unwrap_err();
        std::fs::remove_file(&path).ok();

        assert_eq!(err.code(), "invalid_input");
    }

    #[test]
    fn complete_task_rejects_blank_id() {
        let path = temp_path("complete-blank-id.json");
        let task = Task {
            id: "task-1".to_string(),
            title: "demo".to_string(),
            status: TaskStatus::Pending,
            created_at: "2025-12-01T00:00:00Z".to_string(),
            scheduled_at: None,
            completed_at: None,
            completion_history: Vec::new(),
        };

        json_store::save_tasks(&path, &[task]).unwrap();

        let err = complete_task_with_path(&path, "  ", None).unwrap_err();
        std::fs::remove_file(&path).ok();

        assert_eq!(err.code(), "invalid_input");
    }

    #[test]
    fn complete_task_rejects_blank_message() {
        let path = temp_path("complete-blank-message.json");
        let task = Task {
            id: "task-1".to_string(),
            title: "demo".to_string(),
            status: TaskStatus::Pending,
            created_at: "2025-12-01T00:00:00Z".to_string(),
            scheduled_at: None,
            completed_at: None,
            completion_history: Vec::new(),
        };

        json_store::save_tasks(&path, &[task]).unwrap();

        let err = complete_task_with_path(&path, "task-1", Some("   ")).unwrap_err();
        std::fs::remove_file(&path).ok();

        assert_eq!(err.code(), "invalid_input");
    }

    #[test]
    fn delete_task_removes_task() {
        let path = temp_path("delete-task.json");
        let task = Task {
            id: "task-1".to_string(),
            title: "old".to_string(),
            status: TaskStatus::Pending,
            created_at: "2025-12-01T00:00:00Z".to_string(),
            scheduled_at: None,
            completed_at: None,
            completion_history: Vec::new(),
        };

        json_store::save_tasks(&path, &[task]).unwrap();

        let removed = delete_task_with_path(&path, "task-1").unwrap();
        let loaded = json_store::load_tasks(&path).unwrap();
        std::fs::remove_file(&path).ok();

        assert_eq!(removed.id, "task-1");
        assert!(loaded.is_empty());
    }

    #[test]
    fn delete_task_rejects_missing_id() {
        let path = temp_path("delete-missing.json");
        let task = Task {
            id: "task-1".to_string(),
            title: "old".to_string(),
            status: TaskStatus::Pending,
            created_at: "2025-12-01T00:00:00Z".to_string(),
            scheduled_at: None,
            completed_at: None,
            completion_history: Vec::new(),
        };

        json_store::save_tasks(&path, &[task]).unwrap();

        let err = delete_task_with_path(&path, "task-2").unwrap_err();
        std::fs::remove_file(&path).ok();

        assert_eq!(err.code(), "invalid_input");
    }

    #[test]
    fn delete_task_rejects_blank_id() {
        let path = temp_path("delete-blank-id.json");
        let task = Task {
            id: "task-1".to_string(),
            title: "old".to_string(),
            status: TaskStatus::Pending,
            created_at: "2025-12-01T00:00:00Z".to_string(),
            scheduled_at: None,
            completed_at: None,
            completion_history: Vec::new(),
        };

        json_store::save_tasks(&path, &[task]).unwrap();

        let err = delete_task_with_path(&path, "").unwrap_err();
        std::fs::remove_file(&path).ok();

        assert_eq!(err.code(), "invalid_input");
    }
}


