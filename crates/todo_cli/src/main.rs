use clap::{CommandFactory, Parser};
use std::collections::{HashMap, HashSet};
use std::io::{self, BufRead};
use todo_cli::cli::{
    CONFIG_OVERRIDE_FLAG, Cli, Command, ConfigOverrideTarget, ListCommand, ParsedConfigOverride,
    parse_config_override,
};
use todo_core::config::{
    Config, ConfigOverrides, Palette, canonical_theme_name, merge_overrides, palette_for_theme,
};
use todo_core::error::AppError;
use todo_core::model::{Task, TaskStatus};

fn status_label(status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Pending => "pending",
        TaskStatus::Completed => "completed",
    }
}

fn print_tasks_plain(
    tasks: &[Task],
    focused_task_id: Option<&str>,
    palette: &Palette,
) -> Result<(), AppError> {
    for task in tasks {
        let prefix = if focused_task_id == Some(task.id.as_str()) {
            palette.accentize("[FOCUS] ")
        } else {
            String::new()
        };
        let scheduled_at = task.scheduled_at.as_deref().unwrap_or("-");
        let scheduled_display = palette.mutedize(scheduled_at);
        let overdue = todo_core::task_api::task_overdue(task)?;
        let status = if overdue {
            format!("{} (overdue)", status_label(task.status))
        } else {
            status_label(task.status).to_string()
        };
        let title = palette.accentize(&task.title);
        let status_text = palette.accentize(&status);
        println!(
            "{}{} | {} | {} | {} | {}",
            prefix, task.id, title, status_text, task.created_at, scheduled_display
        );
    }

    Ok(())
}

fn resolve_aliases(mut args: Vec<String>, config: &Config) -> Result<Vec<String>, AppError> {
    loop {
        if args.is_empty() {
            break;
        }

        let alias_index = args
            .iter()
            .position(|arg| !arg.starts_with('-'))
            .unwrap_or(usize::MAX);
        if alias_index == usize::MAX {
            break;
        }

        let alias_name = args[alias_index].clone();
        let replacement = match config.aliases.get(&alias_name) {
            Some(value) => value,
            None => break,
        };

        let alias_tokens = parse_command_line(replacement).map_err(|err| {
            AppError::invalid_data(format!(
                "alias '{alias}' invalid: {error}",
                alias = alias_name,
                error = err
            ))
        })?;

        let mut resolved = Vec::new();
        resolved.extend(args[..alias_index].iter().cloned());
        resolved.extend(alias_tokens);
        resolved.extend(args[(alias_index + 1)..].iter().cloned());
        args = resolved;
    }

    Ok(args)
}

