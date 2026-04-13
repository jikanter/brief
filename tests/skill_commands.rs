use std::process::Command;
use assert_cmd::prelude::*;
use assert_cmd::Command as AssertCommand;
use predicates::prelude::*;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_skill_validate_help() {
    let mut cmd = Command::cargo_bin("brief").unwrap();
    cmd.arg("skill").arg("validate").arg("--help");
    cmd.assert().success();
}

#[test]
fn test_skill_scaffold_help() {
    let mut cmd = Command::cargo_bin("brief").unwrap();
    cmd.arg("skill").arg("scaffold").arg("--help");
    cmd.assert().success();
}

#[test]
fn test_skill_validate_fails_on_missing_file() {
    let mut cmd = Command::cargo_bin("brief").unwrap();
    cmd.arg("skill").arg("validate").arg("non_existent_skill.md");
    cmd.assert().failure().stderr(predicate::str::contains("not found at"));
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
    cmd.assert().failure().stderr(predicate::str::contains("must be ≤1024 chars"));
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
    cmd.assert().failure().stderr(predicate::str::contains("must be < 500 lines"));
}

#[test]
fn test_skill_validate_checks_name_format() {
    let dir = tempdir().unwrap();
    let skill_path = dir.path().join("SKILL.md");
    let content = "---\nname: Invalid Name!\ndescription: test\n---\n\nBody content\n";
    fs::write(&skill_path, content).unwrap();

    let mut cmd = Command::cargo_bin("brief").unwrap();
    cmd.arg("skill").arg("validate").arg(&skill_path);
    cmd.assert().failure().stderr(predicate::str::contains("name format"));
}

#[test]
fn test_skill_scaffold_from_doc_creates_directory() {
    let dir = tempdir().unwrap();
    let doc_path = dir.path().join("doc.md");
    fs::write(&doc_path, "test-skill-name\nInstructions for skill").unwrap();

    let out_dir = tempdir().unwrap();
    let mut cmd = Command::cargo_bin("brief").unwrap();
    cmd.current_dir(out_dir.path())
        .arg("skill")
        .arg("scaffold")
        .arg("--from-doc")
        .arg(&doc_path);
    cmd.assert().success();

    let skill_dir = out_dir.path().join("test-skill-name");
    assert!(skill_dir.exists());
    assert!(skill_dir.join("SKILL.md").exists());
    assert!(skill_dir.join("scripts").exists());
    assert!(skill_dir.join("references").exists());
}

#[test]
fn test_skill_scaffold_from_workflow_creates_directory() {
    let dir = tempdir().unwrap();
    let workflow_path = dir.path().join("workflow.txt");
    fs::write(&workflow_path, "step 1\nstep 2").unwrap();

    let out_dir = tempdir().unwrap();
    let mut cmd = Command::cargo_bin("brief").unwrap();
    cmd.current_dir(out_dir.path())
        .arg("skill")
        .arg("scaffold")
        .arg("--from-workflow")
        .arg(&workflow_path);
    cmd.assert().success();

    let skill_dir = out_dir.path().join("workflow-skill");
    assert!(skill_dir.exists());
    let skill_md = fs::read_to_string(skill_dir.join("SKILL.md")).unwrap();
    assert!(skill_md.contains("Follow these steps from the observed workflow:"));
    assert!(skill_md.contains("step 1"));
    assert!(skill_md.contains("step 2"));
}

#[test]
fn test_skill_scaffold_interactive_creates_directory() {
    let out_dir = tempdir().unwrap();
    let mut cmd = AssertCommand::cargo_bin("brief").unwrap();
    
    // Simulate interactive input
    // 1. Skill name: my-cool-skill
    // 2. Description: A very cool skill
    // 3. Instructions: Do cool things\n(empty line to end)
    let input = "my-cool-skill\nA very cool skill\nDo cool things\n\n";
    
    cmd.current_dir(out_dir.path())
        .arg("skill")
        .arg("scaffold")
        .arg("--interactive")
        .write_stdin(input)
        .assert()
        .success();

    let skill_dir = out_dir.path().join("my-cool-skill");
    assert!(skill_dir.exists());
    let skill_md = fs::read_to_string(skill_dir.join("SKILL.md")).unwrap();
    assert!(skill_md.contains("name: my-cool-skill"));
    assert!(skill_md.contains("description: A very cool skill"));
    assert!(skill_md.contains("Do cool things"));
}

#[test]
fn test_skill_scaffold_interactive_validates_input() {
    let out_dir = tempdir().unwrap();
    let mut cmd = AssertCommand::cargo_bin("brief").unwrap();
    
    // Simulate invalid input then valid input
    // 1. Invalid name: "Invalid Name" (has space and caps)
    // 2. Valid name: "valid-name"
    // 3. Invalid description: empty
    // 4. Valid description: "A valid description"
    // 5. Instructions: "Just do it"\n(empty line)
    let input = "Invalid Name\nvalid-name\n\nA valid description\nJust do it\n\n";
    
    cmd.current_dir(out_dir.path())
        .arg("skill")
        .arg("scaffold")
        .arg("--interactive")
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("Error: Name must be non-empty"));

    let skill_dir = out_dir.path().join("valid-name");
    assert!(skill_dir.exists());
}

#[test]
fn test_skill_validate_sample_docx_skill() {
    let mut cmd = Command::cargo_bin("brief").unwrap();
    cmd.arg("skill").arg("validate").arg("tests/fixtures/sample-skills/docx");
    
    // The sample docx skill is intentionally too long (590 lines),
    // so it should FAIL validation.
    cmd.assert().failure().stderr(predicate::str::contains("SKILL.md is too long"));
}
