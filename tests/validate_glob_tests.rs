//! `docs/bugs.md`: `brief validate` warned "matches no files" for a sacred
//! `dir/**` whose directory was flat.

use assert_cmd::Command;
use predicates::prelude::PredicateBooleanExt;
use tempfile::TempDir;

fn brief_with_sacred(entries: &[(&str, &str)]) -> String {
    let mut s = String::from("---\nstack: [Rust]\n---\n\n# Repro\n\n## Sacred\n\n");
    for (path, reason) in entries {
        s.push_str(&format!("- `{path}` — {reason}\n"));
    }
    s
}

fn write(dir: &TempDir, rel: &str, body: &str) {
    let p = dir.path().join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, body).unwrap();
}

#[test]
fn validate_does_not_warn_on_a_flat_sacred_directory() {
    let dir = TempDir::new().unwrap();
    write(
        &dir,
        ".brief.md",
        &brief_with_sacred(&[("flat/**", "reviewed")]),
    );
    write(&dir, "flat/a.md", "x");

    Command::cargo_bin("brief")
        .unwrap()
        .current_dir(dir.path())
        .arg("validate")
        .assert()
        .success()
        .stderr(predicates::str::contains("matches no files").not());
}

#[test]
fn validate_still_warns_when_a_sacred_pattern_really_matches_nothing() {
    let dir = TempDir::new().unwrap();
    write(
        &dir,
        ".brief.md",
        &brief_with_sacred(&[("missing/**", "reviewed")]),
    );

    Command::cargo_bin("brief")
        .unwrap()
        .current_dir(dir.path())
        .arg("validate")
        .assert()
        .success()
        .stderr(predicates::str::contains(
            "Sacred path `missing/**` matches no files",
        ));
}
