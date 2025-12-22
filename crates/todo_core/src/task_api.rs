use crate::error::AppError;
use crate::model::{CompletionEntry, Task, TaskStatus};
use crate::notify::{Notifier, activation_argument, notifier_from_env};
use crate::storage::json_store;
use std::path::Path;
use time::format_description::well_known::Rfc3339;
use time::{Date, OffsetDateTime, UtcOffset};

#[derive(Debug, Clone)]
pub struct ListResult {
    pub tasks: Vec<Task>,
    pub focused_task_id: Option<String>,
}

#[derive(Debug)]
pub struct NotificationOutcome {
    pub tasks: Vec<Task>,
    pub failures: Vec<NotificationFailure>,
}

#[derive(Debug)]
pub struct NotificationFailure {
    pub task_id: String,
    pub error: AppError,
}

pub fn add_task(title: &str) -> Result<Task, AppError> {
    add_task_with_urgency(title, false)
}

pub fn add_task_with_urgency(title: &str, urgent: bool) -> Result<Task, AppError> {
    let path = json_store::store_path()?;
    add_task_with_path(&path, title, urgent)
}

fn add_task_with_path(path: &Path, title: &str, urgent: bool) -> Result<Task, AppError> {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        return Err(AppError::invalid_input("title is required"));
    }

    let created_at = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|err| AppError::invalid_data(err.to_string()))?;
    let id = format!("task-{}", OffsetDateTime::now_utc().unix_timestamp_nanos());

    let task = Task {
        id,
        title: trimmed.to_string(),
        status: TaskStatus::Pending,
        created_at,
        scheduled_at: None,
        completed_at: None,
        completion_history: Vec::new(),
        urgent,
    };

    let mut state = json_store::load_state(path)?;
    state.tasks.push(task.clone());
    json_store::save_state(path, &state)?;

    Ok(task)
}

pub fn list_today() -> Result<Vec<Task>, AppError> {
    Ok(list_today_with_focus()?.tasks)
}

pub fn list_backlog() -> Result<Vec<Task>, AppError> {
    let path = json_store::store_path()?;
    list_without_focus(&path, ListMode::Backlog)
}

pub fn list_today_with_focus() -> Result<ListResult, AppError> {
    let path = json_store::store_path()?;
    list_today_with_focus_with_path(&path)
}

