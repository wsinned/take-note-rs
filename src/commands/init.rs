use chrono::Local;
use dialoguer::{Confirm, Input, Select};
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;
use toml_edit::{DocumentMut, Item, Table, TableLike};

use crate::helpers::config::{BATCH_SIZE_RANGE, CONFIG_PATH, expand_home};

const NOTES_FOLDER_KEY: &str = "notesFolder";
const EDITOR_KEY: &str = "editor";
const TEMPLATE_KEY: &str = "template";
const BATCH_KEY: &str = "batch";

const EDITOR_OPTIONS: &[&str] = &["generic", "obsidian", "vscode"];

#[derive(Error, Debug)]
pub enum InitError {
    #[error("interrupted")]
    Interrupted,
    #[error("pre-flight fixes applied — rerun `take-note init` to continue")]
    PreFlightFixed,
    #[error("{0}")]
    Other(String),
}

impl From<io::Error> for InitError {
    fn from(e: io::Error) -> Self {
        if e.kind() == io::ErrorKind::Interrupted {
            InitError::Interrupted
        } else {
            InitError::Other(e.to_string())
        }
    }
}

// Map dialoguer errors: Interrupted → InitError::Interrupted, else Other.
fn map_prompt<T>(result: Result<T, dialoguer::Error>) -> Result<T, InitError> {
    result.map_err(|e| {
        let dialoguer::Error::IO(io_err) = e;
        if io_err.kind() == io::ErrorKind::Interrupted {
            InitError::Interrupted
        } else {
            InitError::Other(io_err.to_string())
        }
    })
}

pub fn run() -> Result<(), InitError> {
    let config_path = config_path();
    let config_dir = config_path
        .parent()
        .ok_or_else(|| InitError::Other("cannot determine config directory".into()))?
        .to_path_buf();

    // --- pre-flight ---
    let mut doc = if config_path.exists() {
        let raw =
            std::fs::read_to_string(&config_path).map_err(|e| InitError::Other(e.to_string()))?;

        match raw.parse::<DocumentMut>() {
            Err(parse_err) => {
                eprintln!("warning: config file cannot be parsed: {parse_err}");
                let confirmed = map_prompt(
                    Confirm::new()
                        .with_prompt("Back up the malformed config and start fresh?")
                        .default(false)
                        .interact(),
                )?;
                if confirmed {
                    let backup =
                        backup_malformed_config(&config_path, raw.as_bytes(), &timestamp())?;
                    eprintln!("Backed up malformed config to {}.", backup.display());
                    DocumentMut::new()
                } else {
                    return Err(InitError::Interrupted);
                }
            }
            Ok(parsed) => {
                // Check for invalid field values.
                let fixed = run_preflight_fixes(&parsed, &config_path, &config_dir)?;
                if fixed {
                    return Err(InitError::PreFlightFixed);
                }
                parsed
            }
        }
    } else {
        DocumentMut::new()
    };

    // --- section selection ---
    let section_name = pick_section(&doc)?;

    // --- field prompts ---
    let values = prompt_fields(&doc, &section_name)?;

    // --- write ---
    write_section(&mut doc, &section_name, &values)?;
    std::fs::create_dir_all(&config_dir).map_err(|e| InitError::Other(e.to_string()))?;
    write_atomic(&config_path, doc.to_string().as_bytes())
        .map_err(|e| InitError::Other(e.to_string()))?;

    println!("Config written to {}", config_path.display());
    Ok(())
}

// ---------------------------------------------------------------------------
// Pre-flight
// ---------------------------------------------------------------------------

