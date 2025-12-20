use clap::Parser;
use todo_cli::cli::{Cli, Command, ListCommand};
use todo_core::error::AppError;
use todo_core::model::{Task, TaskStatus};

fn status_label(status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Pending => "pending",
        TaskStatus::Completed => "completed",
    }
}

fn print_tasks_plain(tasks: &[Task]) {
    for task in tasks {
        let scheduled_at = task.scheduled_at.as_deref().unwrap_or("-");
        println!(
            "{} | {} | {} | {} | {}",
            task.id,
            task.title,
            status_label(task.status),
            task.created_at,
            scheduled_at
        );
    }
}

fn print_tasks_json(tasks: &[Task]) {
    let payload: Vec<_> = tasks
        .iter()
        .map(|task| {
            serde_json::json!({
                "id": task.id,
                "title": task.title,
                "status": task.status,
                "created_at": task.created_at,
                "scheduled_at": task.scheduled_at,
            })
        })
        .collect();
    println!("{}", serde_json::Value::Array(payload));
}

fn print_task_json(task: &Task) {
    let json = serde_json::json!({
        "id": task.id,
        "title": task.title,
        "status": task.status,
        "created_at": task.created_at,
        "scheduled_at": task.scheduled_at,
    });
    println!("{}", json);
}

fn print_completed_task_json(task: &Task) {
    let json = serde_json::json!({
        "id": task.id,
        "title": task.title,
        "status": task.status,
        "created_at": task.created_at,
        "scheduled_at": task.scheduled_at,
        "completed_at": task.completed_at,
        "completion_history": task.completion_history,
    });
    println!("{}", json);
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Add { title } => {
            let title = match title {
                Some(value) if !value.trim().is_empty() => value,
                _ => {
                    let err = AppError::invalid_input("title is required");
                    eprintln!("ERROR: {}", err);
                    std::process::exit(1);
                }
            };

            match todo_core::task_api::add_task(&title) {
                Ok(task) => {
                    if cli.json {
                        print_task_json(&task);
                    } else {
                        println!("Added task: {} ({})", task.title, task.id);
                    }
                }
                Err(err) => {
                    eprintln!("ERROR: {}", err);
                    std::process::exit(1);
                }
            }
        }
        Command::Edit { id, new_title } => match todo_core::task_api::edit_task(&id, &new_title) {
            Ok(task) => {
                if cli.json {
                    print_task_json(&task);
                } else {
                    println!("Updated task: {} ({})", task.title, task.id);
                }
            }
            Err(err) => {
                eprintln!("ERROR: {}", err);
                std::process::exit(1);
            }
        },
        Command::Delete { id } => match todo_core::task_api::delete_task(&id) {
            Ok(task) => {
                if cli.json {
                    print_task_json(&task);
                } else {
                    println!("Deleted task: {} ({})", task.title, task.id);
                }
            }
            Err(err) => {
                eprintln!("ERROR: {}", err);
                std::process::exit(1);
            }
        },
        Command::Done { id, message } => match todo_core::task_api::complete_task(&id, message.as_deref()) {
            Ok(task) => {
                if cli.json {
                    print_completed_task_json(&task);
                } else {
                    println!("Completed task: {} ({})", task.title, task.id);
                }
            }
            Err(err) => {
                eprintln!("ERROR: {}", err);
                std::process::exit(1);
            }
        },
        Command::List { list } => {
            let result = match list {
                ListCommand::Today => todo_core::task_api::list_today(),
                ListCommand::Backlog => todo_core::task_api::list_backlog(),
            };

            match result {
                Ok(tasks) => {
                    if cli.json {
                        print_tasks_json(&tasks);
                    } else {
                        print_tasks_plain(&tasks);
                    }
                }
                Err(err) => {
                    eprintln!("ERROR: {}", err);
                    std::process::exit(1);
                }
            }
        }
    }
}
