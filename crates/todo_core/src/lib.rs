pub mod config;
pub mod error;
pub mod model;
pub mod notify;
pub mod storage;
pub mod task_api;

#[cfg(test)]
mod tests {
    use crate::error::AppError;
    use crate::model::{Task, TaskStatus};

    #[test]
    fn task_has_required_fields() {
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

        assert_eq!(task.id, "task-1");
        assert_eq!(task.title, "demo");
        assert_eq!(task.status, TaskStatus::Pending);
        assert_eq!(task.created_at, "2025-12-20T00:00:00Z");
        assert_eq!(task.scheduled_at, None);
        assert_eq!(task.completed_at, None);
        assert!(task.completion_history.is_empty());
        assert!(!task.urgent);
    }

    #[test]
    fn app_error_exposes_code() {
        let err = AppError::invalid_input("missing title");
        assert_eq!(err.code(), "invalid_input");
    }
}
