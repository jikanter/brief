use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

// -- scaffold --

#[test]
fn scaffold_from_description_creates_skill_dir() {
    let dir = tempdir().unwrap();

    Command::cargo_bin("brief")
        .unwrap()
        .current_dir(dir.path())
        .args([
            "skill",
            "scaffold",
            "--description",
            "Review PRs for security",
            "--name",
            "security-review",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Scaffolded"));

    let skill = dir.path().join("security-review");
    assert!(skill.join("SKILL.md").exists());
    assert!(skill.join("scripts").is_dir());
    assert!(skill.join("references").is_dir());
    let md = fs::read_to_string(skill.join("SKILL.md")).unwrap();
    assert!(md.contains("name: security-review"));
    assert!(md.contains("brief.source:"));
}

#[test]
fn scaffold_from_brief_stamps_relative_source() {
    let dir = tempdir().unwrap();
    let brief = dir.path().join("review.brief.md");
    fs::write(
        &brief,
        "---\nstack: [Rust]\nskill_name: review\nskill_description: Review code changes\n---\n\n# Review code\n\n## Deliverable\nComments.\n",
    )
    .unwrap();

    Command::cargo_bin("brief")
        .unwrap()
        .current_dir(dir.path())
        .args(["skill", "scaffold", "--from-brief", "review.brief.md"])
        .assert()
        .success();

    let md = fs::read_to_string(dir.path().join("review").join("SKILL.md")).unwrap();
    assert!(md.contains("name: review"));
    assert!(md.contains("brief.source: ../review.brief.md"));
}

#[test]
fn scaffold_without_any_source_fails() {
    let dir = tempdir().unwrap();
    // No .brief.md in scope, no flags.
    Command::cargo_bin("brief")
        .unwrap()
        .current_dir(dir.path())
        .arg("--file")
        .arg(dir.path().join("nonexistent.brief.md"))
        .args(["skill", "scaffold"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("scaffold needs a source"));
}

// -- install / uninstall --

#[test]
fn install_then_uninstall_roundtrip() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("review");
    fs::create_dir_all(&src).unwrap();
    fs::write(
        src.join("SKILL.md"),
        "---\nname: review\ndescription: Review code\nmetadata:\n  brief.source: ../../x.brief.md\n---\n\nGuidance.\n",
    )
    .unwrap();

    // install
    Command::cargo_bin("brief")
        .unwrap()
        .current_dir(dir.path())
        .args(["skill", "install", "review"])
        .assert()
        .success()
        .stdout(predicate::str::contains(".claude/skills/review/SKILL.md"));
    assert!(dir.path().join(".claude/skills/review/SKILL.md").exists());

    // uninstall
    Command::cargo_bin("brief")
        .unwrap()
        .current_dir(dir.path())
        .args(["skill", "uninstall", "review"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Uninstalled"));
    assert!(!dir.path().join(".claude/skills/review").exists());
}

#[test]
fn uninstall_refuses_non_brief_skill_without_force() {
    let dir = tempdir().unwrap();
    let installed = dir.path().join(".claude/skills/handmade");
    fs::create_dir_all(&installed).unwrap();
    fs::write(
        installed.join("SKILL.md"),
        "---\nname: handmade\ndescription: hand authored\n---\n\nBody\n",
    )
    .unwrap();

    Command::cargo_bin("brief")
        .unwrap()
        .current_dir(dir.path())
        .args(["skill", "uninstall", "handmade"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not brief-managed"));
    assert!(installed.exists(), "must not remove without --force");

    Command::cargo_bin("brief")
        .unwrap()
        .current_dir(dir.path())
        .args(["skill", "uninstall", "handmade", "--force"])
        .assert()
        .success();
    assert!(!installed.exists());
}

// -- search --

#[test]
fn search_finds_installed_skill() {
    let dir = tempdir().unwrap();
    let s = dir.path().join(".claude/skills/review-code");
    fs::create_dir_all(&s).unwrap();
    fs::write(
        s.join("SKILL.md"),
        "---\nname: review-code\ndescription: Helps review pull requests\n---\n\nB\n",
    )
    .unwrap();

    Command::cargo_bin("brief")
        .unwrap()
        .current_dir(dir.path())
        .args(["skill", "search", "review"])
        .assert()
        .success()
        .stdout(predicate::str::contains("review-code"));
}

#[test]
fn search_no_match_exits_nonzero() {
    let dir = tempdir().unwrap();
    fs::create_dir_all(dir.path().join(".claude/skills")).unwrap();

    Command::cargo_bin("brief")
        .unwrap()
        .current_dir(dir.path())
        .args(["skill", "search", "nothingmatches"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("no skills match"));
}
