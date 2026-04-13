use std::process::Command;
use assert_cmd::prelude::*;
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
fn test_skill_validate_sample_docx_skill() {
    let mut cmd = Command::cargo_bin("brief").unwrap();
    cmd.arg("skill").arg("validate").arg("tests/fixtures/sample-skill/docx");
    
    // Now that we've shortened the docx SKILL.md to < 500 lines, it should pass.
    cmd.assert().success();
}
