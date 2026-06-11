use serde::Serialize;

/// Output format for headless mode.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
    Silent,
}

/// Result of creating or finding a note file.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct FileResult {
    pub created: bool,
    pub path: String,
    pub date: String,
}

/// Formats file results for output.
///
/// # Examples
///
/// ```
/// use take_note::helpers::output::{format_output, FileResult, OutputFormat};
///
/// let result = FileResult {
///     created: true,
///     path: "/tmp/notes/2025/07/2025-07-28-Weekly-log.md".to_string(),
///     date: "2025-07-28".to_string(),
/// };
///
/// let text = format_output(&[result], OutputFormat::Text);
/// assert!(text.contains("Created"));
/// assert!(text.contains("/tmp/notes/2025/07/2025-07-28-Weekly-log.md"));
/// ```
pub fn format_output(results: &[FileResult], format: OutputFormat) -> Option<String> {
    match format {
        OutputFormat::Json => Some(serde_json::to_string_pretty(results).unwrap_or_default()),
        OutputFormat::Text => Some(
            results
                .iter()
                .map(|r| {
                    let verb = if r.created { "Created" } else { "Found" };
                    format!("{verb}: {}", r.path)
                })
                .collect::<Vec<_>>()
                .join("\n"),
        ),
        OutputFormat::Silent => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_output_text() {
        let results = vec![
            FileResult {
                created: true,
                path: "/tmp/notes/2025/07/2025-07-28-Weekly-log.md".to_string(),
                date: "2025-07-28".to_string(),
            },
            FileResult {
                created: false,
                path: "/tmp/notes/2025/08/2025-08-04-Weekly-log.md".to_string(),
                date: "2025-08-04".to_string(),
            },
        ];

        let output = format_output(&results, OutputFormat::Text).unwrap();
        assert!(output.contains("Created: /tmp/notes/2025/07/2025-07-28-Weekly-log.md"));
        assert!(output.contains("Found: /tmp/notes/2025/08/2025-08-04-Weekly-log.md"));
    }

    #[test]
    fn test_format_output_json() {
        let result = FileResult {
            created: true,
            path: "/tmp/test.md".to_string(),
            date: "2025-07-28".to_string(),
        };

        let output = format_output(&[result], OutputFormat::Json).unwrap();
        assert!(output.contains("\"created\": true"));
        assert!(output.contains("\"path\": \"/tmp/test.md\""));
    }

    #[test]
    fn test_format_output_silent() {
        let result = FileResult {
            created: true,
            path: "/tmp/test.md".to_string(),
            date: "2025-07-28".to_string(),
        };

        assert_eq!(format_output(&[result], OutputFormat::Silent), None);
    }
}