pub fn list_backlog_with_focus() -> Result<ListResult, AppError> {
    let path = json_store::store_path()?;
    list_backlog_with_focus_with_path(&path)
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

pub fn complete_focused_task(message: Option<&str>) -> Result<Task, AppError> {
    let path = json_store::store_path()?;
    complete_focused_task_with_path(&path, message)
}

pub fn schedule_task(id: &str, datetime: &str) -> Result<Task, AppError> {
    let path = json_store::store_path()?;
    schedule_task_with_path(&path, id, datetime)
}

pub fn reschedule_task(id: &str, datetime: &str) -> Result<Task, AppError> {
    let path = json_store::store_path()?;
    reschedule_task_with_path(&path, id, datetime)
}

pub fn set_focus(id: &str) -> Result<Task, AppError> {
    let path = json_store::store_path()?;
    set_focus_with_path(&path, id)
}

pub fn set_task_urgent(id: &str, urgent: bool) -> Result<Task, AppError> {
    let path = json_store::store_path()?;
    set_task_urgent_with_path(&path, id, urgent)
}

pub fn get_task_by_id(id: &str) -> Result<Task, AppError> {
    let path = json_store::store_path()?;
    get_task_by_id_with_path(&path, id)
}

pub fn notify_overdue_or_urgent() -> Result<NotificationOutcome, AppError> {
    let path = json_store::store_path()?;
    let notifier = notifier_from_env()?;
    notify_overdue_or_urgent_with_path(&path, notifier.as_ref())
}

fn list_today_with_focus_with_path(path: &Path) -> Result<ListResult, AppError> {
    list_with_focus(path, ListMode::Today)
}

fn get_task_by_id_with_path(path: &Path, id: &str) -> Result<Task, AppError> {
    let trimmed_id = id.trim();
    if trimmed_id.is_empty() {
        return Err(AppError::invalid_input("id is required"));
    }

    let state = json_store::load_state(path)?;
    state
        .tasks
        .into_iter()
        .find(|task| task.id == trimmed_id)
        .ok_or_else(|| AppError::invalid_input("task not found"))
}

fn set_task_urgent_with_path(path: &Path, id: &str, urgent: bool) -> Result<Task, AppError> {
    let trimmed_id = id.trim();
    if trimmed_id.is_empty() {
        return Err(AppError::invalid_input("id is required"));
    }

    let mut state = json_store::load_state(path)?;
    let mut updated_task = None;

    for task in &mut state.tasks {
        if task.id == trimmed_id {
            task.urgent = urgent;
            updated_task = Some(task.clone());
            break;
        }
    }

    let updated = updated_task.ok_or_else(|| AppError::invalid_input("task not found"))?;
    json_store::save_state(path, &state)?;

    Ok(updated)
}

fn notify_overdue_or_urgent_with_path(
    path: &Path,
    notifier: &dyn Notifier,
) -> Result<NotificationOutcome, AppError> {
    let state = json_store::load_state(path)?;
    let mut notified = Vec::new();
    let mut failures = Vec::new();

    for task in &state.tasks {
        if task.status != TaskStatus::Pending {
            continue;
        }

        let overdue = task_overdue(task)?;
        if !overdue && !task.urgent {
            continue;
        }

        let action = activation_argument(&task.id);
        match notifier.notify_with_action(task, &action) {
            Ok(_) => notified.push(task.clone()),
            Err(err) => failures.push(NotificationFailure {
                task_id: task.id.clone(),
                error: err,
            }),
        }
    }

    Ok(NotificationOutcome {
        tasks: notified,
        failures,
    })
}

fn list_backlog_with_focus_with_path(path: &Path) -> Result<ListResult, AppError> {
    list_with_focus(path, ListMode::Backlog)
}

fn list_without_focus(path: &Path, mode: ListMode) -> Result<Vec<Task>, AppError> {
    let tasks = json_store::load_state(path)?.tasks;
    let local_offset = local_offset()?;
    let today = OffsetDateTime::now_utc().to_offset(local_offset).date();
    filter_tasks(&tasks, today, local_offset, mode)
}

fn list_with_focus(path: &Path, mode: ListMode) -> Result<ListResult, AppError> {
    let state = json_store::load_state(path)?;
    let local_offset = local_offset()?;
    let today = OffsetDateTime::now_utc().to_offset(local_offset).date();
    let mut tasks = filter_tasks(&state.tasks, today, local_offset, mode)?;
    let focused_task_id = state.focused_task_id.clone();

    if let Some(focused_id) = focused_task_id.as_deref()
        && let Some(index) = tasks.iter().position(|task| task.id == focused_id)
    {
        let focused_task = tasks.remove(index);
        tasks.insert(0, focused_task);
    }

    Ok(ListResult {
        tasks,
        focused_task_id,
    })
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
            None => {
                if matches!(mode, ListMode::Backlog) {
                    filtered.push(task.clone());
                }
                continue;
            }
        };

        let scheduled = OffsetDateTime::parse(scheduled_at, &Rfc3339)
            .map_err(|_| AppError::invalid_data("scheduled_at must be RFC3339"))?;
        let scheduled_local = scheduled.to_offset(local_offset);
        let scheduled_date = scheduled_local.date();

        let matches = match mode {
            ListMode::Today => scheduled_date <= today,
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

    let mut state = json_store::load_state(path)?;
    let mut updated_task = None;

    for task in &mut state.tasks {
        if task.id == trimmed_id {
            task.title = trimmed_title.to_string();
            updated_task = Some(task.clone());
            break;
        }
    }

    let updated = updated_task.ok_or_else(|| AppError::invalid_input("task not found"))?;
    if state.focused_task_id.as_deref() == Some(trimmed_id) {
        state.focused_task_id = None;
    }
    json_store::save_state(path, &state)?;

    Ok(updated)
}

fn delete_task_with_path(path: &Path, id: &str) -> Result<Task, AppError> {
    let trimmed_id = id.trim();
    if trimmed_id.is_empty() {
        return Err(AppError::invalid_input("id is required"));
    }

    let mut state = json_store::load_state(path)?;
    let index = state
        .tasks
        .iter()
        .position(|task| task.id == trimmed_id)
        .ok_or_else(|| AppError::invalid_input("task not found"))?;

    let removed = state.tasks.remove(index);
    if state.focused_task_id.as_deref() == Some(trimmed_id) {
        state.focused_task_id = None;
    }
    json_store::save_state(path, &state)?;

    Ok(removed)
}

fn complete_task_with_path(path: &Path, id: &str, message: Option<&str>) -> Result<Task, AppError> {
    let trimmed_id = id.trim();
    if trimmed_id.is_empty() {
        return Err(AppError::invalid_input("id is required"));
    }

    let mut state = json_store::load_state(path)?;
    let mut updated_task = None;

    for task in &mut state.tasks {
        if task.id == trimmed_id {
            if task.status == TaskStatus::Completed {
                return Err(AppError::invalid_input("task already completed"));
            }

            let trimmed_message = match message {
                Some(value) => {
                    let trimmed = value.trim();
                    if trimmed.is_empty() {
                        return Err(AppError::invalid_input("message is required"));
                    }
                    Some(trimmed.to_string())
                }
                None => None,
            };

            let completed_at = OffsetDateTime::now_utc()
                .format(&Rfc3339)
                .map_err(|err| AppError::invalid_data(err.to_string()))?;

            task.status = TaskStatus::Completed;
            task.completed_at = Some(completed_at.clone());

            if let Some(message) = trimmed_message {
                task.completion_history.push(CompletionEntry {
                    message,
                    completed_at: completed_at.clone(),
                });
            }

            updated_task = Some(task.clone());
            break;
        }
    }

    let updated = updated_task.ok_or_else(|| AppError::invalid_input("task not found"))?;
    if state.focused_task_id.as_deref() == Some(trimmed_id) {
        state.focused_task_id = None;
    }
    json_store::save_state(path, &state)?;

    Ok(updated)
}

fn complete_focused_task_with_path(path: &Path, message: Option<&str>) -> Result<Task, AppError> {
    let mut state = json_store::load_state(path)?;
    let focused_id = state
        .focused_task_id
        .clone()
        .ok_or_else(|| AppError::invalid_input("no focused task"))?;
    let trimmed_message = match message {
        Some(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                return Err(AppError::invalid_input("message is required"));
            }
            Some(trimmed.to_string())
        }
        None => None,
    };
    let mut updated_task = None;

    for task in &mut state.tasks {
        if task.id == focused_id {
            if task.status == TaskStatus::Completed {
                return Err(AppError::invalid_input("task already completed"));
            }

            let completed_at = OffsetDateTime::now_utc()
                .format(&Rfc3339)
                .map_err(|err| AppError::invalid_data(err.to_string()))?;

            task.status = TaskStatus::Completed;
            task.completed_at = Some(completed_at.clone());

            if let Some(message) = trimmed_message {
                task.completion_history.push(CompletionEntry {
                    message,
                    completed_at: completed_at.clone(),
                });
            }

            updated_task = Some(task.clone());
            break;
        }
    }

    let updated = updated_task.ok_or_else(|| AppError::invalid_input("task not found"))?;
    state.focused_task_id = None;
    json_store::save_state(path, &state)?;

    Ok(updated)
}

fn schedule_task_with_path(path: &Path, id: &str, datetime: &str) -> Result<Task, AppError> {
    update_schedule_with_path(path, id, datetime, false, false)
}

fn reschedule_task_with_path(path: &Path, id: &str, datetime: &str) -> Result<Task, AppError> {
    update_schedule_with_path(path, id, datetime, true, true)
}

fn set_focus_with_path(path: &Path, id: &str) -> Result<Task, AppError> {
    let trimmed_id = id.trim();
    if trimmed_id.is_empty() {
        return Err(AppError::invalid_input("id is required"));
    }

    let mut state = json_store::load_state(path)?;
    let mut focused = None;

    for task in &state.tasks {
        if task.id == trimmed_id {
            focused = Some(task.clone());
            break;
        }
    }

    let task = focused.ok_or_else(|| AppError::invalid_input("task not found"))?;
    state.focused_task_id = Some(task.id.clone());
    json_store::save_state(path, &state)?;

    Ok(task)
}

