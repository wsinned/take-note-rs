use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

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
