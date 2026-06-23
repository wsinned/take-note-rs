#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Heading<'a> {
    level: usize,
    title: &'a str,
    start: usize,
}

/// Returns the byte offset where content can be inserted at the end of a heading path.
///
/// The returned position is the start of the next heading at the same or higher level,
/// or the end of the document if the target section runs to the end of the file.
pub fn section_end_for_heading_path(content: &str, path: &[&str]) -> Option<usize> {
    if path.is_empty() {
        return None;
    }

    let headings = parse_headings(content);
    find_heading_path(&headings, path, 0, 1, content.len())
}

fn find_heading_path(
    headings: &[Heading<'_>],
    path: &[&str],
    start_index: usize,
    min_level: usize,
    search_end: usize,
) -> Option<usize> {
    let target = path.first()?;

    for (index, heading) in headings.iter().enumerate().skip(start_index) {
        if heading.start >= search_end {
            break;
        }
        if heading.level < min_level {
            continue;
        }
        if heading.title != *target {
            continue;
        }

        let section_end = section_end_after(headings, index, search_end);
        if path.len() == 1 {
            return Some(section_end);
        }

        if let Some(pos) = find_heading_path(
            headings,
            &path[1..],
            index + 1,
            heading.level + 1,
            section_end,
        ) {
            return Some(pos);
        }
    }

    None
}

fn section_end_after(headings: &[Heading<'_>], current_index: usize, search_end: usize) -> usize {
    let current_level = headings[current_index].level;

    headings
        .iter()
        .skip(current_index + 1)
        .find(|heading| heading.start < search_end && heading.level <= current_level)
        .map(|heading| heading.start)
        .unwrap_or(search_end)
}

fn parse_headings(content: &str) -> Vec<Heading<'_>> {
    let mut headings = Vec::new();
    let mut offset = 0usize;

    for chunk in content.split_inclusive('\n') {
        let line = chunk.strip_suffix('\n').unwrap_or(chunk);
        if let Some((level, title)) = parse_heading(line) {
            headings.push(Heading {
                level,
                title,
                start: offset,
            });
        }
        offset += chunk.len();
    }

    if !content.ends_with('\n') && offset < content.len() {
        let line = &content[offset..];
        if let Some((level, title)) = parse_heading(line) {
            headings.push(Heading {
                level,
                title,
                start: offset,
            });
        }
    }

    headings
}

fn parse_heading(line: &str) -> Option<(usize, &str)> {
    let trimmed = line.trim_end();
    let bytes = trimmed.as_bytes();

    let mut level = 0usize;
    while level < bytes.len() && bytes[level] == b'#' {
        level += 1;
    }

    if level == 0 || level > 6 || bytes.get(level) != Some(&b' ') {
        return None;
    }

    let title = trimmed[level + 1..].trim();
    if title.is_empty() {
        return None;
    }

    Some((level, title))
}

#[cfg(test)]
mod tests {
    use super::section_end_for_heading_path;

    #[test]
    fn finds_end_of_empty_section_before_next_sibling_heading() {
        let content = "# Weekly Log\n\n## Monday\n\n## Tuesday\n";

        let pos = section_end_for_heading_path(content, &["Weekly Log", "Monday"]).unwrap();

        assert_eq!(pos, content.find("## Tuesday").unwrap());
    }

    #[test]
    fn finds_end_of_nested_section_before_next_sibling_heading() {
        let content = "# Weekly Log\n\n## Wednesday\n- [ ] Pick up Isabelle\n\n## Thursday\n";

        let pos = section_end_for_heading_path(content, &["Weekly Log", "Wednesday"]).unwrap();

        assert_eq!(pos, content.find("## Thursday").unwrap());
    }

    #[test]
    fn uses_document_end_when_section_runs_to_end_of_file() {
        let content = "# Weekly Log\n\n## Sunday\n- [ ] Done\n";

        let pos = section_end_for_heading_path(content, &["Weekly Log", "Sunday"]).unwrap();

        assert_eq!(pos, content.len());
    }

    #[test]
    fn returns_none_when_path_does_not_exist() {
        let content = "# Weekly Log\n\n## Monday\n";

        assert!(section_end_for_heading_path(content, &["Weekly Log", "Friday"]).is_none());
    }
}