fn run_preflight_fixes(
    doc: &DocumentMut,
    config_path: &Path,
    config_dir: &Path,
) -> Result<bool, InitError> {
    let mut issues: Vec<String> = Vec::new();

    for (section_name, item) in doc.iter() {
        let table = item.as_table_like().ok_or_else(|| {
            InitError::Other(format!(
                "config entry `{section_name}` must be a table, not {}",
                item.type_name()
            ))
        })?;
        if let Some(editor_item) = table.get(EDITOR_KEY)
            && let Some(s) = editor_item.as_str()
            && !EDITOR_OPTIONS.contains(&s)
        {
            issues.push(format!(
                "[{section_name}].{EDITOR_KEY}: unknown value \"{s}\""
            ));
        }
        if let Some(folder_item) = table.get(NOTES_FOLDER_KEY)
            && let Some(s) = folder_item.as_str()
        {
            let expanded = expand_home(s);
            if !Path::new(&expanded).exists() {
                issues.push(format!(
                    "[{section_name}].{NOTES_FOLDER_KEY}: path \"{s}\" does not exist"
                ));
            }
        }
    }

    if issues.is_empty() {
        return Ok(false);
    }

    eprintln!("Pre-flight found invalid field values:");
    for issue in &issues {
        eprintln!("  • {issue}");
    }

    let confirmed = map_prompt(
        Confirm::new()
            .with_prompt("Fix these interactively now?")
            .default(true)
            .interact(),
    )?;

    if !confirmed {
        return Err(InitError::Interrupted);
    }

    let mut doc = doc.clone();

    for (section_name, item) in doc.iter_mut() {
        let table = item.as_table_like_mut().ok_or_else(|| {
            InitError::Other(format!("config entry `{section_name}` must be a table"))
        })?;

        // Fix invalid editor
        if let Some(editor_item) = table.get(EDITOR_KEY)
            && let Some(s) = editor_item.as_str()
            && !EDITOR_OPTIONS.contains(&s)
        {
            let idx = map_prompt(
                Select::new()
                    .with_prompt(format!("Fix [{section_name}].{EDITOR_KEY}"))
                    .items(EDITOR_OPTIONS)
                    .default(0)
                    .interact(),
            )?;
            table.insert(EDITOR_KEY, toml_edit::value(EDITOR_OPTIONS[idx]));
        }

        // Fix invalid notesFolder
        let needs_folder_fix = if let Some(folder_item) = table.get(NOTES_FOLDER_KEY) {
            folder_item
                .as_str()
                .map(|s| !Path::new(&expand_home(s)).exists())
                .unwrap_or(false)
        } else {
            false
        };

        if needs_folder_fix {
            loop {
                let current = table
                    .get(NOTES_FOLDER_KEY)
                    .and_then(|i| i.as_str())
                    .unwrap_or("")
                    .to_string();

                let choice = map_prompt(
                    Select::new()
                        .with_prompt(format!(
                            "Fix [{section_name}].{NOTES_FOLDER_KEY} (current: \"{current}\")"
                        ))
                        .items(&["Create it", "Enter a different path"])
                        .default(0)
                        .interact(),
                )?;

                if choice == 0 {
                    std::fs::create_dir_all(expand_home(&current))
                        .map_err(|e| InitError::Other(e.to_string()))?;
                    eprintln!("Created {current}");
                    break;
                } else {
                    let new_path: String = map_prompt(
                        Input::new()
                            .with_prompt(format!("[{section_name}].{NOTES_FOLDER_KEY}"))
                            .interact_text(),
                    )?;
                    if Path::new(&expand_home(&new_path)).exists() {
                        table.insert(NOTES_FOLDER_KEY, toml_edit::value(new_path));
                        break;
                    } else {
                        eprintln!("Path does not exist, try again.");
                    }
                }
            }
        }
    }

    std::fs::create_dir_all(config_dir).map_err(|e| InitError::Other(e.to_string()))?;
    write_atomic(config_path, doc.to_string().as_bytes())
        .map_err(|e| InitError::Other(e.to_string()))?;

    eprintln!("Fixes applied. Rerun `take-note init` to continue setup.");
    Ok(true)
}

// ---------------------------------------------------------------------------
// Section selection
// ---------------------------------------------------------------------------

