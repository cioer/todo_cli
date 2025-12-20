use clap::{CommandFactory, Parser};
use std::io::{self, BufRead};
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

fn normalize_parse_error(err: clap::Error) -> AppError {
    let rendered = err.to_string();
    let first_line = rendered.lines().next().unwrap_or("invalid command").trim();
    let message = first_line
        .strip_prefix("error: ")
        .unwrap_or(first_line)
        .to_string();
    AppError::invalid_input(message)
}

fn split_command_line(line: &str) -> Result<Vec<String>, AppError> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut escape = false;

    for ch in line.chars() {
        if escape {
            if ch != '"' && ch != '\\' {
                current.push('\\');
            }
            current.push(ch);
            escape = false;
            continue;
        }

        if in_quotes && ch == '\\' {
            escape = true;
            continue;
        }

        if ch == '"' {
            in_quotes = !in_quotes;
            continue;
        }

        if ch.is_whitespace() && !in_quotes {
            if !current.is_empty() {
                args.push(current.clone());
                current.clear();
            }
            continue;
        }

        current.push(ch);
    }

    if in_quotes {
        return Err(AppError::invalid_input("unterminated quote in command"));
    }

    if !current.is_empty() {
        args.push(current);
    }

    Ok(args)
}

fn print_help() {
    let mut cmd = Cli::command();
    let help = cmd.render_help();
    println!("{help}");
}

fn run_command(cli: Cli) -> Result<(), AppError> {
    match cli.command {
        Command::Add { title } => {
            let title = match title {
                Some(value) if !value.trim().is_empty() => value,
                _ => return Err(AppError::invalid_input("title is required")),
            };

            let task = todo_core::task_api::add_task(&title)?;
            if cli.json {
                print_task_json(&task);
            } else {
                println!("Added task: {} ({})", task.title, task.id);
            }
        }
        Command::Edit { id, new_title } => {
            let task = todo_core::task_api::edit_task(&id, &new_title)?;
            if cli.json {
                print_task_json(&task);
            } else {
                println!("Updated task: {} ({})", task.title, task.id);
            }
        }
        Command::Delete { id } => {
            let task = todo_core::task_api::delete_task(&id)?;
            if cli.json {
                print_task_json(&task);
            } else {
                println!("Deleted task: {} ({})", task.title, task.id);
            }
        }
        Command::Done { id, message } => {
            let task = todo_core::task_api::complete_task(&id, message.as_deref())?;
            if cli.json {
                print_completed_task_json(&task);
            } else {
                println!("Completed task: {} ({})", task.title, task.id);
            }
        }
        Command::List { list } => {
            let tasks = match list {
                ListCommand::Today => todo_core::task_api::list_today()?,
                ListCommand::Backlog => todo_core::task_api::list_backlog()?,
            };

            if cli.json {
                print_tasks_json(&tasks);
            } else {
                print_tasks_plain(&tasks);
            }
        }
    }

    Ok(())
}

fn run_interactive() -> Result<(), AppError> {
    let mut input = String::new();
    let stdin = io::stdin();
    let mut stdin_lock = stdin.lock();

    loop {
        input.clear();
        let bytes = stdin_lock
            .read_line(&mut input)
            .map_err(|err| AppError::io(err.to_string()))?;

        if bytes == 0 {
            break;
        }

        let line = input.trim();
        if line.is_empty() {
            continue;
        }

        if line.eq_ignore_ascii_case("exit") || line.eq_ignore_ascii_case("quit") {
            break;
        }

        if line == "help" || line == "?" {
            print_help();
            continue;
        }

        let args = match split_command_line(line) {
            Ok(args) => args,
            Err(err) => {
                eprintln!("ERROR: {}", err);
                continue;
            }
        };

        if args.is_empty() {
            continue;
        }

        let mut argv = Vec::with_capacity(args.len() + 1);
        argv.push("todo".to_string());
        argv.extend(args);

        let cli = match Cli::try_parse_from(argv) {
            Ok(cli) => cli,
            Err(err) => {
                eprintln!("ERROR: {}", normalize_parse_error(err));
                continue;
            }
        };

        if let Err(err) = run_command(cli) {
            eprintln!("ERROR: {}", err);
        }
    }

    Ok(())
}

fn main() {
    let mut args = std::env::args_os();
    args.next();
    if args.next().is_none() {
        if let Err(err) = run_interactive() {
            eprintln!("ERROR: {}", err);
            std::process::exit(1);
        }
        return;
    }

    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(err) => {
            eprintln!("ERROR: {}", normalize_parse_error(err));
            std::process::exit(1);
        }
    };

    if let Err(err) = run_command(cli) {
        eprintln!("ERROR: {}", err);
        std::process::exit(1);
    }
}
