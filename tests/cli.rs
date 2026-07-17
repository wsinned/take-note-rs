use assert_cmd::cargo::cargo_bin_cmd;
use chrono::{Datelike, Duration, NaiveDate, Weekday};
use predicates::prelude::*;
use std::path::Path;

#[test]
fn help_succeeds() {
    cargo_bin_cmd!("take-note")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "A CLI for creating and managing weekly and daily markdown notes",
        ));
}

#[test]
fn invalid_when_fails_without_creating_notes() {
    let home = tempfile::tempdir().unwrap();
    let notes = home.path().join("notes");

    cargo_bin_cmd!("take-note")
        .env("HOME", home.path())
        .env("USERPROFILE", home.path())
        .args(["daily", "someday", "--notes-folder"])
        .arg(&notes)
        .arg("--no-open")
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid when option"));

    assert!(!notes.exists());
}

#[test]
fn daily_json_mode_creates_reported_note() {
    let home = tempfile::tempdir().unwrap();
    let notes = home.path().join("notes");

    let output = cargo_bin_cmd!("take-note")
        .env("HOME", home.path())
        .env("USERPROFILE", home.path())
        .args(["daily", "today", "--notes-folder"])
        .arg(&notes)
        .args(["--no-open", "--format", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let results: serde_json::Value = serde_json::from_slice(&output).unwrap();
    let result = &results[0];
    assert_eq!(result["created"], true);
    assert!(std::path::Path::new(result["path"].as_str().unwrap()).is_file());
}

#[test]
fn weekly_json_batch_creates_and_reports_notes_without_overwriting_them() {
    let home = tempfile::tempdir().unwrap();
    let notes = home.path().join("notes");
    std::fs::create_dir(&notes).unwrap();
    std::fs::write(notes.join("weekly.md"), "# Week of {{date}}\n").unwrap();

    let first_results = run_weekly_json(home.path(), &notes);
    assert_eq!(first_results.len(), 3);

    let mut preserved_contents = Vec::new();
    let mut previous_date = None;
    for (index, result) in first_results.iter().enumerate() {
        assert_eq!(result["created"], true);

        let date = NaiveDate::parse_from_str(result["date"].as_str().unwrap(), "%Y-%m-%d").unwrap();
        assert_eq!(date.weekday(), Weekday::Mon);
        if let Some(previous_date) = previous_date {
            assert_eq!(date - previous_date, Duration::days(7));
        }
        previous_date = Some(date);

        let expected_path = notes
            .join(format!("{:04}/{:02}", date.year(), date.month()))
            .join(format!(
                "{:04}-{:02}-{:02}-Weekly-log.md",
                date.year(),
                date.month(),
                date.day()
            ));
        assert_eq!(Path::new(result["path"].as_str().unwrap()), expected_path);
        assert_eq!(
            std::fs::read_to_string(&expected_path).unwrap(),
            format!("# Week of {}\n", date.format("%A %d %B %Y"))
        );

        let preserved = format!("preserve existing weekly note {index}\n");
        std::fs::write(&expected_path, &preserved).unwrap();
        preserved_contents.push((expected_path, preserved));
    }

    let second_results = run_weekly_json(home.path(), &notes);
    assert_eq!(second_results.len(), 3);
    assert!(
        second_results
            .iter()
            .all(|result| result["created"] == false)
    );
    for (path, expected) in preserved_contents {
        assert_eq!(std::fs::read_to_string(path).unwrap(), expected);
    }
}

#[test]
fn weekly_text_mode_reports_created_paths() {
    let home = tempfile::tempdir().unwrap();
    let notes = home.path().join("notes");

    let output = cargo_bin_cmd!("take-note")
        .env("HOME", home.path())
        .env("USERPROFILE", home.path())
        .args(["weekly", "thisWeek", "--batch", "2", "--notes-folder"])
        .arg(&notes)
        .arg("--no-open")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output = String::from_utf8(output).unwrap();
    let reported_paths: Vec<_> = output
        .lines()
        .map(|line| line.strip_prefix("Created: ").unwrap())
        .collect();
    assert_eq!(reported_paths.len(), 2);
    assert!(reported_paths.iter().all(|path| Path::new(path).is_file()));
}

#[test]
fn weekly_silent_mode_has_no_output() {
    let home = tempfile::tempdir().unwrap();
    let notes = home.path().join("notes");

    cargo_bin_cmd!("take-note")
        .env("HOME", home.path())
        .env("USERPROFILE", home.path())
        .args(["weekly", "thisWeek", "--notes-folder"])
        .arg(&notes)
        .args(["--no-open", "--format", "silent"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());

    assert!(notes.is_dir());
}

fn run_weekly_json(home: &Path, notes: &Path) -> Vec<serde_json::Value> {
    let output = cargo_bin_cmd!("take-note")
        .env("HOME", home)
        .env("USERPROFILE", home)
        .args(["weekly", "thisWeek", "--batch", "3", "--notes-folder"])
        .arg(notes)
        .args(["--template", "weekly.md", "--no-open", "--format", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    serde_json::from_slice(&output).unwrap()
}
