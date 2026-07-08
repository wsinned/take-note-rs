use crate::helpers::config::Editor;
use std::process::Command;

/// Opens a file with the appropriate editor handler.
///
/// Spawns the editor process and returns immediately (does not wait).
///
/// # Examples
///
/// ```no_run
/// use take_note::handlers::open_with_editor;
/// use take_note::helpers::config::Editor;
///
/// open_with_editor(Editor::Generic, "/tmp/notes/2025-07-28-Weekly-log.md");
/// ```
pub fn open_with_editor(editor: &Editor, file_path: &str) {
    match editor {
        Editor::Generic => open_generic(file_path),
        Editor::Vscode => open_vscode(file_path),
        Editor::Obsidian => open_obsidian(file_path),
    }
}

fn open_generic(file_path: &str) {
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
    let _ = Command::new(&editor).arg(file_path).spawn();
}

fn open_vscode(file_path: &str) {
    let _ = Command::new("code").arg(file_path).spawn();
}

fn open_obsidian(file_path: &str) {
    let url = obsidian_url(file_path);
    let open_cmd = get_open_command();
    let _ = Command::new(open_cmd).arg(&url).spawn();
}

fn obsidian_url(file_path: &str) -> String {
    format!("obsidian://open?path={}", urlencoding::encode(file_path))
}

#[cfg(target_os = "windows")]
fn get_open_command() -> &'static str {
    "start"
}

#[cfg(target_os = "macos")]
fn get_open_command() -> &'static str {
    "open"
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn get_open_command() -> &'static str {
    "xdg-open"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_open_command() {
        let cmd = get_open_command();
        // Just verify it returns a non-empty string
        assert!(!cmd.is_empty());
    }

    #[test]
    fn obsidian_url_encodes_spaces_in_path() {
        let url = obsidian_url("/Users/dennis/My Notes/2025/07/2025-07-28-Weekly-log.md");
        assert!(
            !url.contains(' '),
            "URL contains unencoded space — Obsidian URI will be malformed: {url}"
        );
    }
}
