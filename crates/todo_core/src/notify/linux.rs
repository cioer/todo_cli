use crate::error::AppError;
use crate::model::Task;
use crate::notify::{Notifier, launch_show};
use notify_rust::Notification;

pub struct LinuxNotifier;

impl Notifier for LinuxNotifier {
    fn notify(&self, task: &Task) -> Result<(), AppError> {
        self.notify_with_action(task, "")
    }

    fn notify_with_action(&self, task: &Task, action: &str) -> Result<(), AppError> {
        let mut notification = Notification::new();
        notification.summary("todoapp");
        notification.body(&format!("{} ({})", task.title, task.id));
        if !action.trim().is_empty() {
            notification.action(action, "Open");
        }

        let handle = notification
            .show()
            .map_err(|err| AppError::io(err.to_string()))?;

        if !action.trim().is_empty() {
            let action_key = action.to_string();
            let task_id = task.id.clone();
            std::thread::spawn(move || {
                let _ = handle.wait_for_action(|selected| {
                    if selected == action_key || selected == "default" {
                        let _ = launch_show(&task_id);
                    }
                });
            });
        }

        Ok(())
    }
}
