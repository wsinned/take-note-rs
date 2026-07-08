use chrono::Local;
use dialoguer::{Confirm, Input, Select};
use std::io;
use std::path::{Path, PathBuf};
use thiserror::Error;
use toml_edit::{DocumentMut, Item, Table};

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
                        .with_prompt(format!(
                            "Back up as config.toml.{} and start fresh?",
                            timestamp()
                        ))
                        .default(false)
                        .interact(),
                )?;
                if confirmed {
                    let backup = config_path.with_extension(format!("toml.{}", timestamp()));
                    std::fs::copy(&config_path, &backup)
                        .map_err(|e| InitError::Other(e.to_string()))?;
                    eprintln!(
                        "Backed up to {}. Rerun `take-note init` to continue setup.",
                        backup.display()
                    );
                    return Err(InitError::PreFlightFixed);
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
    write_section(&mut doc, &section_name, &values);
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
        let Some(table) = item.as_table() else {
            continue;
        };
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
        let Some(table) = item.as_table_mut() else {
            continue;
        };

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
            table[EDITOR_KEY] = toml_edit::value(EDITOR_OPTIONS[idx]);
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
                        table[NOTES_FOLDER_KEY] = toml_edit::value(new_path);
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
    let existing = doc
        .get(section_name)
        .and_then(|i| i.as_table())
        .cloned()
        .unwrap_or_default();

    // notesFolder
    let notes_folder = prompt_notes_folder(&existing)?;

    // editor
    let editor = prompt_editor(&existing)?;

    // template
    let template = prompt_template(&existing)?;

    // batch
    let batch = prompt_batch(&existing)?;

    Ok(SectionValues {
        notes_folder,
        editor,
        template,
        batch,
    })
}

fn prompt_notes_folder(existing: &Table) -> Result<String, InitError> {
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

fn prompt_editor(existing: &Table) -> Result<String, InitError> {
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

fn prompt_template(existing: &Table) -> Result<Option<String>, InitError> {
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

fn prompt_batch(existing: &Table) -> Result<usize, InitError> {
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

fn write_section(doc: &mut DocumentMut, name: &str, values: &SectionValues) {
    let table = doc[name].or_insert(Item::Table(Table::new()));
    let t = table.as_table_mut().expect("section must be a table");

    t[NOTES_FOLDER_KEY] = toml_edit::value(values.notes_folder.as_str());
    t[EDITOR_KEY] = toml_edit::value(values.editor.as_str());

    match &values.template {
        Some(tmpl) => {
            t[TEMPLATE_KEY] = toml_edit::value(tmpl.as_str());
        }
        None => {
            t.remove(TEMPLATE_KEY);
        }
    }

    t[BATCH_KEY] = toml_edit::value(values.batch as i64);
}

fn write_atomic(path: &Path, content: &[u8]) -> io::Result<()> {
    use std::io::Write;
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
        );
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
        );
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
        );
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
