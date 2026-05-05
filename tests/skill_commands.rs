use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_skill_validate_help() {
    let mut cmd = Command::cargo_bin("brief").unwrap();
    cmd.arg("skill").arg("validate").arg("--help");
    cmd.assert().success();
}

#[test]
fn test_skill_init_help() {
    let mut cmd = Command::cargo_bin("brief").unwrap();
    cmd.arg("skill").arg("init").arg("--help");
    cmd.assert().success();
}

#[test]
fn test_skill_validate_fails_on_missing_file() {
    let mut cmd = Command::cargo_bin("brief").unwrap();
    cmd.arg("skill").arg("validate").arg("non_existent_skill.md");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("not found at"));
}

#[test]
fn test_skill_validate_checks_description_length() {
    let dir = tempdir().unwrap();
    let skill_path = dir.path().join("SKILL.md");
    let long_description = "a".repeat(1025);
    let content = format!(
        "---\nname: test-skill\ndescription: {}\n---\n\nBody content\n",
        long_description
    );
    fs::write(&skill_path, content).unwrap();

    let mut cmd = Command::cargo_bin("brief").unwrap();
    cmd.arg("skill").arg("validate").arg(&skill_path);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("must be ≤1024 chars"));
}

#[test]
fn test_skill_validate_checks_line_count() {
    let dir = tempdir().unwrap();
    let skill_path = dir.path().join("SKILL.md");
    let mut content = String::from("---\nname: test-skill\ndescription: test\n---\n\n");
    for _ in 0..501 {
        content.push_str("Line\n");
    }
    fs::write(&skill_path, content).unwrap();

    let mut cmd = Command::cargo_bin("brief").unwrap();
    cmd.arg("skill").arg("validate").arg(&skill_path);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("must be < 500 lines"));
}

#[test]
fn test_skill_validate_checks_name_format() {
    let dir = tempdir().unwrap();
    let skill_path = dir.path().join("SKILL.md");
    let content = "---\nname: Invalid Name!\ndescription: test\n---\n\nBody content\n";
    fs::write(&skill_path, content).unwrap();

    let mut cmd = Command::cargo_bin("brief").unwrap();
    cmd.arg("skill").arg("validate").arg(&skill_path);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("name format"));
}

#[test]
fn test_skill_validate_rejects_leading_hyphen() {
    let dir = tempdir().unwrap();
    let skill_path = dir.path().join("SKILL.md");
    let content = "---\nname: -foo\ndescription: test\n---\n\nBody\n";
    fs::write(&skill_path, content).unwrap();

    let mut cmd = Command::cargo_bin("brief").unwrap();
    cmd.arg("skill").arg("validate").arg(&skill_path);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("start or end with a hyphen"));
}

#[test]
fn test_skill_validate_rejects_trailing_hyphen() {
    let dir = tempdir().unwrap();
    let skill_path = dir.path().join("SKILL.md");
    let content = "---\nname: foo-\ndescription: test\n---\n\nBody\n";
    fs::write(&skill_path, content).unwrap();

    let mut cmd = Command::cargo_bin("brief").unwrap();
    cmd.arg("skill").arg("validate").arg(&skill_path);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("start or end with a hyphen"));
}

#[test]
fn test_skill_validate_rejects_consecutive_hyphens() {
    let dir = tempdir().unwrap();
    let skill_path = dir.path().join("SKILL.md");
    let content = "---\nname: foo--bar\ndescription: test\n---\n\nBody\n";
    fs::write(&skill_path, content).unwrap();

    let mut cmd = Command::cargo_bin("brief").unwrap();
    cmd.arg("skill").arg("validate").arg(&skill_path);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("consecutive hyphens"));
}

#[test]
fn test_skill_validate_rejects_name_over_64_chars() {
    let dir = tempdir().unwrap();
    let skill_path = dir.path().join("SKILL.md");
    let long_name = "a".repeat(65);
    let content = format!(
        "---\nname: {}\ndescription: test\n---\n\nBody\n",
        long_name
    );
    fs::write(&skill_path, content).unwrap();

    let mut cmd = Command::cargo_bin("brief").unwrap();
    cmd.arg("skill").arg("validate").arg(&skill_path);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("too long"));
}

#[test]
fn test_skill_validate_rejects_angle_brackets_in_description() {
    let dir = tempdir().unwrap();
    let skill_path = dir.path().join("SKILL.md");
    let content =
        "---\nname: foo\ndescription: Use <tool> to process input\n---\n\nBody\n";
    fs::write(&skill_path, content).unwrap();

    let mut cmd = Command::cargo_bin("brief").unwrap();
    cmd.arg("skill").arg("validate").arg(&skill_path);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("angle brackets"));
}

#[test]
fn test_skill_init_creates_directory_structure() {
    let out_dir = tempdir().unwrap();
    let mut cmd = Command::cargo_bin("brief").unwrap();
    cmd.current_dir(out_dir.path())
        .arg("skill")
        .arg("init")
        .arg("my-skill-name");
    cmd.assert().success();

    let skill_dir = out_dir.path().join("my-skill-name");
    assert!(skill_dir.exists());
    assert!(skill_dir.join("SKILL.md").exists());
    assert!(skill_dir.join("scripts").exists());
    assert!(skill_dir.join("references").exists());
}

