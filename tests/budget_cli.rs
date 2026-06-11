use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

fn write_brief(dir: &std::path::Path) -> std::path::PathBuf {
    let brief = dir.join(".brief.md");
    fs::write(
        &brief,
        "---\nstack: [Rust, PostgreSQL]\ncontext: [./README.md]\n---\n\n# Fix the login bug\n\n## Constraints\n\n### Hard\n- Do not break existing tests\n\n## Sacred\n- `src/auth.rs` — Authentication logic\n\n## Assumptions\n- [ ] Sessions are stored in Redis\n\n## Deliverable\nA working login flow.\n",
    )
    .unwrap();
    brief
}

#[test]
fn budget_flag_reports_to_stderr_not_stdout() {
    let dir = tempdir().unwrap();
    let brief = write_brief(dir.path());

    Command::cargo_bin("brief")
        .unwrap()
        .arg("--file")
        .arg(&brief)
        .arg("emit")
        .arg("prompt")
        .arg("--budget")
        .assert()
        .success()
        .stderr(predicate::str::contains("budget:"))
        .stderr(predicate::str::contains("tokens"))
        // The report must not pollute the emitted stdout.
        .stdout(predicate::str::contains("budget:").not());
}

#[test]
fn no_budget_flag_keeps_stderr_quiet_under_threshold() {
    let dir = tempdir().unwrap();
    let brief = write_brief(dir.path());

    // A small brief under threshold and without --budget produces no stderr.
    Command::cargo_bin("brief")
        .unwrap()
        .arg("--file")
        .arg(&brief)
        .arg("emit")
        .arg("claude")
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}

#[test]
fn over_budget_warns_even_without_budget_flag() {
    let dir = tempdir().unwrap();
    let brief = write_brief(dir.path());

    // Force a tiny budget so any output is over it.
    Command::cargo_bin("brief")
        .unwrap()
        .arg("--file")
        .arg(&brief)
        .arg("emit")
        .arg("prompt")
        .arg("--max-tokens")
        .arg("1")
        .assert()
        .success()
        .stderr(predicate::str::contains("over the 1-token budget"))
        .stderr(predicate::str::contains("--compact"));
}

#[test]
fn compact_strips_reference_prose() {
    let dir = tempdir().unwrap();
    let brief = write_brief(dir.path());

    // Full emit includes stack and assumptions; compact drops them but keeps
    // the goal, constraints, sacred regions, and deliverable.
    Command::cargo_bin("brief")
        .unwrap()
        .arg("--file")
        .arg(&brief)
        .arg("emit")
        .arg("prompt")
        .arg("--compact")
        .assert()
        .success()
        .stdout(predicate::str::contains("Fix the login bug"))
        // Hard constraint "Do not break existing tests" is polarity-framed (P6).
        .stdout(predicate::str::contains("NEVER: break existing tests"))
        .stdout(predicate::str::contains("src/auth.rs"))
        .stdout(predicate::str::contains("A working login flow."))
        .stdout(predicate::str::contains("STACK:").not())
        .stdout(predicate::str::contains("ASSUMPTIONS").not());
}

#[test]
fn compact_emits_fewer_tokens_than_full() {
    let dir = tempdir().unwrap();
    let brief = write_brief(dir.path());

    let full = Command::cargo_bin("brief")
        .unwrap()
        .arg("--file")
        .arg(&brief)
        .arg("emit")
        .arg("prompt")
        .output()
        .unwrap();
    let compact = Command::cargo_bin("brief")
        .unwrap()
        .arg("--file")
        .arg(&brief)
        .arg("emit")
        .arg("prompt")
        .arg("--compact")
        .output()
        .unwrap();

    assert!(
        compact.stdout.len() < full.stdout.len(),
        "compact output ({}) should be smaller than full ({})",
        compact.stdout.len(),
        full.stdout.len()
    );
}
