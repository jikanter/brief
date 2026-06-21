//! P8: scoped constraints — `[glob, ...]` authoring syntax, non-destructive
//! prose rendering across emitters, native Cursor glob fan-out, and a
//! scope-matches-nothing validation lint. End-to-end through the CLI.

use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

const SCOPED_BRIEF: &str = "---\nstack: [Rust]\n---\n\n# Ship scoped constraints\n\n## Constraints\n\n### Hard\n- Do not break existing tests\n- [`src/ui/**`] Use design tokens, not raw hex\n- [`src/api/**`] All handlers return `Result<T, ApiError>`\n\n### Soft\n- [`src/ui/**`] Prefer composition over inheritance\n";

fn write_brief(dir: &std::path::Path) -> std::path::PathBuf {
    let brief = dir.join(".brief.md");
    fs::write(&brief, SCOPED_BRIEF).unwrap();
    brief
}

// -- prose rendering (non-destructive: scope survives in every emitter) --

#[test]
fn cli_emit_claude_renders_scope_as_prose() {
    let dir = tempdir().unwrap();
    let brief = write_brief(dir.path());

    Command::cargo_bin("brief")
        .unwrap()
        .arg("--file")
        .arg(&brief)
        .args(["emit", "claude"])
        .assert()
        .success()
        // Unscoped constraint is framed as before.
        .stdout(predicate::str::contains("NEVER: break existing tests"))
        // Scoped constraints carry their glob inline — nothing is dropped.
        .stdout(predicate::str::contains(
            "When working in `src/ui/**`: Use design tokens",
        ))
        .stdout(predicate::str::contains("When working in `src/api/**`:"));
}

#[test]
fn cli_emit_json_carries_scope() {
    let dir = tempdir().unwrap();
    let brief = write_brief(dir.path());

    let out = Command::cargo_bin("brief")
        .unwrap()
        .arg("--file")
        .arg(&brief)
        .args(["emit", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
    let hard = &v["constraints"]["hard"];
    assert_eq!(hard[0]["text"], "Do not break existing tests");
    assert!(hard[0].get("scope").is_none(), "unscoped omits scope");
    assert_eq!(hard[1]["scope"][0], "src/ui/**");
}

// -- native Cursor glob fan-out --

#[test]
fn cli_emit_cursor_install_fans_out_per_scope() {
    let dir = tempdir().unwrap();
    let brief = write_brief(dir.path());

    Command::cargo_bin("brief")
        .unwrap()
        .current_dir(dir.path())
        .arg("--file")
        .arg(&brief)
        .args(["emit", "cursor", "--install"])
        .assert()
        .success();

    let rules = dir.path().join(".cursor/rules");
    // Base always-apply bundle plus one file per distinct scope.
    let ui = fs::read_to_string(rules.join("brief-src-ui.mdc")).unwrap();
    assert!(ui.contains("globs: src/ui/**"));
    assert!(ui.contains("alwaysApply: false"));
    assert!(ui.contains("Use design tokens, not raw hex"));
    assert!(ui.contains("Prefer composition over inheritance"));

    let api = fs::read_to_string(rules.join("brief-src-api.mdc")).unwrap();
    assert!(api.contains("globs: src/api/**"));

    // The base bundle stays always-on and omits the scoped constraints.
    let base = fs::read_to_string(rules.join("brief.mdc")).unwrap();
    assert!(base.contains("alwaysApply: true"));
    assert!(base.contains("Do not break existing tests"));
    assert!(!base.contains("Use design tokens, not raw hex"));
}

// -- validation lint --

#[test]
fn cli_validate_warns_on_dead_scope_but_succeeds() {
    let dir = tempdir().unwrap();
    let brief = dir.path().join(".brief.md");
    // Scope points at a directory that does not exist in the temp project.
    fs::write(
        &brief,
        "---\nstack: [Rust]\n---\n\n# Goal\n\n## Constraints\n\n### Hard\n- [`does/not/exist/**`] Some rule about the missing area\n",
    )
    .unwrap();

    Command::cargo_bin("brief")
        .unwrap()
        .current_dir(dir.path())
        .arg("--file")
        .arg(&brief)
        .arg("validate")
        .assert()
        // A dead scope is a warning, never a hard failure.
        .success()
        .stderr(
            predicate::str::contains("does/not/exist/**")
                .and(predicate::str::contains("matches no files")),
        );
}
