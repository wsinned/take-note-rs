use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Path to the default config file.
pub const CONFIG_PATH: &str = ".config/take-note/config.toml";

/// Supported editor types.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Editor {
    #[default]
    Generic,
    Obsidian,
    Vscode,
}

/// Configuration for a single named profile.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub struct NamedConfig {
    #[serde(alias = "notesFolder")]
    pub notes_folder: Option<String>,
    pub editor: Option<Editor>,
    pub template: Option<String>,
    pub batch: Option<usize>,
}

/// Full config file contents.
#[derive(Clone, Debug, Deserialize)]
pub struct ConfigFile {
    #[serde(flatten)]
    pub sections: HashMap<String, NamedConfig>,
}

/// Errors that can occur when loading configuration.
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("IO error reading config: {0}")]
    Io(#[from] std::io::Error),
    #[error("TOML parse error: {0}")]
    Parse(#[from] toml::de::Error),
}

/// Loads a named config section, merging with built-in defaults.
///
/// Falls back to `[default]` if the named section doesn't exist.
/// Home directory (`~`) is expanded in path fields.
///
/// # Examples
///
/// ```no_run
/// use take_note::helpers::config::load_config;
///
/// let config = load_config("default", None).unwrap();
/// ```
#[allow(dead_code)]
pub fn load_config(name: &str, config_path: Option<&Path>) -> Result<NamedConfig, ConfigError> {
    load_config_with_fallback(name, None, config_path)
}

/// Loads a config section with an optional command-specific fallback.
///
/// If `command_name` is provided, the lookup order is:
/// 1. Named section (from `--config` flag)
/// 2. Command-specific section (e.g. `[weekly]` or `[daily]`)
/// 3. `[default]` section
///
/// This allows separate defaults for each command without requiring
/// explicit `--config` flags.
///
/// # Examples
///
/// ```no_run
/// use take_note::helpers::config::load_config_with_fallback;
///
/// let config = load_config_with_fallback("default", Some("weekly"), None).unwrap();
/// ```
pub fn load_config_with_fallback(
    name: &str,
    command_name: Option<&str>,
    config_path: Option<&Path>,
) -> Result<NamedConfig, ConfigError> {
    let path = config_path
        .map(PathBuf::from)
        .unwrap_or_else(default_config_path);

    let file = if path.exists() {
        let raw = std::fs::read_to_string(&path)?;
        toml::from_str(&raw)?
    } else {
        ConfigFile {
            sections: HashMap::new(),
        }
    };

    // Determine the effective section to use
    let section = if name != "default" {
        // Explicit --config flag was used, look up that section directly
        let named_section = file.sections
            .get(name)
            .cloned()
            .unwrap_or_default();
        
        // Also merge with default for missing fields
        let default_section = file
            .sections
            .get("default")
            .cloned()
            .unwrap_or_default();
        
        let mut merged = named_section.clone();
        if merged.notes_folder.is_none() {
            merged.notes_folder = default_section.notes_folder.clone();
        }
        if merged.editor.is_none() {
            merged.editor = default_section.editor.clone();
        }
        if merged.template.is_none() {
            merged.template = default_section.template.clone();
        }
        if merged.batch.is_none() {
            merged.batch = default_section.batch;
        }
        merged
    } else {
        // No --config flag, use command-specific section if available
        let command_section = command_name
            .and_then(|cmd| file.sections.get(cmd))
            .cloned();
        
        let default_section = file
            .sections
            .get("default")
            .cloned()
            .unwrap_or_default();
        
        match command_section {
            Some(cmd) => {
                // Merge command-specific with default: command wins for set fields,
                // default fills in missing fields
                let mut merged = cmd.clone();
                if merged.notes_folder.is_none() {
                    merged.notes_folder = default_section.notes_folder.clone();
                }
                if merged.editor.is_none() {
                    merged.editor = default_section.editor.clone();
                }
                if merged.template.is_none() {
                    merged.template = default_section.template.clone();
                }
                if merged.batch.is_none() {
                    merged.batch = default_section.batch;
                }
                merged
            }
            None => default_section,
        }
    };

    let mut merged = section.clone();
    if merged.editor.is_none() {
        merged.editor = Some(Editor::Generic);
    }
    if merged.batch.is_none() {
        merged.batch = Some(1);
    }

    if let Some(ref folder) = merged.notes_folder {
        merged.notes_folder = Some(expand_home(folder));
    }
    if let Some(ref template) = merged.template {
        merged.template = Some(expand_home(template));
    }

    Ok(merged)
}

/// Returns the default config file path (`~/.config/take-note/config.toml`).
fn default_config_path() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home.join(CONFIG_PATH)
}

/// Expands a leading `~` to the user's home directory.
fn expand_home(value: &str) -> String {
    if let Some(rest) = value.strip_prefix("~/") {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        home.join(rest).to_string_lossy().into_owned()
    } else {
        value.to_string()
    }
}

/// Merges config values with CLI flags, with CLI flags taking precedence.
///
/// Only applies a config value when the corresponding flag is `None`.
pub fn merge_with_flags<T>(config: &NamedConfig, flags: T) -> T
where
    T: Mergeable,
{
    flags.merge(config)
}

