//! `brief_version` end to end: authored in YAML, carried through the parser,
//! round-tripped into canonical JSON, and gated by `brief validate`.

use assert_cmd::prelude::*;
use brief_cli::emit::emit_json;
use brief_cli::parse::parse_brief;
use predicates::prelude::*;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

const BODY: &str = "\n# Fix the login bug\n\n## Constraints\n\n### Hard\n- Do not break existing tests\n\n## Deliverable\nA working login flow.\n";

fn emitted(frontmatter: &str) -> serde_json::Value {
    let source = format!("---\n{frontmatter}---\n{BODY}");
    let brief = parse_brief(&source).expect("brief must parse");
    serde_json::from_str(&emit_json(&brief)).expect("emit json must parse")
}

fn write_brief(dir: &std::path::Path, frontmatter: &str) -> std::path::PathBuf {
    let path = dir.join(".brief.md");
    fs::write(&path, format!("---\n{frontmatter}---\n{BODY}")).unwrap();
    path
}

#[test]
fn authored_brief_version_round_trips_into_canonical_json() {
    let value = emitted("stack: [Rust]\nbrief_version: \"1\"\n");
    assert_eq!(value["frontmatter"]["brief_version"], "1");
}

#[test]
fn omitted_brief_version_defaults_to_one_in_canonical_json() {
    let value = emitted("stack: [Rust]\n");
    assert_eq!(value["frontmatter"]["brief_version"], "1");
}

#[test]
fn legacy_version_key_round_trips_as_brief_version() {
    let value = emitted("stack: [Rust]\nversion: \"1\"\n");
    assert_eq!(value["frontmatter"]["brief_version"], "1");
    assert!(
        value["frontmatter"].get("version").is_none(),
        "canonical JSON carries only the `brief_version` spelling"
    );
}

#[test]
fn validate_rejects_an_unknown_brief_version() {
    let dir = tempdir().unwrap();
    let brief = write_brief(dir.path(), "stack: [Rust]\nbrief_version: \"2\"\n");

    Command::cargo_bin("brief")
        .unwrap()
        .arg("--file")
        .arg(&brief)
        .arg("validate")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unsupported `brief_version`: `2`"));
}

#[test]
fn validate_accepts_the_supported_brief_version() {
    let dir = tempdir().unwrap();
    let brief = write_brief(dir.path(), "stack: [Rust]\nbrief_version: \"1\"\n");

    Command::cargo_bin("brief")
        .unwrap()
        .arg("--file")
        .arg(&brief)
        .arg("validate")
        .assert()
        .success();
}
