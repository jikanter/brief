use predicates::prelude::*;
use std::fs;
use tempfile::tempdir;

fn write_brief(dir: &std::path::Path) -> std::path::PathBuf {
    let brief = dir.join(".brief.md");
    fs::write(
        &brief,
        "---\nstack: [Rust]\n---\n\n# Guard sacred regions\n\n## Sacred\n- `src/auth/**` — Authentication boundary\n- `migrations/` — Immutable history\n",
    )
    .unwrap();
    brief
}

#[test]
fn validate_diff_help_works() {
    let mut cmd = assert_cmd::Command::cargo_bin("brief").unwrap();
    cmd.arg("validate-diff").arg("--help");
    cmd.assert().success();
}

#[test]
fn stdin_clean_diff_succeeds() {
    let dir = tempdir().unwrap();
    let brief = write_brief(dir.path());

    let mut cmd = assert_cmd::Command::cargo_bin("brief").unwrap();
    cmd.arg("--file")
        .arg(&brief)
        .arg("validate-diff")
        .arg("--stdin")
        .write_stdin("src/api/routes.rs\nREADME.md\n");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("none in a sacred region"));
}

#[test]
fn stdin_sacred_diff_fails_with_reason() {
    let dir = tempdir().unwrap();
    let brief = write_brief(dir.path());

    let mut cmd = assert_cmd::Command::cargo_bin("brief").unwrap();
    cmd.arg("--file")
        .arg(&brief)
        .arg("validate-diff")
        .arg("--stdin")
        .write_stdin("src/auth/handler.rs\nsrc/api/routes.rs\n");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("src/auth/handler.rs"))
        .stderr(predicate::str::contains("Authentication boundary"));
}

#[test]
fn stdin_json_report_is_machine_readable() {
    let dir = tempdir().unwrap();
    let brief = write_brief(dir.path());

    let mut cmd = assert_cmd::Command::cargo_bin("brief").unwrap();
    cmd.arg("--file")
        .arg(&brief)
        .arg("validate-diff")
        .arg("--stdin")
        .arg("--json")
        .write_stdin("migrations/001_init.sql\n");
    // JSON output still exits non-zero on violations.
    cmd.assert()
        .failure()
        .stdout(predicate::str::contains("\"violations\""))
        .stdout(predicate::str::contains("migrations/001_init.sql"))
        .stdout(predicate::str::contains("Immutable history"));
}

#[test]
fn stdin_empty_diff_is_clean() {
    let dir = tempdir().unwrap();
    let brief = write_brief(dir.path());

    let mut cmd = assert_cmd::Command::cargo_bin("brief").unwrap();
    cmd.arg("--file")
        .arg(&brief)
        .arg("validate-diff")
        .arg("--stdin")
        .write_stdin("");
    cmd.assert().success();
}