/// Trait for merging config into CLI flag structs.
pub trait Mergeable: Sized {
    fn merge(self, config: &NamedConfig) -> Self;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_expand_home() {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/home/user"));
        let expanded = expand_home("~/Documents/Notes");
        assert_eq!(expanded, home.join("Documents/Notes").to_string_lossy());
    }

    #[test]
    fn test_expand_home_no_tilde() {
        let path = "/absolute/path";
        assert_eq!(expand_home(path), path);
    }

    #[test]
    fn test_load_config_from_file() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        write!(
            tmp,
            r#"
[default]
notes_folder = "/tmp/notes"
editor = "obsidian"
batch = 3

[work]
notes_folder = "/tmp/work"
editor = "vscode"
"#
        )
        .unwrap();

        let default_cfg = load_config("default", Some(tmp.path())).unwrap();
        assert_eq!(default_cfg.notes_folder, Some("/tmp/notes".to_string()));
        assert_eq!(default_cfg.editor, Some(Editor::Obsidian));
        assert_eq!(default_cfg.batch, Some(3));
        assert_eq!(default_cfg.template, None);

        let work_cfg = load_config_with_fallback("work", None, Some(tmp.path())).unwrap();
        assert_eq!(work_cfg.notes_folder, Some("/tmp/work".to_string()));
        assert_eq!(work_cfg.editor, Some(Editor::Vscode));
        assert_eq!(work_cfg.batch, Some(3)); // falls back to default
        assert_eq!(work_cfg.template, None);
    }

    #[test]
    fn test_load_config_missing_file() {
        let cfg = load_config("default", Some(Path::new("/nonexistent/config.toml"))).unwrap();
        assert_eq!(cfg.editor, Some(Editor::Generic));
        assert_eq!(cfg.batch, Some(1));
        assert_eq!(cfg.notes_folder, None);
    }

    #[test]
    fn test_load_config_fallback_to_default() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        write!(
            tmp,
            r#"
[default]
notes_folder = "/tmp/notes"
"#
        )
        .unwrap();

        let cfg = load_config_with_fallback("work", None, Some(tmp.path())).unwrap();
        assert_eq!(cfg.notes_folder, Some("/tmp/notes".to_string()));
    }

    #[test]
    fn test_load_config_with_fallback_command_section() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        write!(
            tmp,
            r#"
[default]
notes_folder = "/tmp/notes"
editor = "obsidian"
template = "Templates/Weekly.md"
batch = 3

[weekly]
template = "Templates/Custom-Weekly.md"

[daily]
template = "Templates/Daily.md"
"#
        )
        .unwrap();

        // Weekly should use [weekly] section merged with [default]
        let weekly_cfg = load_config_with_fallback("default", Some("weekly"), Some(tmp.path())).unwrap();
        assert_eq!(weekly_cfg.notes_folder, Some("/tmp/notes".to_string()));
        assert_eq!(weekly_cfg.editor, Some(Editor::Obsidian));
        assert_eq!(weekly_cfg.template, Some("Templates/Custom-Weekly.md".to_string()));
        assert_eq!(weekly_cfg.batch, Some(3));

        // Daily should use [daily] section merged with [default]
        let daily_cfg = load_config_with_fallback("default", Some("daily"), Some(tmp.path())).unwrap();
        assert_eq!(daily_cfg.notes_folder, Some("/tmp/notes".to_string()));
        assert_eq!(daily_cfg.editor, Some(Editor::Obsidian));
        assert_eq!(daily_cfg.template, Some("Templates/Daily.md".to_string()));
        assert_eq!(daily_cfg.batch, Some(3));
    }

    #[test]
    fn test_load_config_with_fallback_no_command_section() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        write!(
            tmp,
            r#"
[default]
notes_folder = "/tmp/notes"
editor = "obsidian"
template = "Templates/Weekly.md"
"#
        )
        .unwrap();

        // No [weekly] section, should fall back to [default]
        let weekly_cfg = load_config_with_fallback("default", Some("weekly"), Some(tmp.path())).unwrap();
        assert_eq!(weekly_cfg.template, Some("Templates/Weekly.md".to_string()));
    }

    #[test]
    fn test_explicit_config_flag_ignores_command_fallback() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        write!(
            tmp,
            r#"
[default]
notes_folder = "/tmp/notes"
editor = "obsidian"
template = "Templates/Weekly.md"

[weekly]
template = "Templates/Custom-Weekly.md"

[custom]
notes_folder = "/tmp/custom"
editor = "vscode"
"#
        )
        .unwrap();

        // With --config custom, should use [custom] merged with [default]
        let cfg = load_config_with_fallback("custom", Some("weekly"), Some(tmp.path())).unwrap();
        assert_eq!(cfg.notes_folder, Some("/tmp/custom".to_string()));
        assert_eq!(cfg.editor, Some(Editor::Vscode));
        assert_eq!(cfg.template, Some("Templates/Weekly.md".to_string())); // inherited from default
    }
}
