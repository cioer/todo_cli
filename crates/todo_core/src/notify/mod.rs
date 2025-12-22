use crate::error::AppError;
use crate::model::Task;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::LinuxNotifier;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::WindowsNotifier;

pub trait Notifier {
    fn notify(&self, task: &Task) -> Result<(), AppError>;

    fn notify_with_action(&self, task: &Task, action: &str) -> Result<(), AppError> {
        let _ = action;
        self.notify(task)
    }
}

pub struct NoopNotifier;

impl Notifier for NoopNotifier {
    fn notify(&self, _task: &Task) -> Result<(), AppError> {
        Ok(())
    }
}

pub fn notifier_from_env() -> Result<Box<dyn Notifier>, AppError> {
    if std::env::var("TODOAPP_DISABLE_NOTIFICATIONS").is_ok() {
        return Ok(Box::new(NoopNotifier));
    }

    match platform_notifier() {
        Ok(notifier) => Ok(notifier),
        Err(err) => match err {
            AppError::InvalidData(_) => Ok(Box::new(NoopNotifier)),
            other => Err(other),
        },
    }
}

const ACTION_PREFIX: &str = "show:";

pub fn activation_argument(task_id: &str) -> String {
    format!("{ACTION_PREFIX}{task_id}")
}

pub fn parse_activation_argument(argument: &str) -> Option<String> {
    argument
        .strip_prefix(ACTION_PREFIX)
        .map(|id| id.to_string())
}

pub fn launch_show(task_id: &str) -> Result<(), AppError> {
    let exe = std::env::current_exe().map_err(|err| AppError::io(err.to_string()))?;
    std::process::Command::new(exe)
        .arg("show")
        .arg(task_id)
        .spawn()
        .map_err(|err| AppError::io(err.to_string()))?;
    Ok(())
}

#[cfg(target_os = "linux")]
pub fn platform_notifier() -> Result<Box<dyn Notifier>, AppError> {
    Ok(Box::new(LinuxNotifier))
}

#[cfg(windows)]
pub fn platform_notifier() -> Result<Box<dyn Notifier>, AppError> {
    Ok(Box::new(WindowsNotifier))
}

#[cfg(not(any(target_os = "linux", windows)))]
pub fn platform_notifier() -> Result<Box<dyn Notifier>, AppError> {
    Err(AppError::invalid_data(
        "notifications are not supported on this platform",
    ))
}

#[cfg(test)]
mod tests {
    use super::{activation_argument, parse_activation_argument};

    #[test]
    fn activation_argument_round_trip() {
        let argument = activation_argument("task-1");
        let parsed = parse_activation_argument(&argument);
        assert_eq!(parsed.as_deref(), Some("task-1"));
    }

    #[test]
    fn parse_activation_argument_rejects_other_values() {
        assert!(parse_activation_argument("other:task-1").is_none());
    }
}
