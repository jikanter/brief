//! `docs/bugs.md`: `brief check` marked non-sacred files sacred, reproduced
//! through the command a hook actually runs.

use assert_cmd::Command;
use tempfile::TempDir;

fn brief_with_sacred(entries: &[(&str, &str)]) -> String {
    let mut s = String::from("---\nstack: [Rust]\n---\n\n# Repro\n\n## Sacred\n\n");
    for (path, reason) in entries {
        s.push_str(&format!("- `{path}` — {reason}\n"));
    }
    s
}

fn write(dir: &TempDir, rel: &str, body: &str) {
    let p = dir.path().join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, body).unwrap();
}

#[test]
fn check_does_not_claim_a_sibling_directory_is_sacred() {
    let dir = TempDir::new().unwrap();
    write(
        &dir,
        ".brief.md",
        &brief_with_sacred(&[("flat/**", "reviewed")]),
    );
    write(&dir, "flat/a.md", "x");
    write(&dir, "flatter/x.md", "x");

    Command::cargo_bin("brief")
        .unwrap()
        .current_dir(dir.path())
        .args(["check", "flatter/x.md"])
        .assert()
        .success();

    // The pattern's own directory is still sacred, with the message hooks parse.
    Command::cargo_bin("brief")
        .unwrap()
        .current_dir(dir.path())
        .args(["check", "flat/a.md"])
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "flat/a.md is in sacred region `flat/**`",
        ));
}

/// The hook path runs `check` on a file that is about to be created, so the
/// verdict must not come from walking the filesystem.
#[test]
fn check_answers_the_same_for_a_file_that_does_not_exist_yet() {
    let dir = TempDir::new().unwrap();
    write(
        &dir,
        ".brief.md",
        &brief_with_sacred(&[("src/auth/**", "audited")]),
    );

    Command::cargo_bin("brief")
        .unwrap()
        .current_dir(dir.path())
        .args(["check", "src/auth/brand_new.rs"])
        .assert()
        .failure();

    Command::cargo_bin("brief")
        .unwrap()
        .current_dir(dir.path())
        .args(["check", "src/authz/brand_new.rs"])
        .assert()
        .success();
}
