use predicates::prelude::*;
use std::fs;
use tempfile::tempdir;

fn write_brief(dir: &std::path::Path) -> std::path::PathBuf {
    let brief = dir.join(".brief.md");
    fs::write(
        &brief,
        "---\nstack: [Rust]\n---\n\n# Guard sacred regions\n\n## Sacred\n- `src/auth/**` — Authentication boundary\n",
    )
    .unwrap();
    brief
}

fn bin() -> assert_cmd::Command {
    assert_cmd::Command::cargo_bin("brief").unwrap()
}

// -- check --hook (PreToolUse protocol) --

#[test]
fn hook_denies_edit_to_sacred_file() {
    let dir = tempdir().unwrap();
    let brief = write_brief(dir.path());

    let event = r#"{"hook_event_name":"PreToolUse","tool_name":"Edit","tool_input":{"file_path":"src/auth/handler.rs"}}"#;
    bin()
        .arg("--file")
        .arg(&brief)
        .arg("check")
        .arg("--hook")
        .write_stdin(event)
        .assert()
        .success() // exit 0; the JSON decision blocks the edit
        .stdout(predicate::str::contains("\"permissionDecision\":\"deny\""))
        .stdout(predicate::str::contains("Authentication boundary"));
}

#[test]
fn hook_allows_edit_to_normal_file() {
    let dir = tempdir().unwrap();
    let brief = write_brief(dir.path());

    let event = r#"{"tool_name":"Edit","tool_input":{"file_path":"src/api/routes.rs"}}"#;
    bin()
        .arg("--file")
        .arg(&brief)
        .arg("check")
        .arg("--hook")
        .write_stdin(event)
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

#[test]
fn hook_ignores_non_file_events() {
    let dir = tempdir().unwrap();
    let brief = write_brief(dir.path());

    let event = r#"{"tool_name":"Bash","tool_input":{"command":"ls"}}"#;
    bin()
        .arg("--file")
        .arg(&brief)
        .arg("check")
        .arg("--hook")
        .write_stdin(event)
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

// -- emit claude --install --hooks --

#[test]
fn install_hooks_registers_pretooluse_in_settings() {
    let dir = tempdir().unwrap();
    write_brief(dir.path());

    bin()
        .current_dir(dir.path())
        .arg("emit")
        .arg("claude")
        .arg("--install")
        .arg("--hooks")
        .assert()
        .success()
        .stdout(predicate::str::contains("Registered"));

    let settings = fs::read_to_string(dir.path().join(".claude/settings.json")).unwrap();
    assert!(settings.contains("PreToolUse"));
    assert!(settings.contains("Edit|Write"));
    assert!(settings.contains("brief check --hook"));
    // CLAUDE.md also written (install implied).
    assert!(dir.path().join("CLAUDE.md").exists());
}

#[test]
fn install_hooks_is_idempotent() {
    let dir = tempdir().unwrap();
    write_brief(dir.path());

    for _ in 0..2 {
        bin()
            .current_dir(dir.path())
            .arg("emit")
            .arg("claude")
            .arg("--install")
            .arg("--hooks")
            .assert()
            .success();
    }

    let settings = fs::read_to_string(dir.path().join(".claude/settings.json")).unwrap();
    let v: serde_json::Value = serde_json::from_str(&settings).unwrap();
    assert_eq!(
        v["hooks"]["PreToolUse"].as_array().unwrap().len(),
        1,
        "hook must not duplicate across re-installs"
    );
}

#[test]
fn hooks_flag_rejected_for_non_claude_target() {
    let dir = tempdir().unwrap();
    write_brief(dir.path());

    bin()
        .current_dir(dir.path())
        .arg("emit")
        .arg("cursor")
        .arg("--hooks")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "only supported for the claude target",
        ));
}
