//! `brief emit jsonl`: one JSON line per brief, carrying the id and the
//! objective it was computed over. Lines append into a stream that aichat's
//! label table and rocky join on `brief_id`. The contract is
//! docs/schema/SPEC.md §7.

use assert_cmd::prelude::*;
use brief_cli::emit::emit_jsonl;
use brief_cli::id::canonical_json;
use brief_cli::parse::parse_brief;
use predicates::prelude::*;
use serde_json::Value;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

const MINIMAL: &str = "---\nstack: [Rust]\n---\n\n# G\n";
/// sha256 of the canonical objective of [`MINIMAL`], computed with `shasum -a 256`.
const MINIMAL_ID: &str = "sha256:519052810012bfa20a99c6495e9cdbeed9ad8187f06b5e985af38f685d1f98f7";
/// sha256 of tests/fixtures/golden/brief-id.objective.json.
const GOLDEN_ID: &str = "sha256:2b939f8f5f0c3337e269db05822810adb2402b3d2405c43f161f9983f294851a";

const FULL: &str = "---
stack: [Rust]
---

# Review Rust changes

## Constraints

### Hard
- Never touch generated code

## Deliverable
A review comment per finding.
";

fn fixture(rel: &str) -> String {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

fn write_brief(dir: &std::path::Path, name: &str, src: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    fs::write(&path, src).unwrap();
    path
}

fn brief_cmd(file: &std::path::Path) -> Command {
    let mut cmd = Command::cargo_bin("brief").unwrap();
    cmd.arg("--file").arg(file);
    cmd
}

fn stdout_of(file: &std::path::Path, args: &[&str]) -> String {
    let out = brief_cmd(file).args(args).output().unwrap();
    assert!(out.status.success(), "{out:?}");
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn emit_jsonl_is_one_canonical_line_of_id_plus_objective() {
    let brief = parse_brief(MINIMAL).unwrap();
    assert_eq!(
        emit_jsonl(&brief),
        format!(
            "{{\"constraints\":{{\"ask_first\":[],\"hard\":[],\"soft\":[]}},\
             \"deliverable\":null,\"goal\":\"G\",\"id\":\"{MINIMAL_ID}\"}}\n"
        )
    );
}

#[test]
fn emit_jsonl_line_minus_its_id_is_the_hashed_objective() {
    let brief = parse_brief(&fixture("tests/fixtures/brief-id.brief.md")).unwrap();
    let out = emit_jsonl(&brief);
    let line = out.strip_suffix('\n').expect("newline-terminated");
    assert!(!line.contains('\n'), "one line per brief: {line}");

    let mut value: Value = serde_json::from_str(line).unwrap();
    let object = value.as_object_mut().unwrap();
    assert_eq!(object.remove("id").unwrap(), GOLDEN_ID);
    assert_eq!(
        canonical_json(&value),
        fixture("tests/fixtures/golden/brief-id.objective.json")
    );
}

#[test]
fn cli_emit_jsonl_prints_the_line_and_lines_append_into_a_stream() {
    let dir = tempdir().unwrap();
    let file = write_brief(dir.path(), "a.brief.md", MINIMAL);
    let first = stdout_of(&file, &["emit", "jsonl"]);
    assert_eq!(first, emit_jsonl(&parse_brief(MINIMAL).unwrap()));

    let other = write_brief(dir.path(), "b.brief.md", FULL);
    let stream = format!("{first}{}", stdout_of(&other, &["emit", "jsonl"]));
    let ids: Vec<String> = stream
        .lines()
        .map(|l| serde_json::from_str::<Value>(l).unwrap()["id"].to_string())
        .collect();
    assert_eq!(ids.len(), 2);
    assert_ne!(ids[0], ids[1]);
}

#[test]
fn cli_emit_jsonl_is_stdout_only() {
    let dir = tempdir().unwrap();
    let file = write_brief(dir.path(), "a.brief.md", MINIMAL);
    brief_cmd(&file)
        .args(["emit", "jsonl", "--install"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--install"));
}
