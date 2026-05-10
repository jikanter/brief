use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

fn write_minimal_brief(dir: &std::path::Path) -> std::path::PathBuf {
    let brief = dir.join(".brief.md");
    fs::write(
        &brief,
        "---\nstack: [Rust]\n---\n\n# Fix the login bug\n\n## Constraints\n\n### Hard\n- Do not break existing tests\n\n## Sacred\n- `src/auth.rs` — Authentication logic\n",
    )
    .unwrap();
    brief
}

#[test]
fn cli_emit_cursor_prints_to_stdout() {
    let dir = tempdir().unwrap();
    let brief = write_minimal_brief(dir.path());

    let mut cmd = Command::cargo_bin("brief").unwrap();
    cmd.arg("--file").arg(&brief).arg("emit").arg("cursor");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("---\n"))
        .stdout(predicate::str::contains("alwaysApply: true"))
        .stdout(predicate::str::contains("description: Fix the login bug"))
        .stdout(predicate::str::contains("# Fix the login bug"))
        .stdout(predicate::str::contains("## Required"))
        .stdout(predicate::str::contains("- Do not break existing tests"));
}

#[test]
fn cli_emit_cursor_install_writes_mdc_file() {
    let dir = tempdir().unwrap();
    let brief = write_minimal_brief(dir.path());

    let mut cmd = Command::cargo_bin("brief").unwrap();
    cmd.current_dir(dir.path())
        .arg("--file")
        .arg(&brief)
        .arg("emit")
        .arg("cursor")
        .arg("--install");
    cmd.assert()
        .success()
        .stderr(predicate::str::is_empty().or(predicate::str::contains("Installed")));

    let written = dir.path().join(".cursor").join("rules").join("brief.mdc");
    assert!(written.exists(), "expected {written:?} to exist");
    let content = fs::read_to_string(&written).unwrap();
    assert!(content.contains("# Fix the login bug"));
    assert!(content.contains("alwaysApply: true"));
}