fn pick_section(doc: &DocumentMut) -> Result<String, InitError> {
    let has_default = doc.contains_key("default");

    if !has_default {
        return Ok("default".to_string());
    }

    let mut options: Vec<String> = doc.iter().map(|(k, _)| k.to_string()).collect();
    options.push("Create new section".to_string());

    let idx = map_prompt(
        Select::new()
            .with_prompt("Which section would you like to edit?")
            .items(&options)
            .default(0)
            .interact(),
    )?;

    if idx == options.len() - 1 {
        // "Create new section" chosen
        return prompt_new_section_name(doc);
    }

    Ok(options[idx].clone())
}

fn prompt_new_section_name(doc: &DocumentMut) -> Result<String, InitError> {
    loop {
        let name: String = map_prompt(
            Input::new()
                .with_prompt("New section name (a-z, 0-9, _, -)")
                .interact_text(),
        )?;

        if name == "default" && doc.contains_key("default") {
            eprintln!("Section 'default' already exists. Choose it from the picker instead.");
            continue;
        }
        if name.is_empty()
            || name
                .chars()
                .any(|c| !matches!(c, 'a'..='z' | '0'..='9' | '_' | '-'))
        {
            eprintln!("Invalid name. Use only lowercase a-z, 0-9, _ and -.");
            continue;
        }

        return Ok(name);
    }
}

// ---------------------------------------------------------------------------
// Field prompts
// ---------------------------------------------------------------------------

struct SectionValues {
    notes_folder: String,
    editor: String,
    template: Option<String>,
    batch: usize,
}

fn prompt_fields(doc: &DocumentMut, section_name: &str) -> Result<SectionValues, InitError> {
    let empty = Table::new();
    let existing = doc
        .get(section_name)
        .and_then(Item::as_table_like)
        .unwrap_or(&empty);

    // notesFolder
    let notes_folder = prompt_notes_folder(existing)?;

    // editor
    let editor = prompt_editor(existing)?;

    // template
    let template = prompt_template(existing)?;

    // batch
    let batch = prompt_batch(existing)?;

    Ok(SectionValues {
        notes_folder,
        editor,
        template,
        batch,
    })
}

fn prompt_notes_folder(existing: &dyn TableLike) -> Result<String, InitError> {
    let default = existing
        .get(NOTES_FOLDER_KEY)
        .and_then(|i| i.as_str())
        .unwrap_or("")
        .to_string();

    loop {
        let value: String = map_prompt(
            Input::new()
                .with_prompt("Notes folder path")
                .default(default.clone())
                .interact_text(),
        )?;

        if Path::new(&expand_home(&value)).exists() {
            return Ok(value);
        }

        let choice = map_prompt(
            Select::new()
                .with_prompt(format!("Path \"{value}\" does not exist"))
                .items(&["Create it", "Enter a different path"])
                .default(0)
                .interact(),
        )?;

        if choice == 0 {
            std::fs::create_dir_all(expand_home(&value))
                .map_err(|e| InitError::Other(e.to_string()))?;
            return Ok(value);
        }
        // else loop to re-prompt
    }
}

fn prompt_editor(existing: &dyn TableLike) -> Result<String, InitError> {
    let current = existing
        .get(EDITOR_KEY)
        .and_then(|i| i.as_str())
        .unwrap_or("generic");

    let default_idx = EDITOR_OPTIONS
        .iter()
        .position(|&e| e == current)
        .unwrap_or_else(|| {
            if current != "generic" {
                eprintln!("warning: unknown editor \"{current}\", defaulting to generic");
            }
            0
        });

    let idx = map_prompt(
        Select::new()
            .with_prompt("Editor")
            .items(EDITOR_OPTIONS)
            .default(default_idx)
            .interact(),
    )?;

    Ok(EDITOR_OPTIONS[idx].to_string())
}

