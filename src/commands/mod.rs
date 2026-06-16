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

fn write_atomic(path: &Path, content: &[u8]) -> std::io::Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("take-note");

    let mut last_error = None;

    for attempt in 0..100 {
        let temp_path = parent.join(format!(
            ".{file_name}.tmp.{}.{}",
            std::process::id(),
            attempt
        ));

        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path);

        let mut file = match file {
            Ok(file) => file,
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                last_error = Some(err);
                continue;
            }
            Err(err) => return Err(err),
        };

        if let Err(err) = file.write_all(content).and_then(|_| file.sync_all()) {
            let _ = std::fs::remove_file(&temp_path);
            return Err(err);
        }

        drop(file);
        if let Err(err) = std::fs::rename(&temp_path, path) {
            let _ = std::fs::remove_file(&temp_path);
            return Err(err);
        }

        return Ok(());
    }

    Err(last_error.unwrap_or_else(|| std::io::Error::other("could not create temporary file")))
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
}
