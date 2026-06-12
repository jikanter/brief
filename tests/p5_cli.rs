use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

fn full_brief(dir: &std::path::Path) -> std::path::PathBuf {
    let p = dir.join(".brief.md");
    fs::write(
        &p,
        "---\nstack: [Rust]\nskill_name: review\nskill_description: Review code changes\n---\n\n# Review code\n\n## Constraints\n\n### Hard\n- Do not break existing tests\n\n## Sacred\n- `src/auth.rs` — Authentication logic\n\n## Commands\n\n- Build: `cargo build`\n- Test: `cargo test`\n",
    )
    .unwrap();
    p
}

// -- --position --

#[test]
fn position_top_prepends_with_preamble() {
    let dir = tempdir().unwrap();
    full_brief(dir.path());
    fs::write(
        dir.path().join("CLAUDE.md"),
        "# Existing\n\nHand-written.\n",
    )
    .unwrap();

    Command::cargo_bin("brief")
        .unwrap()
        .current_dir(dir.path())
        .args([
            "--file",
            ".brief.md",
            "emit",
            "claude",
            "--install",
            "--position",
            "top",
        ])
        .assert()
        .success();

    let md = fs::read_to_string(dir.path().join("CLAUDE.md")).unwrap();
    assert!(md.starts_with("<brief:generated>"));
    assert!(md.contains("supplement the project instructions below"));
    assert!(md.find("# Briefing").unwrap() < md.find("# Existing").unwrap());
}

#[test]
fn position_after_heading_inserts_within_section() {
    let dir = tempdir().unwrap();
    full_brief(dir.path());
    fs::write(
        dir.path().join("CLAUDE.md"),
        "# Project\n\n## Setup\n\nsteps\n\n## Usage\n\nusage\n",
    )
    .unwrap();

    Command::cargo_bin("brief")
        .unwrap()
        .current_dir(dir.path())
        .args([
            "--file",
            ".brief.md",
            "emit",
            "claude",
            "--install",
            "--position",
            "after:Setup",
        ])
        .assert()
        .success();

    let md = fs::read_to_string(dir.path().join("CLAUDE.md")).unwrap();
    assert!(md.find("steps").unwrap() < md.find("# Briefing").unwrap());
    assert!(md.find("# Briefing").unwrap() < md.find("## Usage").unwrap());
}

#[test]
fn position_on_non_claude_target_is_rejected() {
    let dir = tempdir().unwrap();
    let brief = full_brief(dir.path());
    Command::cargo_bin("brief")
        .unwrap()
        .current_dir(dir.path())
        .arg("--file")
        .arg(&brief)
        .args(["emit", "agents-md", "--install", "--position", "top"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "--position is only supported for the claude target",
        ));
}

#[test]
fn invalid_position_value_is_rejected() {
    let dir = tempdir().unwrap();
    let brief = full_brief(dir.path());
    Command::cargo_bin("brief")
        .unwrap()
        .current_dir(dir.path())
        .arg("--file")
        .arg(&brief)
        .args(["emit", "claude", "--install", "--position", "sideways"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid --position"));
}

// -- --full --

#[test]
fn full_install_writes_section_skill_hook_and_permissions() {
    let dir = tempdir().unwrap();
    let brief = full_brief(dir.path());

    Command::cargo_bin("brief")
        .unwrap()
        .current_dir(dir.path())
        .arg("--file")
        .arg(&brief)
        .args(["emit", "claude", "--full"])
        .assert()
        .success()
        .stdout(predicate::str::contains(".claude/skills/review/SKILL.md"))
        .stdout(predicate::str::contains("command permission"));

    assert!(dir.path().join("CLAUDE.md").exists());
    assert!(dir.path().join(".claude/skills/review/SKILL.md").exists());

    let settings = fs::read_to_string(dir.path().join(".claude/settings.json")).unwrap();
    assert!(settings.contains("brief check --hook"), "hook registered");
    assert!(
        settings.contains("Bash(cargo build:*)"),
        "command permission added"
    );
    assert!(settings.contains("Bash(cargo test:*)"));
}

// -- --uninstall --

#[test]
fn uninstall_reverses_full_install() {
    let dir = tempdir().unwrap();
    let brief = full_brief(dir.path());

    // Install everything.
    Command::cargo_bin("brief")
        .unwrap()
        .current_dir(dir.path())
        .arg("--file")
        .arg(&brief)
        .args(["emit", "claude", "--full"])
        .assert()
        .success();
    assert!(dir.path().join(".claude/skills/review/SKILL.md").exists());

    // Uninstall.
    Command::cargo_bin("brief")
        .unwrap()
        .current_dir(dir.path())
        .arg("--file")
        .arg(&brief)
        .args(["emit", "claude", "--uninstall"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Removed"));

    let md = fs::read_to_string(dir.path().join("CLAUDE.md")).unwrap();
    assert!(!md.contains("<brief:generated>"), "brief section removed");
    assert!(
        !dir.path().join(".claude/skills/review").exists(),
        "skill removed"
    );
    let settings = fs::read_to_string(dir.path().join(".claude/settings.json")).unwrap();
    assert!(!settings.contains("brief check --hook"), "hook removed");
}

#[test]
fn uninstall_cannot_combine_with_install() {
    let dir = tempdir().unwrap();
    let brief = full_brief(dir.path());
    Command::cargo_bin("brief")
        .unwrap()
        .current_dir(dir.path())
        .arg("--file")
        .arg(&brief)
        .args(["emit", "claude", "--install", "--uninstall"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be combined"));
}