// Parse a command-line string using the same quoting rules as interactive mode.
fn parse_command_line(line: &str) -> Result<Vec<String>, AppError> {
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

fn validate_alias_definitions(aliases: &HashMap<String, String>) -> Result<(), AppError> {
    for (alias, replacement) in aliases {
        let tokens = parse_command_line(replacement)
            .map_err(|err| AppError::invalid_input(format!("alias '{alias}' invalid: {err}")))?;
        if tokens.is_empty() {
            return Err(AppError::invalid_input(format!(
                "alias '{alias}' has empty expansion"
            )));
        }
    }

    for alias in aliases.keys() {
        let mut visited = HashSet::new();
        let mut current = alias.as_str();

        loop {
            if !visited.insert(current) {
                return Err(AppError::invalid_input(format!(
                    "alias '{}' introduces a cycle",
                    alias
                )));
            }

            let expansion = match aliases.get(current) {
                Some(value) => value,
                None => break,
            };

            let first_token = expansion.split_whitespace().next().unwrap_or("");
            if first_token.is_empty() {
                break;
            }

            if first_token == current {
                return Err(AppError::invalid_input(format!(
                    "alias '{}' expands to itself",
                    alias
                )));
            }

            if !aliases.contains_key(first_token) {
                break;
            }

            current = first_token;
        }
    }

    Ok(())
}

fn print_tasks_json(tasks: &[Task]) -> Result<(), AppError> {
    let mut payload = Vec::with_capacity(tasks.len());
    for task in tasks {
        let overdue = todo_core::task_api::task_overdue(task)?;
        let status = if overdue {
            format!("{} (overdue)", status_label(task.status))
        } else {
            status_label(task.status).to_string()
        };
        payload.push(serde_json::json!({
            "id": task.id,
            "title": task.title,
            "status": status,
            "created_at": task.created_at,
            "scheduled_at": task.scheduled_at,
        }));
    }
    println!("{}", serde_json::Value::Array(payload));
    Ok(())
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

fn print_task_json_with_overdue(task: &Task) -> Result<(), AppError> {
    let overdue = todo_core::task_api::task_overdue(task)?;
    let status = if overdue {
        format!("{} (overdue)", status_label(task.status))
    } else {
        status_label(task.status).to_string()
    };
    let json = serde_json::json!({
        "id": task.id,
        "title": task.title,
        "status": status,
        "created_at": task.created_at,
        "scheduled_at": task.scheduled_at,
    });
    println!("{}", json);
    Ok(())
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

fn print_help() {
    let mut cmd = Cli::command();
    let help = cmd.render_help();
    println!("{help}");
}

fn run_command(cli: Cli, palette: &Palette) -> Result<(), AppError> {
    match cli.command {
        Command::Add { title, urgent } => {
            let title = match title {
                Some(value) if !value.trim().is_empty() => value,
                _ => return Err(AppError::invalid_input("title is required")),
            };

            let task = todo_core::task_api::add_task_with_urgency(&title, urgent)?;
            if cli.json {
                print_task_json(&task);
            } else {
                let title_display = palette.accentize(&task.title);
                println!("Added task: {} ({})", title_display, task.id);
            }
        }
        Command::Focus { id } => {
            let task = todo_core::task_api::set_focus(&id)?;
            if cli.json {
                print_task_json(&task);
            } else {
                let title_display = palette.accentize(&task.title);
                println!("Focused task: {} ({})", title_display, task.id);
            }
        }
        Command::Urgent { id, clear } => {
            let task = todo_core::task_api::set_task_urgent(&id, !clear)?;
            if cli.json {
                print_task_json(&task);
            } else {
                let title_display = palette.accentize(&task.title);
                let action = if clear {
                    "Cleared urgent flag"
                } else {
                    "Marked as urgent"
                };
                println!("{action}: {} ({})", title_display, task.id);
            }
        }
        Command::Edit { id, new_title } => {
            let task = todo_core::task_api::edit_task(&id, &new_title)?;
            if cli.json {
                print_task_json(&task);
            } else {
                let title_display = palette.accentize(&task.title);
                println!("Updated task: {} ({})", title_display, task.id);
            }
        }
        Command::Delete { id } => {
            let task = todo_core::task_api::delete_task(&id)?;
            if cli.json {
                print_task_json(&task);
            } else {
                let title_display = palette.accentize(&task.title);
                println!("Deleted task: {} ({})", title_display, task.id);
            }
        }
        Command::Show { id } => {
            let task = todo_core::task_api::get_task_by_id(&id)?;
            if cli.json {
                print_task_json_with_overdue(&task)?;
            } else {
                print_tasks_plain(std::slice::from_ref(&task), None, palette)?;
            }
        }
        Command::Done {
            id,
            message,
            message_flag,
        } => {
            let task = match id {
                Some(id) => {
                    if message.is_some() && message_flag.is_some() {
                        return Err(AppError::invalid_input("message provided twice"));
                    }
                    let message_input = message.as_deref().or(message_flag.as_deref());
                    todo_core::task_api::complete_task(&id, message_input)?
                }
                None => todo_core::task_api::complete_focused_task(message_flag.as_deref())?,
            };
            if cli.json {
                print_completed_task_json(&task);
            } else {
                let title_display = palette.accentize(&task.title);
                println!("Completed task: {} ({})", title_display, task.id);
            }
        }
        Command::Schedule { id, datetime } => {
            let task = todo_core::task_api::schedule_task(&id, &datetime)?;
            if cli.json {
                print_task_json(&task);
            } else {
                let scheduled_at = task.scheduled_at.as_deref().unwrap_or("-");
                let title_display = palette.accentize(&task.title);
                let scheduled_display = palette.mutedize(scheduled_at);
                println!(
                    "Scheduled task: {} ({}) at {}",
                    title_display, task.id, scheduled_display
                );
            }
        }
        Command::Reschedule { id, datetime } => {
            let task = todo_core::task_api::reschedule_task(&id, &datetime)?;
            if cli.json {
                print_task_json(&task);
            } else {
                let scheduled_at = task.scheduled_at.as_deref().unwrap_or("-");
                let title_display = palette.accentize(&task.title);
                let scheduled_display = palette.mutedize(scheduled_at);
                println!(
                    "Rescheduled task: {} ({}) at {}",
                    title_display, task.id, scheduled_display
                );
            }
        }
        Command::Notify => {
            let outcome = todo_core::task_api::notify_overdue_or_urgent()?;
            if !outcome.failures.is_empty() {
                for failure in &outcome.failures {
                    eprintln!(
                        "WARNING: Unable to notify {}: {}",
                        failure.task_id, failure.error
                    );
                }
            }
            let tasks = outcome.tasks;
            if cli.json {
                print_tasks_json(&tasks)?;
            } else if tasks.is_empty() {
                println!("No notifications sent.");
            } else {
                for task in tasks {
                    let title_display = palette.accentize(&task.title);
                    println!("Notified task: {} ({})", title_display, task.id);
                }
            }
        }
        Command::List { list } => match list {
            ListCommand::Today => {
                let result = todo_core::task_api::list_today_with_focus()?;
                if cli.json {
                    print_tasks_json(&result.tasks)?;
                } else {
                    print_tasks_plain(&result.tasks, result.focused_task_id.as_deref(), palette)?;
                }
            }
            ListCommand::Backlog => {
                let tasks = todo_core::task_api::list_backlog()?;
                if cli.json {
                    print_tasks_json(&tasks)?;
                } else {
                    print_tasks_plain(&tasks, None, palette)?;
                }
            }
        },
    }

    Ok(())
}

fn run_interactive(config: &Config, palette: &Palette) -> Result<(), AppError> {
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

        let args = match parse_command_line(line) {
            Ok(args) => args,
            Err(err) => {
                eprintln!("ERROR: {}", err);
                continue;
            }
        };

        if args.is_empty() {
            continue;
        }

        let args = match resolve_aliases(args, config) {
            Ok(resolved) => resolved,
            Err(err) => {
                eprintln!("ERROR: {}", err);
                continue;
            }
        };
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

        if let Err(err) = run_command(cli, palette) {
            eprintln!("ERROR: {}", err);
        }
    }

    Ok(())
}

fn main() {
    let config_load = todo_core::config::load_config_with_fallback();
    if let Some(err) = config_load.error.as_ref() {
        eprintln!("ERROR: {}", err);
    }

    let raw_args: Vec<String> = std::env::args_os()
        .skip(1)
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();

    let (filtered_args, override_entries, override_tokens) =
        match extract_config_override_args(&raw_args) {
            Ok(tuple) => tuple,
            Err(err) => {
                eprintln!("ERROR: {}", err);
                std::process::exit(1);
            }
        };

    let overrides = build_config_overrides(&override_entries);
    let effective_config = merge_overrides(&config_load.config, &overrides);
    let palette = palette_for_theme(effective_config.theme.as_deref());

    if let Err(err) = validate_alias_definitions(&effective_config.aliases) {
        eprintln!("ERROR: {}", err);
        std::process::exit(1);
    }

    if filtered_args.is_empty() {
        if let Err(err) = run_interactive(&effective_config, &palette) {
            eprintln!("ERROR: {}", err);
            std::process::exit(1);
        }
        return;
    }

    let parsed_args = match resolve_aliases(filtered_args, &effective_config) {
        Ok(args) => args,
        Err(err) => {
            eprintln!("ERROR: {}", err);
            std::process::exit(1);
        }
    };

    let insert_index = parsed_args
        .iter()
        .position(|arg| arg == "--")
        .unwrap_or(parsed_args.len());

    let mut cli_argv = Vec::with_capacity(parsed_args.len() + override_tokens.len() + 1);
    cli_argv.push("todo".to_string());
    cli_argv.extend(parsed_args[..insert_index].iter().cloned());
    cli_argv.extend(override_tokens.iter().cloned());
    cli_argv.extend(parsed_args[insert_index..].iter().cloned());

    let cli = match Cli::try_parse_from(cli_argv) {
        Ok(cli) => cli,
        Err(err) => {
            eprintln!("ERROR: {}", normalize_parse_error(err));
            std::process::exit(1);
        }
    };

    if let Err(err) = run_command(cli, &palette) {
        eprintln!("ERROR: {}", err);
        std::process::exit(1);
    }
}

fn extract_config_override_args(
    raw_args: &[String],
) -> Result<(Vec<String>, Vec<ParsedConfigOverride>, Vec<String>), AppError> {
    let mut filtered = Vec::new();
    let mut overrides = Vec::new();
    let mut tokens = Vec::new();
    let prefix = format!("{CONFIG_OVERRIDE_FLAG}=");
    let mut iter = raw_args.iter();

    while let Some(arg) = iter.next() {
        if arg == CONFIG_OVERRIDE_FLAG {
            let value = iter
                .next()
                .ok_or_else(|| AppError::invalid_input("missing value for --config-override"))?;
            let parsed = parse_config_override(value).map_err(|msg| {
                AppError::invalid_input(format!("--config-override invalid: {msg}"))
            })?;
            overrides.push(parsed);
            tokens.push(arg.clone());
            tokens.push(value.clone());
        } else if let Some(value) = arg.strip_prefix(&prefix) {
            let parsed = parse_config_override(value).map_err(|msg| {
                AppError::invalid_input(format!("--config-override invalid: {msg}"))
            })?;
            overrides.push(parsed);
            tokens.push(arg.clone());
        } else {
            filtered.push(arg.clone());
        }
    }

    Ok((filtered, overrides, tokens))
}

fn build_config_overrides(entries: &[ParsedConfigOverride]) -> ConfigOverrides {
    let mut overrides = ConfigOverrides::default();
    for entry in entries {
        match &entry.target {
            ConfigOverrideTarget::Theme => {
                if let Some(normalized) = canonical_theme_name(&entry.value) {
                    overrides.theme = Some(normalized);
                }
            }
            ConfigOverrideTarget::Alias(name) => {
                overrides.aliases.insert(name.clone(), entry.value.clone());
            }
        }
    }
    overrides
}
