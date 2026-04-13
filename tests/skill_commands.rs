use std::process::Command;
use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::fs;
use tempfile::tempdir;
use std::io::Write;

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
fn test_skill_validate_sample_docx_skill() {
    let mut cmd = Command::cargo_bin("brief").unwrap();
    cmd.arg("skill").arg("validate").arg("tests/fixtures/sample-skills/docx");
    
    // The sample skill is intentionally long (> 500 lines) and should fail validation.
    // It is okay that it fails; we document this failure here.
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("SKILL.md is too long"));
}

#[test]
fn test_skill_scaffold_from_workflow_creates_directory() {
    let dir = tempdir().unwrap();
    let workflow_path = dir.path().join("workflow.txt");
    fs::write(&workflow_path, "ls -la\ncat README.md\ncargo build").unwrap();

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
    assert!(skill_dir.join("SKILL.md").exists());
    let skill_md = fs::read_to_string(skill_dir.join("SKILL.md")).unwrap();
    assert!(skill_md.contains("ls -la"));
    assert!(skill_md.contains("cat README.md"));
    assert!(skill_md.contains("cargo build"));
}

#[test]
fn test_skill_scaffold_interactive_creates_directory() {
    let out_dir = tempdir().unwrap();
    let mut cmd = Command::cargo_bin("brief").unwrap();
    cmd.current_dir(out_dir.path())
        .arg("skill")
        .arg("scaffold")
        .arg("--interactive")
        .stdin(std::process::Stdio::piped());
    
    let mut child = cmd.spawn().expect("failed to spawn child");
    let mut stdin = child.stdin.take().expect("failed to get stdin");
    stdin.write_all(b"interactive-skill\nA descriptive skill\nStep 1: Do something\nStep 2: Done\n\n")
        .expect("failed to write to stdin");
    drop(stdin);

    let output = child.wait_with_output().expect("failed to wait for output");
    assert!(output.status.success());

    let skill_dir = out_dir.path().join("interactive-skill");
    assert!(skill_dir.exists());
    assert!(skill_dir.join("SKILL.md").exists());
    let skill_md = fs::read_to_string(skill_dir.join("SKILL.md")).unwrap();
    assert!(skill_md.contains("name: interactive-skill"));
    assert!(skill_md.contains("description: A descriptive skill"));
    assert!(skill_md.contains("Step 1: Do something"));
    assert!(skill_md.contains("Step 2: Done"));
}

#[test]
fn test_skill_scaffold_interactive_validates_input() {
    let out_dir = tempdir().unwrap();
    let mut cmd = Command::cargo_bin("brief").unwrap();
    cmd.current_dir(out_dir.path())
        .arg("skill")
        .arg("scaffold")
        .arg("--interactive")
        .stdin(std::process::Stdio::piped());
    
    let mut child = cmd.spawn().expect("failed to spawn child");
    let mut stdin = child.stdin.take().expect("failed to get stdin");
    // 1. Invalid Name (spaces)
    // 2. Invalid Name (uppercase)
    // 3. Valid Name
    // 4. Empty Description
    // 5. Valid Description
    // 6. Instructions
    // 7. Empty line to end instructions
    stdin.write_all(b"invalid name\nInvalidName\nvalid-name\n\nValid description\nInstruction line\n\n")
        .expect("failed to write to stdin");
    drop(stdin);

    let output = child.wait_with_output().expect("failed to wait for output");
    assert!(output.status.success());

    let skill_dir = out_dir.path().join("valid-name");
    assert!(skill_dir.exists());
    let skill_md = fs::read_to_string(skill_dir.join("SKILL.md")).unwrap();
    assert!(skill_md.contains("name: valid-name"));
    assert!(skill_md.contains("description: Valid description"));
}
