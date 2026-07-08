use clap::Args;
use std::path::PathBuf;

use crate::handlers::open_with_editor;
use crate::helpers::config::{BATCH_SIZE_RANGE, Editor, Mergeable, NamedConfig, merge_with_flags};
use crate::helpers::date::{
    When, date_for_header, date_from_when, get_batch_dates, name_from_date,
};
use crate::helpers::output::{FileResult, OutputFormat, format_output};
use crate::helpers::template::{get_template_content, update_template_variables};
use crate::options::editor::editor_from_str;
use chrono::Datelike;

/// Arguments for the weekly note command.
#[derive(Args, Clone, Debug)]
pub struct WeeklyArgs {
    /// Which week's note to open
    #[arg(value_name = "WHEN")]
    when: Option<String>,

    /// Blob to append to the resolved note without opening an editor
    #[arg(value_name = "APPEND", allow_hyphen_values = true)]
    append: Option<String>,

    /// Which week's note to open
    #[arg(long = "when", value_name = "WHEN")]
    when_flag: Option<String>,

    /// Named config section to use from ~/.config/take-note/config.toml
    #[arg(long, value_name = "NAME")]
    config: Option<String>,

    /// The root folder containing your notes
    #[arg(long, value_name = "PATH")]
    notes_folder: Option<String>,

    /// Which editor configuration to use
    #[arg(long, value_name = "EDITOR")]
    editor: Option<String>,

    /// The template file to use when creating new weekly notes
    #[arg(long, value_name = "PATH")]
    template: Option<String>,

    /// The number of files to create
    #[arg(long, value_name = "N")]
    batch: Option<usize>,

    /// Create the file without opening it in an editor
    #[arg(long)]
    no_open: bool,

    /// Output format for --no-open mode
    #[arg(long, value_name = "FORMAT")]
    format: Option<String>,
}

/// Run the weekly note command.
pub fn run(args: WeeklyArgs) -> Result<(), Box<dyn std::error::Error>> {
    if args.config.as_deref() == Some("default") {
        return Err("'default' is reserved as the base config; pass a named section instead, e.g. --config work".into());
    }
    let config_name = args.config.as_deref().unwrap_or("default");
    let cfg = crate::helpers::config::load_config_with_fallback(config_name, Some("weekly"), None)?;
    let merged = merge_with_flags(&cfg, args);

    let notes_folder = merged
        .notes_folder
        .ok_or("notesFolder is required. Set it in ~/.config/take-note/config.toml or pass --notes-folder.")?;

    let batch_size = if merged.append.is_some() {
        1
    } else {
        merged.batch.unwrap_or(1)
    };
    if !BATCH_SIZE_RANGE.contains(&batch_size) {
        return Err("batch size must be between 1 and 8".into());
    }

    let when = resolve_when(merged.when.as_deref(), merged.when_flag.as_deref())?;
    let when = When::from_str(when)?;
    let start_date = date_from_when(chrono::Local::now().naive_local().date(), when);
    let dates = get_batch_dates(start_date, batch_size);

    const SUFFIX: &str = "Weekly-log";
    const FILE_EXT: &str = "md";

    let mut results = Vec::new();

    for date in dates {
        let (path_part, file_name) = name_from_date(date, SUFFIX, FILE_EXT);
        let full_path = PathBuf::from(&notes_folder).join(&path_part);

        std::fs::create_dir_all(&full_path)?;
        let file_path = full_path.join(&file_name);
        let file_exists = file_path.exists();

        if !file_exists {
            let content = get_template_content(&notes_folder, merged.template.as_deref())?;
            let content = update_template_variables(&content, &date_for_header(date));
            std::fs::write(&file_path, content)?;
        }

        results.push(FileResult {
            created: !file_exists,
            path: file_path.to_string_lossy().into_owned(),
            date: format!(
                "{:04}-{:02}-{:02}",
                date.year_ce().1,
                date.month(),
                date.day()
            ),
        });
    }

    if let Some(blob) = merged.append.as_deref() {
        let path = &results
            .first()
            .ok_or("internal error: no dates resolved")?
            .path;
        crate::commands::append_blob_atomic(std::path::Path::new(path), blob)?;
        println!("{}", path);
        return Ok(());
    }

    if merged.no_open {
        let format = parse_format(merged.format.as_deref())?;
        if let Some(output) = format_output(&results, format) {
            println!("{}", output);
        }
        return Ok(());
    }

    let editor = parse_editor(merged.editor.as_deref())?;
    let first_file = &results
        .first()
        .ok_or("internal error: no dates resolved")?
        .path;
    println!("Opening {} with {:?}", first_file, editor);
    open_with_editor(&editor, first_file);

    Ok(())
}

impl Mergeable for WeeklyArgs {
    fn merge(self, config: &NamedConfig) -> Self {
        Self {
            notes_folder: self.notes_folder.or_else(|| config.notes_folder.clone()),
            editor: self
                .editor
                .or_else(|| config.editor.as_ref().map(editor_to_string)),
            template: self.template.or_else(|| config.template.clone()),
            batch: self.batch.or(config.batch),
            ..self
        }
    }
}

fn resolve_when<'a>(
    positional: Option<&'a str>,
    flag: Option<&'a str>,
) -> Result<&'a str, Box<dyn std::error::Error>> {
    match (positional, flag) {
        (Some(_), Some(_)) => Err("pass WHEN either positionally or with --when, not both".into()),
        (Some(value), None) | (None, Some(value)) => Ok(value),
        (None, None) => Err("WHEN is required".into()),
    }
}

fn editor_to_string(editor: &Editor) -> String {
    match editor {
        Editor::Generic => "generic".to_string(),
        Editor::Obsidian => "obsidian".to_string(),
        Editor::Vscode => "vscode".to_string(),
    }
}

fn parse_format(s: Option<&str>) -> Result<OutputFormat, Box<dyn std::error::Error>> {
    match s.unwrap_or("text") {
        "text" => Ok(OutputFormat::Text),
        "json" => Ok(OutputFormat::Json),
        "silent" => Ok(OutputFormat::Silent),
        other => Err(format!("invalid format: {other}").into()),
    }
}

fn parse_editor(s: Option<&str>) -> Result<Editor, Box<dyn std::error::Error>> {
    match s {
        Some(val) => Ok(editor_from_str(val)?),
        None => Ok(Editor::Generic),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_format() {
        assert_eq!(parse_format(Some("text")).unwrap(), OutputFormat::Text);
        assert_eq!(parse_format(Some("json")).unwrap(), OutputFormat::Json);
        assert_eq!(parse_format(Some("silent")).unwrap(), OutputFormat::Silent);
        assert_eq!(parse_format(None).unwrap(), OutputFormat::Text);
        assert!(parse_format(Some("invalid")).is_err());
    }

    #[test]
    fn test_parse_editor() {
        assert_eq!(parse_editor(Some("obsidian")).unwrap(), Editor::Obsidian);
        assert_eq!(parse_editor(Some("vscode")).unwrap(), Editor::Vscode);
        assert_eq!(parse_editor(None).unwrap(), Editor::Generic);
    }

    #[test]
    fn rejects_config_named_default() {
        let err = run(WeeklyArgs {
            when: Some("thisWeek".to_string()),
            append: None,
            when_flag: None,
            config: Some("default".to_string()),
            notes_folder: None,
            editor: None,
            template: None,
            batch: None,
            no_open: false,
            format: None,
        })
        .unwrap_err();
        assert!(
            err.to_string().contains("reserved"),
            "expected 'reserved' in error message, got: {err}"
        );
    }
}
