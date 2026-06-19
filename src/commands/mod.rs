pub mod daily;
pub mod weekly;

use std::io::Write;
use std::path::Path;

pub(crate) fn append_blob_atomic(path: &Path, blob: &str) -> std::io::Result<()> {
    let mut content = std::fs::read(path)?;

    if !content.is_empty() && !content.ends_with(b"\n") {
        content.push(b'\n');
    }

    content.extend_from_slice(blob.as_bytes());
    write_atomic(path, &content)
}

pub(crate) fn insert_blob_at_heading_path_atomic(
    path: &Path,
    heading_path: &str,
    blob: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    let headings: Vec<&str> = heading_path
        .split('/')
        .map(str::trim)
        .filter(|heading| !heading.is_empty())
        .collect();

    if headings.is_empty() {
        return Err("heading path is required".into());
    }

    let position = crate::helpers::markdown::section_end_for_heading_path(&content, &headings)
        .ok_or_else(|| format!("heading path not found: {heading_path}"))?;

    let mut insertion = String::new();
    if position > 0 && !content[..position].ends_with('\n') {
        insertion.push('\n');
    }
    insertion.push_str(blob);
    if position < content.len() && !insertion.ends_with('\n') {
        insertion.push('\n');
    }

    let mut updated = String::with_capacity(content.len() + insertion.len());
    updated.push_str(&content[..position]);
    updated.push_str(&insertion);
    updated.push_str(&content[position..]);

    write_atomic(path, updated.as_bytes())?;
    Ok(())
}

fn write_atomic(path: &Path, content: &[u8]) -> std::io::Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("take-note");

    let mut temp_file = tempfile::Builder::new()
        .prefix(&format!(".{file_name}.tmp."))
        .tempfile_in(parent)?;

    temp_file.write_all(content)?;
    temp_file.as_file().sync_all()?;

    temp_file.persist(path).map(|_| ()).map_err(|err| err.error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn append_blob_inserts_newline_when_existing_content_has_no_trailing_newline() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("note.md");
        std::fs::write(&path, "existing").unwrap();

        append_blob_atomic(&path, "blob").unwrap();

        assert_eq!(std::fs::read_to_string(&path).unwrap(), "existing\nblob");
    }

    #[test]
    fn append_blob_preserves_blob_when_existing_content_has_trailing_newline() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("note.md");
        std::fs::write(&path, "existing\n").unwrap();

        append_blob_atomic(&path, "blob\n").unwrap();

        assert_eq!(std::fs::read_to_string(&path).unwrap(), "existing\nblob\n");
    }

    #[test]
    fn append_blob_preserves_blob_for_empty_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("note.md");
        std::fs::write(&path, "").unwrap();

        append_blob_atomic(&path, "blob").unwrap();

        assert_eq!(std::fs::read_to_string(&path).unwrap(), "blob");
    }

    #[test]
    fn insert_blob_at_heading_path_inserts_before_next_sibling_heading() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("note.md");
        std::fs::write(&path, "# Weekly Log\n\n## Monday\n\n## Tuesday\n").unwrap();

        insert_blob_at_heading_path_atomic(&path, "Weekly Log/Monday", "- shipped insert mode")
            .unwrap();

        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "# Weekly Log\n\n## Monday\n\n- shipped insert mode\n## Tuesday\n"
        );
    }

    #[test]
    fn insert_blob_at_heading_path_uses_document_end_for_last_section() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("note.md");
        std::fs::write(&path, "# Daily Log\n\n## Notes\nexisting").unwrap();

        insert_blob_at_heading_path_atomic(&path, "Daily Log/Notes", "new").unwrap();

        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "# Daily Log\n\n## Notes\nexisting\nnew"
        );
    }

    #[test]
    fn insert_blob_at_heading_path_errors_when_heading_path_is_missing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("note.md");
        std::fs::write(&path, "# Daily Log\n").unwrap();

        let result = insert_blob_at_heading_path_atomic(&path, "Daily Log/Tasks", "new");

        assert!(result.is_err());
    }
}
