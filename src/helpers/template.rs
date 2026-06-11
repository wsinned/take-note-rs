use std::path::PathBuf;

/// Reads template content from a file relative to the notes folder.
///
/// Returns an empty string if no template is specified.
/// Logs the template path to stderr on success.
///
/// # Errors
///
/// Returns an error if the template file is specified but does not exist.
///
/// # Examples
///
/// ```no_run
/// use take_note::helpers::template::get_template_content;
///
/// let content = get_template_content("/tmp/notes", Some("Templates/weekly.md")).unwrap();
/// ```
pub fn get_template_content(
    notes_folder: &str,
    template: Option<&str>,
) -> Result<String, TemplateError> {
    let template = match template {
        Some(t) => t,
        None => return Ok(String::new()),
    };

    let path = PathBuf::from(notes_folder).join(template);

    if !path.exists() || !path.is_file() {
        return Err(TemplateError::NotFound(path));
    }

    eprintln!("Using template {}", path.display());

    std::fs::read_to_string(&path).map_err(|e| TemplateError::Io(e.to_string()))
}

/// Replaces `{{date}}` in template content with the given date string.
///
/// # Examples
///
/// ```
/// use take_note::helpers::template::update_template_variables;
///
/// let content = "# W/C {{date}}\n\n";
/// let updated = update_template_variables(content, "Monday 28 July 2025");
/// assert_eq!(updated, "# W/C Monday 28 July 2025\n\n");
/// ```
pub fn update_template_variables(content: &str, date: &str) -> String {
    content.replace("{{date}}", date)
}

/// Errors that can occur when loading templates.
#[derive(Debug, PartialEq)]
pub enum TemplateError {
    NotFound(PathBuf),
    Io(String),
}

impl std::fmt::Display for TemplateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TemplateError::NotFound(path) => write!(f, "Template not found: {}", path.display()),
            TemplateError::Io(msg) => write!(f, "IO error reading template: {}", msg),
        }
    }
}

impl std::error::Error for TemplateError {}

impl From<std::io::Error> for TemplateError {
    fn from(e: std::io::Error) -> Self {
        TemplateError::Io(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_update_template_variables() {
        let content = "# W/C {{date}}\n\nSome content\n";
        let updated = update_template_variables(content, "Monday 28 July 2025");
        assert_eq!(updated, "# W/C Monday 28 July 2025\n\nSome content\n");
    }

    #[test]
    fn test_update_template_variables_no_placeholder() {
        let content = "# Static Header\n";
        let updated = update_template_variables(content, "Monday 28 July 2025");
        assert_eq!(updated, "# Static Header\n");
    }

    #[test]
    fn test_get_template_content_none() {
        let result = get_template_content("/tmp/notes", None).unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn test_get_template_content_missing() {
        let result = get_template_content("/tmp/notes", Some("missing.md"));
        assert!(result.is_err());
        match result {
            Err(TemplateError::NotFound(_)) => {}
            _ => panic!("Expected NotFound error"),
        }
    }

    #[test]
    fn test_get_template_content_valid() {
        let dir = tempfile::tempdir().unwrap();
        let template_path = dir.path().join("template.md");
        let mut file = std::fs::File::create(&template_path).unwrap();
        write!(file, "# W/C {{{{date}}}}\n").unwrap();

        let result =
            get_template_content(dir.path().to_str().unwrap(), Some("template.md")).unwrap();
        assert_eq!(result, "# W/C {{date}}\n");
    }
}
