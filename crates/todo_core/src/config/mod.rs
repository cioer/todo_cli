use crate::error::AppError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const CONFIG_FILE_NAME: &str = "config.json";
const CONFIG_ENV_VAR: &str = "TODOAPP_CONFIG_PATH";

#[derive(Debug, Clone)]
pub struct Palette {
    pub accent: &'static str,
    pub muted: &'static str,
    pub reset: &'static str,
}

impl Palette {
    pub fn accentize(&self, text: &str) -> String {
        if self.accent.is_empty() {
            text.to_string()
        } else {
            format!("{}{}{}", self.accent, text, self.reset)
        }
    }

    pub fn mutedize(&self, text: &str) -> String {
        if self.muted.is_empty() {
            text.to_string()
        } else {
            format!("{}{}{}", self.muted, text, self.reset)
        }
    }
}

pub fn palette_for_theme(theme: Option<&str>) -> Palette {
    match canonical_theme_name_option(theme) {
        Some(ref name) if name == "noir" => Palette {
            accent: "\x1b[38;5;208m",
            muted: "\x1b[38;5;250m",
            reset: "\x1b[0m",
        },
        Some(ref name) if name == "solarized" => Palette {
            accent: "\x1b[38;5;108m",
            muted: "\x1b[38;5;250m",
            reset: "\x1b[0m",
        },
        _ => Palette {
            accent: "",
            muted: "",
            reset: "",
        },
    }
}

fn canonical_theme_name_option(theme: Option<&str>) -> Option<String> {
    theme.and_then(|value| canonical_theme_name(value))
}

