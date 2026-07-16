use crate::helpers::config::Editor;
use std::io::Write;
use std::process::Command;

/// Opens a file with the appropriate editor handler.
///
/// Spawns the editor process and returns immediately (does not wait).
/// Prints a warning to stderr if the editor binary cannot be launched.
pub fn open_with_editor(editor: &Editor, file_path: &str) {
    match editor {
        Editor::Generic => open_generic(file_path),
        Editor::Vscode => open_vscode(file_path),
        Editor::Obsidian => open_obsidian(file_path),
    }
}

fn open_generic(file_path: &str) {
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
    spawn_with_stderr(&editor, file_path, &mut std::io::stderr());
}

fn open_vscode(file_path: &str) {
    spawn_with_stderr("code", file_path, &mut std::io::stderr());
}

fn open_obsidian(file_path: &str) {
    let url = obsidian_url(file_path);
    let open_cmd = get_open_command();
    spawn_with_stderr(open_cmd, &url, &mut std::io::stderr());
}

fn spawn_with_stderr(program: &str, arg: &str, stderr: &mut dyn Write) {
    if let Err(e) = Command::new(program).arg(arg).spawn() {
        let _ = writeln!(stderr, "warning: could not open '{program}': {e}");
    }
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
    fn spawn_reports_error_when_binary_not_found() {
        let mut buf = Vec::<u8>::new();
        spawn_with_stderr("__no_such_binary_xyz__", "/any/path", &mut buf);
        assert!(
            !buf.is_empty(),
            "expected a warning on stderr when binary is not found, but got nothing"
        );
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
