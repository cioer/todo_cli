use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// Output JSON
    #[arg(long, global = true)]
    pub json: bool,

    /// Override configuration values (format KEY=VALUE)
    #[arg(long = "config-override", value_name = "KEY=VALUE", global = true)]
    pub config_override: Vec<String>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Add a new task
    ///
    /// Example: todo add "Buy milk" --urgent
    Add {
        title: Option<String>,
        #[arg(long)]
        urgent: bool,
    },
    /// Focus on a specific task
    ///
    /// Example: todo focus 1
    Focus {
        id: String,
    },
    /// Mark a task as urgent or clear urgency
    ///
    /// Example: todo urgent 1
    /// Example: todo urgent 1 --clear
    Urgent {
        id: String,
        #[arg(long)]
        clear: bool,
    },
    /// Edit a task's title
    ///
    /// Example: todo edit 1 "Buy organic milk"
    Edit {
        id: String,
        new_title: String,
    },
    /// Delete a task
    ///
    /// Example: todo delete 1
    Delete {
        id: String,
    },
    /// Show details of a task
    ///
    /// Example: todo show 1
    Show {
        id: String,
    },
    /// Mark a task as completed
    ///
    /// Example: todo done 1
    /// Example: todo done 1 -m "Bought from local store"
    Done {
        id: Option<String>,
        message: Option<String>,
        #[arg(short = 'm', long = "message", value_name = "MESSAGE")]
        message_flag: Option<String>,
    },
    /// Schedule a task for a specific time
    ///
    /// Example: todo schedule 1 "2023-12-25 10:00"
    /// Example: todo schedule 1 "2023-12-25 10:00:00"
    /// Example: todo schedule 1 "2023-12-25" (Defaults to midnight)
    Schedule {
        id: String,
        datetime: String,
    },
    /// Reschedule a task
    ///
    /// Example: todo reschedule 1 "2023-12-26 14:00"
    /// Example: todo reschedule 1 "2023-12-26 14:00:00"
    /// Example: todo reschedule 1 "2023-12-26" (Defaults to midnight)
    Reschedule {
        id: String,
        datetime: String,
    },
    /// Send notifications for due tasks
    ///
    /// Example: todo notify
    Notify,
    /// List tasks
    ///
    /// Example: todo list today
    /// Example: todo list backlog
    List {
        #[command(subcommand)]
        list: ListCommand,
    },
}

#[derive(Subcommand, Debug)]
pub enum ListCommand {
    /// List tasks scheduled for today
    ///
    /// Example: todo list today
    Today,
    /// List backlog tasks
    ///
    /// Example: todo list backlog
    Backlog,
}

/// Flag name used to identify config override arguments by the runtime.
pub const CONFIG_OVERRIDE_FLAG: &str = "--config-override";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigOverrideTarget {
    Theme,
    Alias(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedConfigOverride {
    pub target: ConfigOverrideTarget,
    pub value: String,
}

/// Parse a raw `KEY=VALUE` override string into a structured target.
pub fn parse_config_override(raw: &str) -> Result<ParsedConfigOverride, String> {
    let trimmed = raw.trim();
    let (key_raw, value_raw) = trimmed
        .split_once('=')
        .ok_or_else(|| "override must be in KEY=VALUE format".to_string())?;

    let value = value_raw.trim().to_string();
    let (field, remainder) = key_raw
        .split_once('.')
        .map(|(field, rest)| (field.trim(), Some(rest.trim())))
        .unwrap_or((key_raw.trim(), None));

    let canonical_field =
        canonicalize_flag_name(field).ok_or_else(|| "override key cannot be empty".to_string())?;

    match canonical_field.as_str() {
        "theme" => {
            if remainder.is_some() {
                Err("theme override cannot have subfields".to_string())
            } else {
                Ok(ParsedConfigOverride {
                    target: ConfigOverrideTarget::Theme,
                    value,
                })
            }
        }
        "aliases" | "alias" => {
            let alias_name = remainder
                .filter(|segment| !segment.is_empty())
                .ok_or_else(|| "aliases override requires an alias name".to_string())?;
            Ok(ParsedConfigOverride {
                target: ConfigOverrideTarget::Alias(alias_name.to_string()),
                value,
            })
        }
        other => Err(format!("unknown config field '{other}'")),
    }
}

fn canonicalize_flag_name(name: &str) -> Option<String> {
    let mut cleaned = String::new();
    let mut previous_underscore = false;

    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            cleaned.push(ch.to_ascii_lowercase());
            previous_underscore = false;
        } else if !previous_underscore && !cleaned.is_empty() {
            cleaned.push('_');
            previous_underscore = true;
        }
    }

    let trimmed = cleaned.trim_matches('_');
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::{ConfigOverrideTarget, parse_config_override};

    #[test]
    fn parse_config_override_canonicalizes_field_names() {
        let parsed = parse_config_override(" THEME = Midnight ").unwrap();

        match parsed.target {
            ConfigOverrideTarget::Theme => {}
            other => panic!("unexpected target: {other:?}"),
        }

        assert_eq!(parsed.value, "Midnight");
    }

    #[test]
    fn parse_config_override_rejects_empty_alias_name() {
        let err = parse_config_override("aliases. = foo").unwrap_err();
        assert!(err.contains("aliases override requires an alias name"));
    }

    #[test]
    fn parse_config_override_rejects_unknown_fields() {
        let err = parse_config_override("unknown.field=value").unwrap_err();
        assert!(err.contains("unknown config field"));
    }

    #[test]
    fn parse_config_override_rejects_missing_equals() {
        let err = parse_config_override("aliasesls").unwrap_err();
        assert!(err.contains("KEY=VALUE"));
    }

    #[test]
    fn parse_config_override_trims_whitespace_for_alias_names() {
        let parsed = parse_config_override("aliases. ls = show today").unwrap();

        match parsed.target {
            ConfigOverrideTarget::Alias(alias) => assert_eq!(alias, "ls"),
            other => panic!("unexpected target: {other:?}"),
        }

        assert_eq!(parsed.value, "show today");
    }
}
