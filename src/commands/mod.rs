pub mod daily;
pub mod init;
pub mod weekly;

use std::fs::OpenOptions;
use std::io::{ErrorKind, Write};
use std::path::Path;

pub(crate) fn create_note(path: &Path, content: &str) -> std::io::Result<bool> {
    match OpenOptions::new().write(true).create_new(true).open(path) {
        Ok(mut file) => {
            file.write_all(content.as_bytes())?;
            Ok(true)
        }
        Err(error) if error.kind() == ErrorKind::AlreadyExists => Ok(false),
        Err(error) => Err(error),
    }
}

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
    // Resolve symlinks before replacing the file so an append updates the note
    // target rather than replacing the link itself.
    let target = path.canonicalize()?;
    let permissions = std::fs::metadata(&target)?.permissions();
    let parent = target.parent().unwrap_or_else(|| Path::new("."));
    let file_name = target
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("take-note");

    let mut temp_file = tempfile::Builder::new()
        .prefix(&format!(".{file_name}.tmp."))
        .tempfile_in(parent)?;

    temp_file.write_all(content)?;
    temp_file.as_file().set_permissions(permissions)?;
    temp_file.as_file().sync_all()?;

    temp_file
        .persist(target)
        .map(|_| ())
        .map_err(|err| err.error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_note_does_not_overwrite_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("note.md");
        std::fs::write(&path, "existing").unwrap();

        let created = create_note(&path, "replacement").unwrap();

        assert!(!created);
        assert_eq!(std::fs::read_to_string(path).unwrap(), "existing");
    }

    #[test]
    fn concurrent_note_creation_has_exactly_one_winner() {
        use std::sync::{Arc, Barrier};

        let dir = tempfile::tempdir().unwrap();
        let path = Arc::new(dir.path().join("note.md"));
        let barrier = Arc::new(Barrier::new(2));
        let mut threads = Vec::new();

        for content in ["first", "second"] {
            let path = Arc::clone(&path);
            let barrier = Arc::clone(&barrier);
            threads.push(std::thread::spawn(move || {
                barrier.wait();
                (content, create_note(&path, content).unwrap())
            }));
        }

        let results: Vec<_> = threads
            .into_iter()
            .map(|thread| thread.join().unwrap())
            .collect();
        let winners: Vec<_> = results
            .iter()
            .filter_map(|(content, created)| created.then_some(*content))
            .collect();

        assert_eq!(winners.len(), 1);
        assert_eq!(std::fs::read_to_string(&*path).unwrap(), winners[0]);
    }

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

    #[cfg(unix)]
    #[test]
    fn append_blob_preserves_unix_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("note.md");
        std::fs::write(&path, "existing").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o640)).unwrap();

        append_blob_atomic(&path, "blob").unwrap();

        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o640
        );
    }

    #[cfg(unix)]
    #[test]
    fn append_blob_through_symlink_updates_target_and_preserves_link() {
        use std::os::unix::fs::symlink;

        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("note.md");
        let link = dir.path().join("linked-note.md");
        std::fs::write(&target, "existing").unwrap();
        symlink(&target, &link).unwrap();

        append_blob_atomic(&link, "blob").unwrap();

        assert_eq!(std::fs::read_to_string(&target).unwrap(), "existing\nblob");
        assert!(
            std::fs::symlink_metadata(&link)
                .unwrap()
                .file_type()
                .is_symlink()
        );
    }
}