fn prompt_template(existing: &dyn TableLike) -> Result<Option<String>, InitError> {
    let current = existing
        .get(TEMPLATE_KEY)
        .and_then(|i| i.as_str())
        .unwrap_or("")
        .to_string();

    let value: String = map_prompt(
        Input::new()
            .with_prompt("Template path (optional, press Enter to skip)")
            .default(current)
            .allow_empty(true)
            .interact_text(),
    )?;

    if value.is_empty() {
        Ok(None)
    } else {
        Ok(Some(value))
    }
}

fn prompt_batch(existing: &dyn TableLike) -> Result<usize, InitError> {
    let current: usize = existing
        .get(BATCH_KEY)
        .and_then(|i| i.as_integer())
        .map(|n| n as usize)
        .unwrap_or(1);

    loop {
        let raw: String = map_prompt(
            Input::new()
                .with_prompt(format!(
                    "Batch size ({}-{})",
                    BATCH_SIZE_RANGE.start(),
                    BATCH_SIZE_RANGE.end()
                ))
                .default(current.to_string())
                .interact_text(),
        )?;

        match raw.trim().parse::<usize>() {
            Ok(n) if BATCH_SIZE_RANGE.contains(&n) => return Ok(n),
            Ok(n) => eprintln!(
                "Batch size must be between {} and {}, got {n}.",
                BATCH_SIZE_RANGE.start(),
                BATCH_SIZE_RANGE.end()
            ),
            Err(_) => eprintln!("Please enter a whole number."),
        }
    }
}

// ---------------------------------------------------------------------------
// TOML write helpers
// ---------------------------------------------------------------------------

fn backup_malformed_config(
    config_path: &Path,
    content: &[u8],
    timestamp: &str,
) -> Result<PathBuf, InitError> {
    let permissions = std::fs::metadata(config_path)
        .map_err(|error| InitError::Other(format!("cannot read config permissions: {error}")))?
        .permissions();
    let base = config_path.with_extension(format!("toml.{timestamp}"));

    for suffix in 0usize.. {
        let backup_path = if suffix == 0 {
            base.clone()
        } else {
            let mut name = base.as_os_str().to_os_string();
            name.push(format!(".{suffix}"));
            PathBuf::from(name)
        };

        let mut backup = match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&backup_path)
        {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(InitError::Other(format!(
                    "cannot create config backup {}: {error}",
                    backup_path.display()
                )));
            }
        };

        let backup_result = (|| -> io::Result<()> {
            backup.write_all(content)?;
            backup.set_permissions(permissions.clone())?;
            backup.sync_all()
        })();
        drop(backup);

        if let Err(error) = backup_result {
            let cleanup_error = std::fs::remove_file(&backup_path).err();
            let cleanup = cleanup_error
                .map(|error| format!("; also could not remove partial backup: {error}"))
                .unwrap_or_default();
            return Err(InitError::Other(format!(
                "cannot complete config backup {}: {error}{cleanup}",
                backup_path.display()
            )));
        }

        if let Err(error) = std::fs::remove_file(config_path) {
            return Err(InitError::Other(format!(
                "backed up malformed config to {}, but could not remove {}: {error}",
                backup_path.display(),
                config_path.display()
            )));
        }

        return Ok(backup_path);
    }

    unreachable!("unbounded backup suffix search must return");
}

fn write_section(
    doc: &mut DocumentMut,
    name: &str,
    values: &SectionValues,
) -> Result<(), InitError> {
    let table = doc[name].or_insert(Item::Table(Table::new()));
    let item_type = table.type_name();
    let t = table.as_table_like_mut().ok_or_else(|| {
        InitError::Other(format!(
            "config entry `{name}` must be a table, not {}",
            item_type
        ))
    })?;

    t.insert(
        NOTES_FOLDER_KEY,
        toml_edit::value(values.notes_folder.as_str()),
    );
    t.insert(EDITOR_KEY, toml_edit::value(values.editor.as_str()));

    match &values.template {
        Some(tmpl) => {
            t.insert(TEMPLATE_KEY, toml_edit::value(tmpl.as_str()));
        }
        None => {
            t.remove(TEMPLATE_KEY);
        }
    }

    t.insert(BATCH_KEY, toml_edit::value(values.batch as i64));
    Ok(())
}