pub fn canonical_theme_name(raw: &str) -> Option<String> {
    let mut cleaned = String::new();
    let mut previous_underscore = false;

    for ch in raw.chars() {
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
        return Some("default".into());
    }

    match trimmed {
        "vanilla" | "light" => Some("default".to_string()),
        "dark" | "dark_mode" | "darkmode" => Some("noir".to_string()),
        other => Some(other.to_string()),
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub theme: Option<String>,
    #[serde(default)]
    pub aliases: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct ConfigLoad {
    pub config: Config,
    pub error: Option<AppError>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ConfigOverrides {
    pub theme: Option<String>,
    pub aliases: HashMap<String, String>,
}

pub fn config_path() -> Result<PathBuf, AppError> {
    if let Ok(path) = std::env::var(CONFIG_ENV_VAR)
        && !path.trim().is_empty()
    {
        return Ok(PathBuf::from(path));
    }

    if cfg!(windows) {
        let appdata =
            std::env::var("APPDATA").map_err(|_| AppError::invalid_data("APPDATA is not set"))?;
        Ok(PathBuf::from(appdata)
            .join("todoapp")
            .join(CONFIG_FILE_NAME))
    } else {
        let home = std::env::var("HOME").map_err(|_| AppError::invalid_data("HOME is not set"))?;
        Ok(PathBuf::from(home)
            .join(".config")
            .join("todoapp")
            .join(CONFIG_FILE_NAME))
    }
}

pub fn load_config() -> Result<Config, AppError> {
    let path = config_path()?;
    load_config_from_path(&path)
}

pub fn load_config_with_fallback() -> ConfigLoad {
    match config_path() {
        Ok(path) => load_config_with_fallback_from_path(&path),
        Err(err) => ConfigLoad {
            config: Config::default(),
            error: Some(err),
        },
    }
}

fn load_config_with_fallback_from_path(path: &Path) -> ConfigLoad {
    if !path.exists() {
        return ConfigLoad {
            config: Config::default(),
            error: None,
        };
    }

    match load_config_from_path(path) {
        Ok(config) => ConfigLoad {
            config,
            error: None,
        },
        Err(err) => ConfigLoad {
            config: Config::default(),
            error: Some(err),
        },
    }
}

fn load_config_from_path(path: &Path) -> Result<Config, AppError> {
    let content = std::fs::read_to_string(path)
        .map_err(|err| AppError::io(format!("{}: {}", path.display(), err)))?;
    let config = serde_json::from_str(&content).map_err(|err| {
        AppError::invalid_data(format!("invalid JSON in {}: {}", path.display(), err))
    })?;
    Ok(normalize_config_theme(config))
}

fn normalize_config_theme(mut config: Config) -> Config {
    config.theme = normalize_theme_value(config.theme);
    config
}

fn normalize_theme_value(value: Option<String>) -> Option<String> {
    value.and_then(|name| canonical_theme_name(&name))
}

pub fn merge_overrides(base: &Config, overrides: &ConfigOverrides) -> Config {
    let mut merged = base.clone();
    if let Some(theme) = overrides.theme.as_ref() {
        if let Some(normalized) = canonical_theme_name(theme) {
            merged.theme = Some(normalized);
        }
    }

    for (alias, value) in overrides.aliases.iter() {
        merged.aliases.insert(alias.clone(), value.clone());
    }

    merged
}

#[cfg(test)]
mod tests {
    use super::{
        Config, ConfigOverrides, canonical_theme_name, load_config_from_path,
        load_config_with_fallback_from_path, merge_overrides, palette_for_theme,
    };
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_path(file_name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("todoapp-{nanos}-{file_name}"))
    }

    #[test]
    fn load_config_missing_returns_defaults_and_error() {
        let path = temp_path("missing-config.json");
        let result = load_config_with_fallback_from_path(&path);

        assert_eq!(result.config, Config::default());
        assert!(result.error.is_none());
    }

    #[test]
    fn load_config_invalid_returns_defaults_and_error() {
        let path = temp_path("invalid-config.json");
        fs::write(&path, "{ invalid json ").unwrap();

        let result = load_config_with_fallback_from_path(&path);
        fs::remove_file(&path).ok();

        assert_eq!(result.config, Config::default());
        assert!(result.error.is_some());
    }

    #[test]
    fn load_config_reads_valid_file() {
        let path = temp_path("valid-config.json");
        let content = serde_json::json!({
            "theme": "noir",
            "aliases": {
                "ls": "list today"
            }
        });
        fs::write(&path, serde_json::to_string(&content).unwrap()).unwrap();

        let loaded = load_config_from_path(&path).unwrap();
        fs::remove_file(&path).ok();

        assert_eq!(loaded.theme.as_deref(), Some("noir"));
        assert_eq!(
            loaded.aliases.get("ls").map(String::as_str),
            Some("list today")
        );
    }

    #[test]
    fn merge_overrides_updates_theme_and_aliases() {
        let base = Config {
            theme: Some("light".into()),
            aliases: [("ls".into(), "list today".into())].into_iter().collect(),
        };

        let overrides = ConfigOverrides {
            theme: Some("noir".into()),
            aliases: [
                ("ls".into(), "list backlog".into()),
                ("show".into(), "show today".into()),
            ]
            .into_iter()
            .collect(),
        };

        let merged = merge_overrides(&base, &overrides);
        assert_eq!(merged.theme.as_deref(), Some("noir"));
        assert_eq!(
            merged.aliases.get("ls").map(String::as_str),
            Some("list backlog")
        );
        assert_eq!(
            merged.aliases.get("show").map(String::as_str),
            Some("show today")
        );
    }

    #[test]
    fn merge_overrides_preserves_base_config() {
        let base = Config {
            theme: Some("light".into()),
            aliases: [("ls".into(), "list today".into())].into_iter().collect(),
        };

        let overrides = ConfigOverrides {
            theme: Some("noir".into()),
            aliases: [("focus".into(), "focus today".into())]
                .into_iter()
                .collect(),
        };

        let merged = merge_overrides(&base, &overrides);

        assert_eq!(base.theme.as_deref(), Some("light"));
        assert!(base.aliases.get("focus").is_none());

        assert_eq!(merged.theme.as_deref(), Some("noir"));
        assert_eq!(
            merged.aliases.get("focus").map(String::as_str),
            Some("focus today")
        );
        assert_eq!(
            merged.aliases.get("ls").map(String::as_str),
            Some("list today")
        );
    }

    #[test]
    fn merge_overrides_with_empty_overrides_returns_clone() {
        let base = Config {
            theme: Some("light".into()),
            aliases: [("ls".into(), "list today".into())].into_iter().collect(),
        };

        let merged = merge_overrides(&base, &ConfigOverrides::default());

        assert_eq!(merged, base);
    }

    #[test]
    fn canonical_theme_name_maps_variants() {
        assert_eq!(canonical_theme_name("Vanilla"), Some("default".into()));
        assert_eq!(canonical_theme_name("Noir"), Some("noir".into()));
        assert_eq!(canonical_theme_name("Solarized"), Some("solarized".into()));
        assert_eq!(canonical_theme_name("dark-mode"), Some("noir".into()));
        assert_eq!(canonical_theme_name("  "), Some("default".into()));
    }

    #[test]
    fn palette_for_theme_returns_palette() {
        let default_palette = palette_for_theme(Some("vanilla"));
        assert!(default_palette.accent.is_empty());
        assert!(default_palette.muted.is_empty());

        let noir_palette = palette_for_theme(Some("noir"));
        assert_eq!(noir_palette.accent, "\x1b[38;5;208m");
        assert_eq!(noir_palette.muted, "\x1b[38;5;250m");

        let unknown_palette = palette_for_theme(Some("oceanic"));
        assert!(unknown_palette.accent.is_empty());
    }
}