fn update_schedule_with_path(
    path: &Path,
    id: &str,
    datetime: &str,
    require_existing: bool,
    require_overdue: bool,
) -> Result<Task, AppError> {
    let trimmed_id = id.trim();
    if trimmed_id.is_empty() {
        return Err(AppError::invalid_input("id is required"));
    }

    let trimmed_datetime = datetime.trim();
    if trimmed_datetime.is_empty() {
        return Err(AppError::invalid_input("datetime is required"));
    }

    let parsed = OffsetDateTime::parse(trimmed_datetime, &Rfc3339)
        .map_err(|_| AppError::invalid_input("datetime must be RFC3339"))?;
    let scheduled_at = parsed
        .format(&Rfc3339)
        .map_err(|err| AppError::invalid_data(err.to_string()))?;

    let mut state = json_store::load_state(path)?;
    let mut updated_task = None;
    let local_offset = local_offset()?;
    let now_local = OffsetDateTime::now_utc().to_offset(local_offset);

    for task in &mut state.tasks {
        if task.id == trimmed_id {
            if require_existing && task.scheduled_at.is_none() {
                return Err(AppError::invalid_input("task is not scheduled"));
            }
            if require_overdue {
                let scheduled_at = task
                    .scheduled_at
                    .as_deref()
                    .ok_or_else(|| AppError::invalid_input("task is not scheduled"))?;
                if !is_overdue(scheduled_at, local_offset, now_local)? {
                    return Err(AppError::invalid_input("task is not overdue"));
                }
            }
            task.scheduled_at = Some(scheduled_at.clone());
            updated_task = Some(task.clone());
            break;
        }
    }

    let updated = updated_task.ok_or_else(|| AppError::invalid_input("task not found"))?;
    json_store::save_state(path, &state)?;

    Ok(updated)
}

fn is_overdue(
    scheduled_at: &str,
    local_offset: UtcOffset,
    now_local: OffsetDateTime,
) -> Result<bool, AppError> {
    let scheduled = OffsetDateTime::parse(scheduled_at, &Rfc3339)
        .map_err(|_| AppError::invalid_data("scheduled_at must be RFC3339"))?;
    Ok(scheduled.to_offset(local_offset) < now_local)
}

pub fn task_overdue(task: &Task) -> Result<bool, AppError> {
    let scheduled_at = match task.scheduled_at.as_deref() {
        Some(value) => value,
        None => return Ok(false),
    };
    let local_offset = local_offset()?;
    let now_local = OffsetDateTime::now_utc().to_offset(local_offset);
    is_overdue(scheduled_at, local_offset, now_local)
}
#[cfg(test)]
mod tests {
    use super::{
        ListMode, add_task_with_path, complete_focused_task_with_path, complete_task_with_path,
        delete_task_with_path, edit_task_with_path, filter_tasks, get_task_by_id_with_path,
        list_today_with_focus_with_path, list_without_focus, notify_overdue_or_urgent_with_path,
        reschedule_task_with_path, schedule_task_with_path, set_focus_with_path,
        set_task_urgent_with_path,
    };
    use crate::error::AppError;
    use crate::model::{CompletionEntry, Task, TaskStatus};
    use crate::notify::Notifier;
    use crate::storage::json_store;
    use std::cell::RefCell;
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
        let err = add_task_with_path(&path, "  ", false).unwrap_err();
        std::fs::remove_file(&path).ok();