fn write_atomic(path: &Path, content: &[u8]) -> io::Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("take-note");

    let mut tmp = tempfile::Builder::new()
        .prefix(&format!(".{file_name}.tmp."))
        .tempfile_in(parent)?;

    tmp.write_all(content)?;
    tmp.as_file().sync_all()?;
    tmp.persist(path).map(|_| ()).map_err(|e| e.error)
}

fn config_path() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home.join(CONFIG_PATH)
}

fn timestamp() -> String {
    Local::now().format("%Y%m%d-%H%M%S").to_string()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helpers::config::load_config;

    #[test]
    fn malformed_config_backup_removes_original_and_preserves_contents() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config.toml");
        let malformed = b"[default\ninvalid";
        std::fs::write(&config_path, malformed).unwrap();

        let backup = backup_malformed_config(&config_path, malformed, "20260713-120000").unwrap();

        assert_eq!(backup, dir.path().join("config.toml.20260713-120000"));
        assert_eq!(std::fs::read(&backup).unwrap(), malformed);
        assert!(!config_path.exists());
    }

    #[test]
    fn malformed_config_backup_uses_suffix_without_overwriting() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config.toml");
        let existing_backup = dir.path().join("config.toml.20260713-120000");
        std::fs::write(&config_path, "malformed").unwrap();
        std::fs::write(&existing_backup, "older backup").unwrap();

        let backup =
            backup_malformed_config(&config_path, b"malformed", "20260713-120000").unwrap();

        assert_eq!(backup, dir.path().join("config.toml.20260713-120000.1"));
        assert_eq!(
            std::fs::read_to_string(existing_backup).unwrap(),
            "older backup"
        );
        assert_eq!(std::fs::read_to_string(backup).unwrap(), "malformed");
    }

    #[cfg(unix)]
    #[test]
    fn malformed_config_backup_preserves_unix_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config.toml");
        std::fs::write(&config_path, "malformed").unwrap();
        std::fs::set_permissions(&config_path, std::fs::Permissions::from_mode(0o640)).unwrap();

        let backup =
            backup_malformed_config(&config_path, b"malformed", "20260713-120000").unwrap();

        assert_eq!(
            std::fs::metadata(backup).unwrap().permissions().mode() & 0o777,
            0o640
        );
    }

    #[test]
    fn write_section_creates_correct_toml_keys() {
        let mut doc = DocumentMut::new();
        write_section(
            &mut doc,
            "default",
            &SectionValues {
                notes_folder: "~/Notes".into(),
                editor: "obsidian".into(),
                template: Some("Templates/Weekly.md".into()),
                batch: 3,
            },
        )
        .unwrap();
        let toml = doc.to_string();
        assert!(toml.contains("notesFolder"), "must use notesFolder key");
        assert!(toml.contains("obsidian"));
        assert!(toml.contains("Templates/Weekly.md"));
        assert!(toml.contains("batch = 3"));
        assert!(
            !toml.contains("notes_folder"),
            "must not use snake_case key"
        );
    }

    #[test]
    fn write_section_rejects_scalar_section() {
        let mut doc: DocumentMut = "default = 1\n".parse().unwrap();

        let error = write_section(
            &mut doc,
            "default",
            &SectionValues {
                notes_folder: "~/Notes".into(),
                editor: "generic".into(),
                template: None,
                batch: 1,
            },
        )
        .unwrap_err();

        assert_eq!(
            error.to_string(),
            "config entry `default` must be a table, not integer"
        );
    }

    #[test]
    fn write_section_rejects_array_section() {
        let mut doc: DocumentMut = "work = [1, 2]\n".parse().unwrap();

        let error = write_section(
            &mut doc,
            "work",
            &SectionValues {
                notes_folder: "~/Notes".into(),
                editor: "generic".into(),
                template: None,
                batch: 1,
            },
        )
        .unwrap_err();

        assert_eq!(
            error.to_string(),
            "config entry `work` must be a table, not array"
        );
    }

    #[test]
    fn preflight_rejects_non_table_entry_before_section_selection() {
        let dir = tempfile::tempdir().unwrap();
        let doc: DocumentMut = "default = 1\n".parse().unwrap();

        let error =
            run_preflight_fixes(&doc, &dir.path().join("config.toml"), dir.path()).unwrap_err();

        assert_eq!(
            error.to_string(),
            "config entry `default` must be a table, not integer"
        );
    }

    #[test]
    fn write_section_updates_inline_table() {
        let mut doc: DocumentMut = "default = { editor = \"generic\", batch = 1 }\n"
            .parse()
            .unwrap();

        write_section(
            &mut doc,
            "default",
            &SectionValues {
                notes_folder: "~/Notes".into(),
                editor: "vscode".into(),
                template: None,
                batch: 2,
            },
        )
        .unwrap();

        let default = doc["default"].as_inline_table().unwrap();
        assert_eq!(default[NOTES_FOLDER_KEY].as_str(), Some("~/Notes"));
        assert_eq!(default[EDITOR_KEY].as_str(), Some("vscode"));
        assert_eq!(default[BATCH_KEY].as_integer(), Some(2));
    }

    #[test]
    fn round_trip_through_load_config() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config.toml");

        let mut doc = DocumentMut::new();
        write_section(
            &mut doc,
            "default",
            &SectionValues {
                notes_folder: dir.path().to_string_lossy().to_string(),
                editor: "vscode".into(),
                template: None,
                batch: 2,
            },
        )
        .unwrap();
        std::fs::write(&config_path, doc.to_string()).unwrap();

        let cfg = load_config("default", Some(&config_path)).unwrap();
        assert_eq!(
            cfg.notes_folder,
            Some(dir.path().to_string_lossy().to_string())
        );
        assert_eq!(cfg.editor, Some(crate::helpers::config::Editor::Vscode));
        assert_eq!(cfg.batch, Some(2));
        assert_eq!(cfg.template, None);
    }

    #[test]
    fn round_trip_preserves_existing_sections() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config.toml");

        // Write initial file with two sections
        let initial = format!(
            "[default]\nnotesFolder = \"{path}\"\neditor = \"generic\"\nbatch = 1\n\n[weekly]\nnotesFolder = \"{path}\"\neditor = \"obsidian\"\nbatch = 2\n",
            path = dir.path().display()
        );
        std::fs::write(&config_path, &initial).unwrap();

        // Parse, update only [weekly]
        let mut doc: DocumentMut = initial.parse().unwrap();
        write_section(
            &mut doc,
            "weekly",
            &SectionValues {
                notes_folder: dir.path().to_string_lossy().to_string(),
                editor: "vscode".into(),
                template: None,
                batch: 4,
            },
        )
        .unwrap();
        std::fs::write(&config_path, doc.to_string()).unwrap();

        // [default] must be unchanged
        let default_cfg = load_config("default", Some(&config_path)).unwrap();
        assert_eq!(
            default_cfg.editor,
            Some(crate::helpers::config::Editor::Generic)
        );
        assert_eq!(default_cfg.batch, Some(1));

        // [weekly] must reflect new values
        let weekly_cfg = load_config("weekly", Some(&config_path)).unwrap();
        assert_eq!(
            weekly_cfg.editor,
            Some(crate::helpers::config::Editor::Vscode)
        );
        assert_eq!(weekly_cfg.batch, Some(4));
    }
}