#[test]
fn test_skill_init_skeleton_is_valid() {
    let out_dir = tempdir().unwrap();
    let mut cmd = Command::cargo_bin("brief").unwrap();
    cmd.current_dir(out_dir.path())
        .arg("skill")
        .arg("init")
        .arg("hello-world");
    cmd.assert().success();

    let skill_dir = out_dir.path().join("hello-world");
    let skill_md = fs::read_to_string(skill_dir.join("SKILL.md")).unwrap();
    assert!(skill_md.starts_with("---\n"));
    assert!(skill_md.contains("name: hello-world"));
    assert!(skill_md.contains("description:"));
    assert!(skill_md.contains("# Hello World"));

    // The freshly initialized skeleton should pass `brief skill validate`.
    let mut validate_cmd = Command::cargo_bin("brief").unwrap();
    validate_cmd.arg("skill").arg("validate").arg(&skill_dir);
    validate_cmd.assert().success();
}

#[test]
fn test_skill_init_defaults_to_current_directory_name() {
    let parent = tempdir().unwrap();
    let working = parent.path().join("my-default-skill");
    fs::create_dir(&working).unwrap();

    let mut cmd = Command::cargo_bin("brief").unwrap();
    cmd.current_dir(&working).arg("skill").arg("init");
    cmd.assert().success();

    assert!(working.join("my-default-skill").exists());
    assert!(working.join("my-default-skill").join("SKILL.md").exists());
}

#[test]
fn test_skill_init_refuses_to_overwrite() {
    let out_dir = tempdir().unwrap();
    fs::create_dir(out_dir.path().join("existing")).unwrap();

    let mut cmd = Command::cargo_bin("brief").unwrap();
    cmd.current_dir(out_dir.path())
        .arg("skill")
        .arg("init")
        .arg("existing");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));
}

#[test]
fn test_skill_init_rejects_uppercase_name() {
    let out_dir = tempdir().unwrap();
    let mut cmd = Command::cargo_bin("brief").unwrap();
    cmd.current_dir(out_dir.path())
        .arg("skill")
        .arg("init")
        .arg("BadName");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("name format"));
}

#[test]
fn test_skill_init_rejects_trailing_hyphen() {
    let out_dir = tempdir().unwrap();
    let mut cmd = Command::cargo_bin("brief").unwrap();
    cmd.current_dir(out_dir.path())
        .arg("skill")
        .arg("init")
        .arg("bad-");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("start or end with a hyphen"));
}

#[test]
fn test_skill_init_rejects_consecutive_hyphens() {
    let out_dir = tempdir().unwrap();
    let mut cmd = Command::cargo_bin("brief").unwrap();
    cmd.current_dir(out_dir.path())
        .arg("skill")
        .arg("init")
        .arg("foo--bar");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("consecutive hyphens"));
}

#[test]
fn test_skill_init_rejects_name_over_64_chars() {
    let out_dir = tempdir().unwrap();
    let long_name = "a".repeat(65);
    let mut cmd = Command::cargo_bin("brief").unwrap();
    cmd.current_dir(out_dir.path())
        .arg("skill")
        .arg("init")
        .arg(&long_name);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("too long"));
}

#[test]
fn test_skill_validate_sample_docx_skill() {
    let mut cmd = Command::cargo_bin("brief").unwrap();
    cmd.arg("skill")
        .arg("validate")
        .arg("tests/fixtures/sample-skills/docx");

    // The sample docx skill is intentionally too long (590 lines),
    // so it should FAIL validation.
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("SKILL.md is too long"));
}

#[test]
fn test_skill_emit() {
    let mut cmd = Command::cargo_bin("brief").unwrap();
    cmd.arg("--file")
        .arg("tests/fixtures/skill.brief.md")
        .arg("skill")
        .arg("emit");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("name: review"))
        .stdout(predicate::str::contains(
            "description: Review code changes following team standards",
        ));
}

#[test]
fn test_skill_emit_install_stamps_metadata_brief_source() {
    let dir = tempdir().unwrap();
    let brief_path = dir.path().join("review.brief.md");
    let content = "---\nstack: [Rust]\nskill_name: review\nskill_description: Review code\n---\n\n# Review code\n\n## Deliverable\nReview comments.\n";
    fs::write(&brief_path, content).unwrap();

    let mut cmd = Command::cargo_bin("brief").unwrap();
    cmd.current_dir(dir.path())
        .arg("--file")
        .arg(&brief_path)
        .arg("skill")
        .arg("emit")
        .arg("--install");
    cmd.assert().success();

    let installed = dir.path().join(".claude/skills/review/SKILL.md");
    let body = fs::read_to_string(&installed).unwrap();
    assert!(
        body.contains("metadata:\n  brief.source: ../../../review.brief.md"),
        "expected metadata.brief.source pointing back to the brief, got:\n{body}"
    );
}

#[test]
fn test_skill_emit_install() {
    let dir = tempdir().unwrap();
    let mut cmd = Command::cargo_bin("brief").unwrap();

    // We need to run in a temp dir to avoid messing with real .claude/skills
    cmd.current_dir(dir.path())
        .arg("--file")
        .arg(
            std::env::current_dir()
                .unwrap()
                .join("tests/fixtures/skill.brief.md"),
        )
        .arg("skill")
        .arg("emit")
        .arg("--install");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains(
            "Installed .claude/skills/review/SKILL.md",
        ));

    assert!(dir.path().join(".claude/skills/review/SKILL.md").exists());
}

#[test]
fn test_skill_emit_fails_when_slugified_name_exceeds_limit() {
    let dir = tempdir().unwrap();
    let brief_path = dir.path().join("brief.md");
    // Goal that slugifies to a name that exceeds 64 characters.
    let content = "---\nstack: [Rust]\n---\n\n# this is an extremely long goal that when slugified produces a name that definitely exceeds sixty four characters\n\n## Deliverable\nSomething.\n";
    fs::write(&brief_path, content).unwrap();

    let mut cmd = Command::cargo_bin("brief").unwrap();
    cmd.arg("--file").arg(&brief_path).arg("skill").arg("emit");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("would not pass"));
}
