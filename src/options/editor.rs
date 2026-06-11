use crate::helpers::config::Editor;
use thiserror::Error;

/// Errors from invalid editor options.
#[derive(Error, Debug, PartialEq)]
#[error("invalid Editor option, must be one of |generic|vscode|obsidian|")]
pub struct InvalidEditorError;

/// Parse an editor option from a string.
///
/// # Examples
///
/// ```
/// use take_note::helpers::config::Editor;
/// use take_note::options::editor::editor_from_str;
///
/// assert_eq!(editor_from_str("obsidian").unwrap(), Editor::Obsidian);
/// assert!(editor_from_str("invalid").is_err());
/// ```
pub fn editor_from_str(s: &str) -> Result<Editor, InvalidEditorError> {
    match s {
        "generic" => Ok(Editor::Generic),
        "vscode" => Ok(Editor::Vscode),
        "obsidian" => Ok(Editor::Obsidian),
        _ => Err(InvalidEditorError),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_editor_from_str() {
        assert_eq!(editor_from_str("generic").unwrap(), Editor::Generic);
        assert_eq!(editor_from_str("vscode").unwrap(), Editor::Vscode);
        assert_eq!(editor_from_str("obsidian").unwrap(), Editor::Obsidian);
        assert!(editor_from_str("invalid").is_err());
    }
}
