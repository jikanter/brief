use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

fn write_brief(dir: &std::path::Path) -> std::path::PathBuf {
    let brief = dir.join(".brief.md");
    fs::write(
        &brief,
        "---\nstack: [Rust]\nmodel: claude-opus-4-8\n---\n\n# Fix the login bug\n\n## Constraints\n\n### Hard\n- Do not break existing tests\n\n### Soft\n- small focused commits\n\n### Ask First\n- Schema changes\n\n## Sacred\n- `src/auth.rs` — Authentication logic\n",
    )
    .unwrap();
    brief
}

// -- copilot --

#[test]
fn cli_emit_copilot_prints_descriptive_markdown() {
    let dir = tempdir().unwrap();
    let brief = write_brief(dir.path());

    Command::cargo_bin("brief")
        .unwrap()
        .arg("--file")
        .arg(&brief)
        .arg("emit")
        .arg("copilot")
        .assert()
        .success()
        .stdout(predicate::str::contains("# Fix the login bug"))
        .stdout(predicate::str::contains("## Requirements"))
        .stdout(predicate::str::contains("- Do not break existing tests"))
        .stdout(predicate::str::contains("## Protected files"))
        .stdout(predicate::str::contains("**IMPORTANT:**").not());
}

#[test]
fn cli_emit_copilot_install_writes_github_file() {
    let dir = tempdir().unwrap();
    let brief = write_brief(dir.path());

    Command::cargo_bin("brief")
        .unwrap()
        .current_dir(dir.path())
        .arg("--file")
        .arg(&brief)
        .arg("emit")
        .arg("copilot")
        .arg("--install")
        .assert()
        .success()
        .stdout(predicate::str::contains("Installed"));

    let written = dir.path().join(".github").join("copilot-instructions.md");
    assert!(written.exists());
    let content = fs::read_to_string(&written).unwrap();
    assert!(content.contains("<brief:generated>"));
    assert!(content.contains("# Fix the login bug"));
}

// -- windsurf --

#[test]
fn cli_emit_windsurf_prints_always_on_rule() {
    let dir = tempdir().unwrap();
    let brief = write_brief(dir.path());

    Command::cargo_bin("brief")
        .unwrap()
        .arg("--file")
        .arg(&brief)
        .arg("emit")
        .arg("windsurf")
        .assert()
        .success()
        .stdout(predicate::str::contains("trigger: always_on"))
        .stdout(predicate::str::contains("# Fix the login bug"))
        .stdout(predicate::str::contains("## Required"));
}

#[test]
fn cli_emit_windsurf_install_writes_rule_file() {
    let dir = tempdir().unwrap();
    let brief = write_brief(dir.path());

    Command::cargo_bin("brief")
        .unwrap()
        .current_dir(dir.path())
        .arg("--file")
        .arg(&brief)
        .arg("emit")
        .arg("windsurf")
        .arg("--install")
        .assert()
        .success()
        .stdout(predicate::str::contains("Installed"));

    let written = dir.path().join(".windsurf").join("rules").join("brief.md");
    assert!(written.exists());
    let content = fs::read_to_string(&written).unwrap();
    assert!(content.contains("trigger: always_on"));
}

// -- aider --

#[test]
fn cli_emit_aider_prints_conversational_conventions() {
    let dir = tempdir().unwrap();
    let brief = write_brief(dir.path());

    Command::cargo_bin("brief")
        .unwrap()
        .arg("--file")
        .arg(&brief)
        .arg("emit")
        .arg("aider")
        .assert()
        .success()
        .stdout(predicate::str::contains("# Fix the login bug"))
        .stdout(predicate::str::contains("## Guidelines"))
        .stdout(predicate::str::contains("- Prefer: small focused commits"))
        .stdout(predicate::str::contains("- Ask before: Schema changes"))
        .stdout(predicate::str::contains("## Files not to modify"));
}

#[test]
fn cli_emit_aider_install_writes_both_files() {
    let dir = tempdir().unwrap();
    let brief = write_brief(dir.path());

    Command::cargo_bin("brief")
        .unwrap()
        .current_dir(dir.path())
        .arg("--file")
        .arg(&brief)
        .arg("emit")
        .arg("aider")
        .arg("--install")
        .assert()
        .success()
        .stdout(predicate::str::contains("CONVENTIONS.md"))
        .stdout(predicate::str::contains(".aider.conf.yml"));

    let conventions = dir.path().join("CONVENTIONS.md");
    let conf = dir.path().join(".aider.conf.yml");
    assert!(conventions.exists());
    assert!(conf.exists());

    let conf_content = fs::read_to_string(&conf).unwrap();
    assert!(conf_content.contains("CONVENTIONS.md"));
    // model from frontmatter lands in the aider config.
    assert!(conf_content.contains("claude-opus-4-8"));
}

#[test]
fn cli_emit_prompt_install_is_rejected() {
    let dir = tempdir().unwrap();
    let brief = write_brief(dir.path());

    Command::cargo_bin("brief")
        .unwrap()
        .arg("--file")
        .arg(&brief)
        .arg("emit")
        .arg("prompt")
        .arg("--install")
        .assert()
        .failure()
        .stderr(predicate::str::contains("--install is only supported"));
}
