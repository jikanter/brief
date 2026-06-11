use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

// -- anchor target --

#[test]
fn cli_emit_anchor_frames_and_wraps() {
    let dir = tempdir().unwrap();
    let brief = dir.path().join(".brief.md");
    fs::write(
        &brief,
        "---\nstack: [Rust]\n---\n\n# Ship it\n\n## Constraints\n\n### Hard\n- Do not break the public API\n- Must pass CI\n- Use thiserror for errors\n\n### Ask First\n- Changing the schema\n\n## Sacred\n- `src/auth.rs` — Auth\n",
    )
    .unwrap();

    Command::cargo_bin("brief")
        .unwrap()
        .arg("--file")
        .arg(&brief)
        .arg("emit")
        .arg("anchor")
        .assert()
        .success()
        .stdout(predicate::str::contains("<brief:anchor>"))
        .stdout(predicate::str::contains("</brief:anchor>"))
        .stdout(predicate::str::contains("NEVER: break the public API"))
        .stdout(predicate::str::contains("MUST: pass CI"))
        // Convention stays plain.
        .stdout(predicate::str::contains("Use thiserror for errors"))
        .stdout(predicate::str::contains("STOP before: Changing the schema"))
        .stdout(predicate::str::contains(
            "Sacred (do not modify): src/auth.rs",
        ));
}

#[test]
fn cli_emit_anchor_install_is_rejected() {
    let dir = tempdir().unwrap();
    let brief = dir.path().join(".brief.md");
    fs::write(&brief, "---\nstack: [Rust]\n---\n\n# Goal\n").unwrap();

    Command::cargo_bin("brief")
        .unwrap()
        .arg("--file")
        .arg(&brief)
        .arg("emit")
        .arg("anchor")
        .arg("--install")
        .assert()
        .failure()
        .stderr(predicate::str::contains("--install is only supported"));
}

// -- specificity validation --

#[test]
fn cli_validate_warns_on_vague_constraint_but_succeeds() {
    let dir = tempdir().unwrap();
    let brief = dir.path().join(".brief.md");
    fs::write(
        &brief,
        "---\nstack: [Rust]\n---\n\n# Fix it\n\n## Constraints\n\n### Soft\n- Follow best practices\n",
    )
    .unwrap();

    Command::cargo_bin("brief")
        .unwrap()
        .arg("--file")
        .arg(&brief)
        .arg("validate")
        .assert()
        .success()
        .stderr(predicate::str::contains("Vague constraint"));
}

#[test]
fn cli_validate_does_not_flag_specific_constraint() {
    let dir = tempdir().unwrap();
    let brief = dir.path().join(".brief.md");
    fs::write(
        &brief,
        "---\nstack: [Rust]\n---\n\n# Fix it\n\n## Constraints\n\n### Hard\n- All public functions must return `Result<T, AppError>`\n",
    )
    .unwrap();

    Command::cargo_bin("brief")
        .unwrap()
        .arg("--file")
        .arg(&brief)
        .arg("validate")
        .assert()
        .success()
        .stderr(predicate::str::contains("Vague constraint").not());
}

// -- conflict detection on install --

#[test]
fn cli_install_warns_on_conflicting_constraint() {
    let dir = tempdir().unwrap();
    let brief = dir.path().join(".brief.md");
    fs::write(
        &brief,
        "---\nstack: [Rust]\n---\n\n# Fix it\n\n## Constraints\n\n### Hard\n- Use tabs for indentation\n",
    )
    .unwrap();
    // Pre-existing CLAUDE.md carries the opposite-polarity rule.
    fs::write(
        dir.path().join("CLAUDE.md"),
        "# Project\n\n## Style\n\n- Never use tabs for indentation\n",
    )
    .unwrap();

    Command::cargo_bin("brief")
        .unwrap()
        .current_dir(dir.path())
        .arg("--file")
        .arg(&brief)
        .arg("emit")
        .arg("claude")
        .arg("--install")
        .assert()
        .success()
        .stderr(predicate::str::contains("may conflict"))
        .stdout(predicate::str::contains("Installed"));
}

#[test]
fn cli_install_no_false_conflict_on_unrelated_rules() {
    let dir = tempdir().unwrap();
    let brief = dir.path().join(".brief.md");
    fs::write(
        &brief,
        "---\nstack: [Rust]\n---\n\n# Fix it\n\n## Constraints\n\n### Hard\n- Use tabs for indentation\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("CLAUDE.md"),
        "# Project\n\n- Never deploy on Fridays\n",
    )
    .unwrap();

    Command::cargo_bin("brief")
        .unwrap()
        .current_dir(dir.path())
        .arg("--file")
        .arg(&brief)
        .arg("emit")
        .arg("claude")
        .arg("--install")
        .assert()
        .success()
        .stderr(predicate::str::contains("may conflict").not());
}
