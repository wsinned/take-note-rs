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
pub fn load_config(name: &str, config_path: Option<&Path>) -> Result<NamedConfig, ConfigError> {
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

    let section = file
        .sections
        .get(name)
        .or_else(|| file.sections.get("default"))
        .cloned()
        .unwrap_or_default();

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

        let work_cfg = load_config("work", Some(tmp.path())).unwrap();
        assert_eq!(work_cfg.notes_folder, Some("/tmp/work".to_string()));
        assert_eq!(work_cfg.editor, Some(Editor::Vscode));
        assert_eq!(work_cfg.batch, Some(1)); // falls back to default
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

        let cfg = load_config("work", Some(tmp.path())).unwrap();
        assert_eq!(cfg.notes_folder, Some("/tmp/notes".to_string()));
    }
}