        assert_eq!(err.code(), "invalid_input");
    }

    #[test]
    fn add_task_writes_to_store() {
        let path = temp_path("add-task.json");
        let task = add_task_with_path(&path, "demo", false).unwrap();
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
    fn set_focus_sets_focused_task_id() {
        let path = temp_path("focus.json");
        let tasks = vec![
            Task {
                id: "task-1".to_string(),
                title: "first".to_string(),
                status: TaskStatus::Pending,
                created_at: "2025-12-01T00:00:00Z".to_string(),
                scheduled_at: None,
                completed_at: None,
                completion_history: Vec::new(),
                urgent: false,
            },
            Task {
                id: "task-2".to_string(),
                title: "second".to_string(),
                status: TaskStatus::Pending,
                created_at: "2025-12-01T00:00:00Z".to_string(),
                scheduled_at: None,
                completed_at: None,
                completion_history: Vec::new(),
                urgent: false,
            },
        ];

        json_store::save_state(
            &path,
            &json_store::TaskState {
                tasks: tasks.clone(),
                focused_task_id: None,
            },
        )
        .unwrap();

        let focused = set_focus_with_path(&path, "task-2").unwrap();
        let loaded = json_store::load_state(&path).unwrap();
        std::fs::remove_file(&path).ok();

        assert_eq!(focused.id, "task-2");
        assert_eq!(loaded.focused_task_id, Some("task-2".to_string()));
        assert_eq!(loaded.tasks.len(), 2);
    }

    #[test]
    fn set_focus_rejects_missing_task() {
        let path = temp_path("focus-missing.json");
        let tasks = vec![Task {
            id: "task-1".to_string(),
            title: "first".to_string(),
            status: TaskStatus::Pending,
            created_at: "2025-12-01T00:00:00Z".to_string(),
            scheduled_at: None,
            completed_at: None,
            completion_history: Vec::new(),
            urgent: false,
        }];

        json_store::save_state(
            &path,
            &json_store::TaskState {
                tasks,
                focused_task_id: None,
            },
        )
        .unwrap();

        let err = set_focus_with_path(&path, "task-2").unwrap_err();
        std::fs::remove_file(&path).ok();

        assert_eq!(err.code(), "invalid_input");
    }

    #[test]
    fn set_task_urgent_updates_flag() {
        let path = temp_path("urgent-toggle.json");
        let task = Task {
            id: "task-1".to_string(),
            title: "urgent".to_string(),
            status: TaskStatus::Pending,
            created_at: "2025-12-01T00:00:00Z".to_string(),
            scheduled_at: None,
            completed_at: None,
            completion_history: Vec::new(),
            urgent: false,
        };

        json_store::save_tasks(&path, &[task.clone()]).unwrap();

        let updated = set_task_urgent_with_path(&path, "task-1", true).unwrap();
        let loaded = json_store::load_tasks(&path).unwrap();
        std::fs::remove_file(&path).ok();

        assert!(updated.urgent);
        assert!(!loaded.is_empty());
        assert!(loaded[0].urgent);
    }

    #[test]
    fn set_task_urgent_rejects_missing_task() {
        let path = temp_path("urgent-missing.json");
        json_store::save_tasks(&path, &[]).unwrap();

        let err = set_task_urgent_with_path(&path, "task-1", true).unwrap_err();
        std::fs::remove_file(&path).ok();

        assert_eq!(err.code(), "invalid_input");
    }

    #[test]
    fn get_task_by_id_returns_task() {
        let path = temp_path("get-task.json");
        let task = Task {
            id: "task-1".to_string(),
            title: "demo".to_string(),
            status: TaskStatus::Pending,
            created_at: "2025-12-01T00:00:00Z".to_string(),
            scheduled_at: None,
            completed_at: None,
            completion_history: Vec::new(),
            urgent: false,
        };

        json_store::save_tasks(&path, std::slice::from_ref(&task)).unwrap();

        let fetched = get_task_by_id_with_path(&path, "task-1").unwrap();
        std::fs::remove_file(&path).ok();

        assert_eq!(fetched, task);
    }

    #[test]
    fn get_task_by_id_rejects_missing_task() {
        let path = temp_path("get-task-missing.json");
        json_store::save_tasks(&path, &[]).unwrap();

        let err = get_task_by_id_with_path(&path, "task-1").unwrap_err();
        std::fs::remove_file(&path).ok();

        assert_eq!(err.code(), "invalid_input");
    }

    #[test]
    fn complete_focused_task_marks_completed_and_clears_focus() {
        let path = temp_path("complete-focused.json");
        let task = Task {
            id: "task-1".to_string(),
            title: "demo".to_string(),
            status: TaskStatus::Pending,
            created_at: "2025-12-01T00:00:00Z".to_string(),
            scheduled_at: None,
            completed_at: None,
            completion_history: Vec::new(),
            urgent: false,
        };

        json_store::save_state(
            &path,
            &json_store::TaskState {
                tasks: vec![task],
                focused_task_id: Some("task-1".to_string()),
            },
        )
        .unwrap();

        let completed = complete_focused_task_with_path(&path, Some("ship it")).unwrap();
        let loaded = json_store::load_state(&path).unwrap();
        std::fs::remove_file(&path).ok();

        assert_eq!(completed.status, TaskStatus::Completed);
        assert!(completed.completed_at.is_some());
        assert_eq!(completed.completion_history.len(), 1);
        assert_eq!(loaded.focused_task_id, None);
    }

    #[test]
    fn complete_focused_task_rejects_missing_focus() {
        let path = temp_path("complete-focused-missing.json");
        json_store::save_state(
            &path,
            &json_store::TaskState {
                tasks: Vec::new(),
                focused_task_id: None,
            },
        )
        .unwrap();

        let err = complete_focused_task_with_path(&path, None).unwrap_err();
        std::fs::remove_file(&path).ok();

        assert_eq!(err.code(), "invalid_input");
    }

    #[test]
    fn complete_task_clears_focus_when_matching_id() {
        let path = temp_path("complete-clears-focus.json");
        let task = Task {
            id: "task-1".to_string(),
            title: "demo".to_string(),
            status: TaskStatus::Pending,
            created_at: "2025-12-01T00:00:00Z".to_string(),
            scheduled_at: None,
            completed_at: None,
            completion_history: Vec::new(),
            urgent: false,
        };

        json_store::save_state(
            &path,
            &json_store::TaskState {
                tasks: vec![task],
                focused_task_id: Some("task-1".to_string()),
            },
        )
        .unwrap();

        let completed = complete_task_with_path(&path, "task-1", None).unwrap();
        let loaded = json_store::load_state(&path).unwrap();
        std::fs::remove_file(&path).ok();

        assert_eq!(completed.status, TaskStatus::Completed);
        assert_eq!(loaded.focused_task_id, None);
    }

    #[test]
    fn filter_tasks_returns_today_and_backlog() {
        let today = Date::from_calendar_date(2025, Month::December, 20).unwrap();
        let offset = UtcOffset::UTC;
        let today_dt = today.with_hms(12, 0, 0).unwrap().assume_offset(offset);
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
                urgent: false,
            },
            Task {
                id: "task-2".to_string(),
                title: "future".to_string(),
                status: TaskStatus::Pending,
                created_at: "2025-12-01T00:00:00Z".to_string(),
                scheduled_at: Some(tomorrow_dt.format(&Rfc3339).unwrap()),
                completed_at: None,
                completion_history: Vec::new(),
                urgent: false,
            },
            Task {
                id: "task-3".to_string(),
                title: "unscheduled".to_string(),
                status: TaskStatus::Pending,
                created_at: "2025-12-01T00:00:00Z".to_string(),
                scheduled_at: None,
                completed_at: None,
                completion_history: Vec::new(),
                urgent: false,
            },
        ];

        let today_tasks = filter_tasks(&tasks, today, offset, ListMode::Today).unwrap();
        assert_eq!(today_tasks.len(), 1);
        assert_eq!(today_tasks[0].id, "task-1");

        let backlog_tasks = filter_tasks(&tasks, today, offset, ListMode::Backlog).unwrap();
        assert_eq!(backlog_tasks.len(), 2);
        assert!(backlog_tasks.iter().any(|task| task.id == "task-2"));
        assert!(backlog_tasks.iter().any(|task| task.id == "task-3"));
    }

    #[test]
    fn filter_tasks_backlog_includes_unscheduled_tasks() {
        let today = Date::from_calendar_date(2025, Month::December, 20).unwrap();
        let offset = UtcOffset::UTC;
        let future_dt = (today + Duration::days(2))
            .with_hms(10, 0, 0)
            .unwrap()
            .assume_offset(offset);

        let tasks = vec![
            Task {
                id: "future".to_string(),
                title: "scheduled".to_string(),
                status: TaskStatus::Pending,
                created_at: "2025-12-01T00:00:00Z".to_string(),
                scheduled_at: Some(future_dt.format(&Rfc3339).unwrap()),
                completed_at: None,
                completion_history: Vec::new(),
                urgent: false,
            },
            Task {
                id: "unscheduled".to_string(),
                title: "later".to_string(),
                status: TaskStatus::Pending,
                created_at: "2025-12-01T00:00:00Z".to_string(),
                scheduled_at: None,
                completed_at: None,
                completion_history: Vec::new(),
                urgent: false,
            },
        ];

        let backlog_tasks = filter_tasks(&tasks, today, offset, ListMode::Backlog).unwrap();
        assert_eq!(backlog_tasks.len(), 2);
        assert!(backlog_tasks.iter().any(|task| task.id == "future"));
        assert!(backlog_tasks.iter().any(|task| task.id == "unscheduled"));
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
            urgent: false,
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
            urgent: false,
        };

        json_store::save_tasks(&path, std::slice::from_ref(&original)).unwrap();

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
            urgent: false,
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
            urgent: false,
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
            urgent: false,
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
            urgent: false,
        };

        json_store::save_tasks(&path, std::slice::from_ref(&task)).unwrap();

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
            urgent: false,
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
            urgent: false,
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
            urgent: false,
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
            urgent: false,
        };

        json_store::save_tasks(&path, &[task]).unwrap();

        let err = complete_task_with_path(&path, "task-1", Some("   ")).unwrap_err();
        std::fs::remove_file(&path).ok();

        assert_eq!(err.code(), "invalid_input");
    }

    #[test]
    fn complete_task_rejects_missing_id() {
        let path = temp_path("complete-missing.json");
        let task = Task {
            id: "task-1".to_string(),
            title: "demo".to_string(),
            status: TaskStatus::Pending,
            created_at: "2025-12-01T00:00:00Z".to_string(),
            scheduled_at: None,
            completed_at: None,
            completion_history: Vec::new(),
            urgent: false,
        };

        json_store::save_tasks(&path, &[task]).unwrap();

        let err = complete_task_with_path(&path, "task-2", None).unwrap_err();
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
            urgent: false,
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
            urgent: false,
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
            urgent: false,
        };

        json_store::save_tasks(&path, &[task]).unwrap();

        let err = delete_task_with_path(&path, "").unwrap_err();
        std::fs::remove_file(&path).ok();

        assert_eq!(err.code(), "invalid_input");
    }

    #[test]
    fn schedule_task_updates_scheduled_at_and_persists() {
        let path = temp_path("schedule-task.json");
        let task = Task {
            id: "task-1".to_string(),
            title: "demo".to_string(),
            status: TaskStatus::Pending,
            created_at: "2025-12-01T00:00:00Z".to_string(),
            scheduled_at: None,
            completed_at: None,
            completion_history: Vec::new(),
            urgent: false,
        };

        json_store::save_tasks(&path, &[task]).unwrap();

        let updated = schedule_task_with_path(&path, "task-1", "2025-12-21T09:00:00Z").unwrap();
        let loaded = json_store::load_tasks(&path).unwrap();
        std::fs::remove_file(&path).ok();

        assert_eq!(
            updated.scheduled_at,
            Some("2025-12-21T09:00:00Z".to_string())
        );
        assert_eq!(loaded[0].scheduled_at, updated.scheduled_at);
    }

    #[test]
    fn schedule_task_rejects_invalid_datetime() {
        let path = temp_path("schedule-invalid.json");
        let task = Task {
            id: "task-1".to_string(),
            title: "demo".to_string(),
            status: TaskStatus::Pending,
            created_at: "2025-12-01T00:00:00Z".to_string(),
            scheduled_at: None,
            completed_at: None,
            completion_history: Vec::new(),
            urgent: false,
        };

        json_store::save_tasks(&path, &[task]).unwrap();

        let err = schedule_task_with_path(&path, "task-1", "bad-date").unwrap_err();
        std::fs::remove_file(&path).ok();

        assert_eq!(err.code(), "invalid_input");
    }

    #[test]
    fn schedule_task_rejects_blank_id() {
        let path = temp_path("schedule-blank-id.json");
        let task = Task {
            id: "task-1".to_string(),
            title: "demo".to_string(),
            status: TaskStatus::Pending,
            created_at: "2025-12-01T00:00:00Z".to_string(),
            scheduled_at: None,
            completed_at: None,
            completion_history: Vec::new(),
            urgent: false,
        };

        json_store::save_tasks(&path, &[task]).unwrap();

        let err = schedule_task_with_path(&path, "  ", "2025-12-21T09:00:00Z").unwrap_err();
        std::fs::remove_file(&path).ok();

        assert_eq!(err.code(), "invalid_input");
    }

    #[test]
    fn schedule_task_rejects_unknown_id() {
        let path = temp_path("schedule-missing.json");
        let task = Task {
            id: "task-1".to_string(),
            title: "demo".to_string(),
            status: TaskStatus::Pending,
            created_at: "2025-12-01T00:00:00Z".to_string(),
            scheduled_at: None,
            completed_at: None,
            completion_history: Vec::new(),
            urgent: false,
        };

        json_store::save_tasks(&path, &[task]).unwrap();

        let err = schedule_task_with_path(&path, "task-2", "2025-12-21T09:00:00Z").unwrap_err();
        std::fs::remove_file(&path).ok();

        assert_eq!(err.code(), "invalid_input");
    }

    #[test]
    fn reschedule_task_rejects_unscheduled_task() {
        let path = temp_path("reschedule-unscheduled.json");
        let task = Task {
            id: "task-1".to_string(),
            title: "demo".to_string(),
            status: TaskStatus::Pending,
            created_at: "2025-12-01T00:00:00Z".to_string(),
            scheduled_at: None,
            completed_at: None,
            completion_history: Vec::new(),
            urgent: false,
        };

        json_store::save_tasks(&path, &[task]).unwrap();

        let err = reschedule_task_with_path(&path, "task-1", "2025-12-21T09:00:00Z").unwrap_err();
        std::fs::remove_file(&path).ok();

        assert_eq!(err.code(), "invalid_input");
    }

    #[test]
    fn reschedule_task_rejects_non_overdue_task() {
        let path = temp_path("reschedule-not-overdue.json");
        let future = (OffsetDateTime::now_utc() + Duration::days(1))
            .format(&Rfc3339)
            .unwrap();
        let task = Task {
            id: "task-1".to_string(),
            title: "demo".to_string(),
            status: TaskStatus::Pending,
            created_at: "2025-12-01T00:00:00Z".to_string(),
            scheduled_at: Some(future),
            completed_at: None,
            completion_history: Vec::new(),
            urgent: false,
        };

        json_store::save_tasks(&path, &[task]).unwrap();

        let err = reschedule_task_with_path(&path, "task-1", "2025-12-21T09:00:00Z").unwrap_err();
        std::fs::remove_file(&path).ok();

        assert_eq!(err.code(), "invalid_input");
    }

    #[test]
    fn reschedule_task_updates_scheduled_at_and_persists() {
        let path = temp_path("reschedule-task.json");
        let now = OffsetDateTime::now_utc();
        let past = (now - Duration::days(1)).format(&Rfc3339).unwrap();
        let future = (now + Duration::days(1)).format(&Rfc3339).unwrap();
        let task = Task {
            id: "task-1".to_string(),
            title: "demo".to_string(),
            status: TaskStatus::Pending,
            created_at: "2025-12-01T00:00:00Z".to_string(),
            scheduled_at: Some(past),
            completed_at: None,
            completion_history: Vec::new(),
            urgent: false,
        };

        json_store::save_tasks(&path, &[task]).unwrap();

        let updated = reschedule_task_with_path(&path, "task-1", &future).unwrap();
        let loaded = json_store::load_tasks(&path).unwrap();
        std::fs::remove_file(&path).ok();

        assert_eq!(updated.scheduled_at, Some(future));
        assert_eq!(loaded[0].scheduled_at, updated.scheduled_at);
    }

    #[test]
    fn reschedule_task_rejects_invalid_datetime() {
        let path = temp_path("reschedule-invalid.json");
        let past = (OffsetDateTime::now_utc() - Duration::days(1))
            .format(&Rfc3339)
            .unwrap();
        let task = Task {
            id: "task-1".to_string(),
            title: "demo".to_string(),
            status: TaskStatus::Pending,
            created_at: "2025-12-01T00:00:00Z".to_string(),
            scheduled_at: Some(past),
            completed_at: None,
            completion_history: Vec::new(),
            urgent: false,
        };

        json_store::save_tasks(&path, &[task]).unwrap();

        let err = reschedule_task_with_path(&path, "task-1", "bad-date").unwrap_err();
        std::fs::remove_file(&path).ok();

        assert_eq!(err.code(), "invalid_input");
    }

    #[test]
    fn reschedule_task_rejects_blank_id() {
        let path = temp_path("reschedule-blank-id.json");
        let past = (OffsetDateTime::now_utc() - Duration::days(1))
            .format(&Rfc3339)
            .unwrap();
        let task = Task {
            id: "task-1".to_string(),
            title: "demo".to_string(),
            status: TaskStatus::Pending,
            created_at: "2025-12-01T00:00:00Z".to_string(),
            scheduled_at: Some(past),
            completed_at: None,
            completion_history: Vec::new(),
            urgent: false,
        };

        json_store::save_tasks(&path, &[task]).unwrap();

        let err = reschedule_task_with_path(&path, "  ", "2025-12-21T09:00:00Z").unwrap_err();
        std::fs::remove_file(&path).ok();

        assert_eq!(err.code(), "invalid_input");
    }

    #[test]
    fn reschedule_task_rejects_unknown_id() {
        let path = temp_path("reschedule-missing.json");
        let past = (OffsetDateTime::now_utc() - Duration::days(1))
            .format(&Rfc3339)
            .unwrap();
        let task = Task {
            id: "task-1".to_string(),
            title: "demo".to_string(),
            status: TaskStatus::Pending,
            created_at: "2025-12-01T00:00:00Z".to_string(),
            scheduled_at: Some(past),
            completed_at: None,
            completion_history: Vec::new(),
            urgent: false,
        };

        json_store::save_tasks(&path, &[task]).unwrap();

        let err = reschedule_task_with_path(&path, "task-2", "2025-12-21T09:00:00Z").unwrap_err();
        std::fs::remove_file(&path).ok();

        assert_eq!(err.code(), "invalid_input");
    }

    #[test]
    fn schedule_task_keeps_list_filters_working() {
        let path = temp_path("schedule-list.json");
        let tasks = vec![
            Task {
                id: "task-1".to_string(),
                title: "today".to_string(),
                status: TaskStatus::Pending,
                created_at: "2025-12-01T00:00:00Z".to_string(),
                scheduled_at: None,
                completed_at: None,
                completion_history: Vec::new(),
                urgent: false,
            },
            Task {
                id: "task-2".to_string(),
                title: "future".to_string(),
                status: TaskStatus::Pending,
                created_at: "2025-12-01T00:00:00Z".to_string(),
                scheduled_at: None,
                completed_at: None,
                completion_history: Vec::new(),
                urgent: false,
            },
        ];

        json_store::save_tasks(&path, &tasks).unwrap();

        schedule_task_with_path(&path, "task-1", "2025-12-20T12:00:00Z").unwrap();
        schedule_task_with_path(&path, "task-2", "2025-12-21T09:00:00Z").unwrap();

        let loaded = json_store::load_tasks(&path).unwrap();
        std::fs::remove_file(&path).ok();

        let today = Date::from_calendar_date(2025, Month::December, 20).unwrap();
        let offset = UtcOffset::UTC;

        let today_tasks = filter_tasks(&loaded, today, offset, ListMode::Today).unwrap();
        assert_eq!(today_tasks.len(), 1);
        assert_eq!(today_tasks[0].id, "task-1");

        let backlog_tasks = filter_tasks(&loaded, today, offset, ListMode::Backlog).unwrap();
        assert_eq!(backlog_tasks.len(), 1);
        assert_eq!(backlog_tasks[0].id, "task-2");
    }

    #[test]
    fn reschedule_task_keeps_list_filters_working() {
        let path = temp_path("reschedule-list.json");
        let now = OffsetDateTime::now_utc();
        let past = (now - Duration::days(1)).format(&Rfc3339).unwrap();
        let future = (now + Duration::days(1)).format(&Rfc3339).unwrap();
        let tasks = vec![
            Task {
                id: "task-1".to_string(),
                title: "today".to_string(),
                status: TaskStatus::Pending,
                created_at: "2025-12-01T00:00:00Z".to_string(),
                scheduled_at: Some(past.clone()),
                completed_at: None,
                completion_history: Vec::new(),
                urgent: false,
            },
            Task {
                id: "task-2".to_string(),
                title: "future".to_string(),
                status: TaskStatus::Pending,
                created_at: "2025-12-01T00:00:00Z".to_string(),
                scheduled_at: Some(past),
                completed_at: None,
                completion_history: Vec::new(),
                urgent: false,
            },
        ];

        json_store::save_tasks(&path, &tasks).unwrap();

        reschedule_task_with_path(&path, "task-2", &future).unwrap();

        let loaded = json_store::load_tasks(&path).unwrap();
        std::fs::remove_file(&path).ok();

        let offset = UtcOffset::UTC;
        let today = OffsetDateTime::now_utc().to_offset(offset).date();

        let today_tasks = filter_tasks(&loaded, today, offset, ListMode::Today).unwrap();
        assert_eq!(today_tasks.len(), 1);
        assert_eq!(today_tasks[0].id, "task-1");

        let backlog_tasks = filter_tasks(&loaded, today, offset, ListMode::Backlog).unwrap();
        assert_eq!(backlog_tasks.len(), 1);
        assert_eq!(backlog_tasks[0].id, "task-2");
    }

    #[test]
    fn list_today_backlog_with_scheduled_tasks_smoke() {
        let path = temp_path("list-smoke.json");
        let local_offset = UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC);
        let now_local = OffsetDateTime::now_utc().to_offset(local_offset);
        let today = now_local.date();
        let today_dt = today
            .with_hms(12, 0, 0)
            .unwrap()
            .assume_offset(local_offset);
        let future_dt = (today + Duration::days(2))
            .with_hms(12, 0, 0)
            .unwrap()
            .assume_offset(local_offset);

        let tasks = vec![
            Task {
                id: "task-1".to_string(),
                title: "today".to_string(),
                status: TaskStatus::Pending,
                created_at: now_local.format(&Rfc3339).unwrap(),
                scheduled_at: Some(today_dt.format(&Rfc3339).unwrap()),
                completed_at: None,
                completion_history: Vec::new(),
                urgent: false,
            },
            Task {
                id: "task-2".to_string(),
                title: "future".to_string(),
                status: TaskStatus::Pending,
                created_at: now_local.format(&Rfc3339).unwrap(),
                scheduled_at: Some(future_dt.format(&Rfc3339).unwrap()),
                completed_at: None,
                completion_history: Vec::new(),
                urgent: false,
            },
        ];

        json_store::save_tasks(&path, &tasks).unwrap();

        let today_tasks = list_today_with_focus_with_path(&path).unwrap().tasks;
        let backlog_tasks = list_without_focus(&path, ListMode::Backlog).unwrap();

        std::fs::remove_file(&path).ok();

        assert_eq!(today_tasks.len(), 1);
        assert_eq!(today_tasks[0].id, "task-1");
        assert_eq!(backlog_tasks.len(), 1);
        assert_eq!(backlog_tasks[0].id, "task-2");
    }

    #[test]
    fn list_today_with_focus_orders_focused_task_first() {
        let path = temp_path("list-focus.json");
        let local_offset = UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC);
        let now_local = OffsetDateTime::now_utc().to_offset(local_offset);
        let today = now_local.date();
        let today_dt = today
            .with_hms(12, 0, 0)
            .unwrap()
            .assume_offset(local_offset);

        let tasks = vec![
            Task {
                id: "task-1".to_string(),
                title: "first".to_string(),
                status: TaskStatus::Pending,
                created_at: now_local.format(&Rfc3339).unwrap(),
                scheduled_at: Some(today_dt.format(&Rfc3339).unwrap()),
                completed_at: None,
                completion_history: Vec::new(),
                urgent: false,
            },
            Task {
                id: "task-2".to_string(),
                title: "second".to_string(),
                status: TaskStatus::Pending,
                created_at: now_local.format(&Rfc3339).unwrap(),
                scheduled_at: Some(today_dt.format(&Rfc3339).unwrap()),
                completed_at: None,
                completion_history: Vec::new(),
                urgent: false,
            },
        ];

        json_store::save_state(
            &path,
            &json_store::TaskState {
                tasks,
                focused_task_id: Some("task-2".to_string()),
            },
        )
        .unwrap();

        let result = list_today_with_focus_with_path(&path).unwrap();
        std::fs::remove_file(&path).ok();

        assert_eq!(result.focused_task_id, Some("task-2".to_string()));
        assert_eq!(result.tasks.len(), 2);
        assert_eq!(result.tasks[0].id, "task-2");
    }

    #[test]
    fn list_today_with_focus_omits_focus_when_not_in_list() {
        let path = temp_path("list-focus-missing.json");
        let local_offset = UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC);
        let now_local = OffsetDateTime::now_utc().to_offset(local_offset);
        let today = now_local.date();
        let today_dt = today
            .with_hms(12, 0, 0)
            .unwrap()
            .assume_offset(local_offset);
        let future_dt = (today + Duration::days(1))
            .with_hms(9, 0, 0)
            .unwrap()
            .assume_offset(local_offset);

        let tasks = vec![
            Task {
                id: "task-1".to_string(),
                title: "today".to_string(),
                status: TaskStatus::Pending,
                created_at: now_local.format(&Rfc3339).unwrap(),
                scheduled_at: Some(today_dt.format(&Rfc3339).unwrap()),
                completed_at: None,
                completion_history: Vec::new(),
                urgent: false,
            },
            Task {
                id: "task-2".to_string(),
                title: "future".to_string(),
                status: TaskStatus::Pending,
                created_at: now_local.format(&Rfc3339).unwrap(),
                scheduled_at: Some(future_dt.format(&Rfc3339).unwrap()),
                completed_at: None,
                completion_history: Vec::new(),
                urgent: false,
            },
        ];

        json_store::save_state(
            &path,
            &json_store::TaskState {
                tasks,
                focused_task_id: Some("task-2".to_string()),
            },
        )
        .unwrap();

        let result = list_today_with_focus_with_path(&path).unwrap();
        std::fs::remove_file(&path).ok();

        assert_eq!(result.focused_task_id, Some("task-2".to_string()));
        assert_eq!(result.tasks.len(), 1);
        assert_eq!(result.tasks[0].id, "task-1");
    }

    #[derive(Default)]
    struct MockNotifier {
        notified: RefCell<Vec<(String, String)>>,
    }

    impl Notifier for MockNotifier {
        fn notify(&self, task: &Task) -> Result<(), AppError> {
            self.notify_with_action(task, "")
        }

        fn notify_with_action(&self, task: &Task, action: &str) -> Result<(), AppError> {
            self.notified
                .borrow_mut()
                .push((task.id.clone(), action.to_string()));
            Ok(())
        }
    }

    #[test]
    fn notify_overdue_or_urgent_selects_pending_tasks() {
        let path = temp_path("notify-selects.json");
        let now = OffsetDateTime::now_utc();
        let past = (now - Duration::days(1)).format(&Rfc3339).unwrap();

        let tasks = vec![
            Task {
                id: "task-1".to_string(),
                title: "overdue".to_string(),
                status: TaskStatus::Pending,
                created_at: "2025-12-01T00:00:00Z".to_string(),
                scheduled_at: Some(past.clone()),
                completed_at: None,
                completion_history: Vec::new(),
                urgent: false,
            },
            Task {
                id: "task-2".to_string(),
                title: "urgent".to_string(),
                status: TaskStatus::Pending,
                created_at: "2025-12-01T00:00:00Z".to_string(),
                scheduled_at: None,
                completed_at: None,
                completion_history: Vec::new(),
                urgent: true,
            },
            Task {
                id: "task-3".to_string(),
                title: "normal".to_string(),
                status: TaskStatus::Pending,
                created_at: "2025-12-01T00:00:00Z".to_string(),
                scheduled_at: None,
                completed_at: None,
                completion_history: Vec::new(),
                urgent: false,
            },
            Task {
                id: "task-4".to_string(),
                title: "done".to_string(),
                status: TaskStatus::Completed,
                created_at: "2025-12-01T00:00:00Z".to_string(),
                scheduled_at: Some(past),
                completed_at: Some("2025-12-02T00:00:00Z".to_string()),
                completion_history: Vec::new(),
                urgent: true,
            },
        ];

        json_store::save_tasks(&path, &tasks).unwrap();

        let notifier = MockNotifier::default();
        let outcome = notify_overdue_or_urgent_with_path(&path, &notifier).unwrap();
        std::fs::remove_file(&path).ok();

        let ids = notifier.notified.borrow().clone();
        assert_eq!(
            ids,
            vec![
                ("task-1".to_string(), "show:task-1".to_string()),
                ("task-2".to_string(), "show:task-2".to_string())
            ]
        );
        assert!(outcome.failures.is_empty());
        assert_eq!(outcome.tasks.len(), 2);
        assert_eq!(outcome.tasks[0].id, "task-1");
        assert_eq!(outcome.tasks[1].id, "task-2");
    }

    #[test]
    fn notify_overdue_or_urgent_returns_empty_when_none() {
        let path = temp_path("notify-none.json");
        let future = (OffsetDateTime::now_utc() + Duration::days(1))
            .format(&Rfc3339)
            .unwrap();
        let tasks = vec![Task {
            id: "task-1".to_string(),
            title: "future".to_string(),
            status: TaskStatus::Pending,
            created_at: "2025-12-01T00:00:00Z".to_string(),
            scheduled_at: Some(future),
            completed_at: None,
            completion_history: Vec::new(),
            urgent: false,
        }];

        json_store::save_tasks(&path, &tasks).unwrap();

        let notifier = MockNotifier::default();
        let outcome = notify_overdue_or_urgent_with_path(&path, &notifier).unwrap();
        std::fs::remove_file(&path).ok();

        assert!(notifier.notified.borrow().is_empty());
        assert!(outcome.failures.is_empty());
        assert!(outcome.tasks.is_empty());
    }

    struct FailingNotifier;

    impl Notifier for FailingNotifier {
        fn notify(&self, _task: &Task) -> Result<(), AppError> {
            Err(AppError::io("no display"))
        }

        fn notify_with_action(&self, _task: &Task, _action: &str) -> Result<(), AppError> {
            Err(AppError::io("no display"))
        }
    }

    #[test]
    fn notify_overdue_or_urgent_reports_failures() {
        let path = temp_path("notify-failures.json");
        let now = OffsetDateTime::now_utc();
        let past = (now - Duration::days(1)).format(&Rfc3339).unwrap();

        let tasks = vec![Task {
            id: "task-urgent".to_string(),
            title: "urgent".to_string(),
            status: TaskStatus::Pending,
            created_at: "2025-12-01T00:00:00Z".to_string(),
            scheduled_at: Some(past),
            completed_at: None,
            completion_history: Vec::new(),
            urgent: true,
        }];

        json_store::save_tasks(&path, &tasks).unwrap();

        let notifier = FailingNotifier;
        let outcome = notify_overdue_or_urgent_with_path(&path, &notifier).unwrap();
        std::fs::remove_file(&path).ok();

        assert!(outcome.tasks.is_empty());
        assert_eq!(outcome.failures.len(), 1);
        assert_eq!(outcome.failures[0].task_id, "task-urgent");
        assert!(outcome.failures[0].error.message().contains("no display"));
    }
}
