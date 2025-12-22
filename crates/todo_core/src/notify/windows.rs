use crate::error::AppError;
use crate::model::Task;
use crate::notify::{Notifier, launch_show, parse_activation_argument};
use tauri_winrt_notification::Toast;

pub struct WindowsNotifier;

impl Notifier for WindowsNotifier {
    fn notify(&self, task: &Task) -> Result<(), AppError> {
        self.notify_with_action(task, "")
    }

    fn notify_with_action(&self, task: &Task, action: &str) -> Result<(), AppError> {
        let task_id = task.id.clone();
        let action_value = action.to_string();
        let mut toast = Toast::new(Toast::POWERSHELL_APP_ID)
            .title("todoapp")
            .text1(&task.title)
            .text2(&task.id);

        if !action_value.trim().is_empty() {
            toast = toast.add_button("Open", &action_value);
        }

        let action_match = action_value.clone();
        toast
            .on_activated(move |args| {
                match args {
                    Some(args) => {
                        if !action_match.is_empty() && args == action_match {
                            let _ = launch_show(&task_id);
                        } else if let Some(id) = parse_activation_argument(&args) {
                            let _ = launch_show(&id);
                        } else if args.trim().is_empty() {
                            let _ = launch_show(&task_id);
                        }
                    }
                    None => {
                        let _ = launch_show(&task_id);
                    }
                }
                Ok(())
            })
            .show()
            .map_err(|err| AppError::io(err.to_string()))?;
        Ok(())
    }
}
