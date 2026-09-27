//! Cassette bindings: a brief names the astrophage eval-replay sets its role
//! replays, in `cassettes:` frontmatter or a `## Fixtures` section, and
//! `brief emit promptfoo` compiles them into promptfoo providers that point at
//! the astrophage gateway. Brief only parses and prints; it never runs the
//! gateway or makes a request.

use assert_cmd::prelude::*;
use brief_cli::emit::emit_promptfoo;
use brief_cli::fixtures::{CassetteBinding, cassette_bindings, malformed_fixture_items};
use brief_cli::model::Severity;
use brief_cli::parse::parse_brief;
use brief_cli::validate::validate;
use predicates::prelude::*;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

const BOUND: &str = "---
stack: [Rust]
model: gpt-4o-mini
cassettes: [rust-reviewer-v1]
---

# Review Rust changes

## Fixtures
- cassette: rust-reviewer-v1 — pinned eval-replay set for the rust-reviewer role
- cassette: `rust-reviewer-edge` -- edge cases the reviewer must flag
";

fn binding(name: &str, reason: Option<&str>) -> CassetteBinding {
    CassetteBinding {
        name: name.to_string(),
        reason: reason.map(str::to_string),
    }
}

// -- 48A: parsing --

#[test]
fn frontmatter_cassettes_bind_in_order() {
    let brief = parse_brief("---\nstack: [Rust]\ncassettes: [a-v1, b-v2]\n---\n\n# G\n").unwrap();
    assert_eq!(brief.frontmatter.cassettes, vec!["a-v1", "b-v2"]);
    assert_eq!(
        cassette_bindings(&brief),
        vec![binding("a-v1", None), binding("b-v2", None)]
    );
}

#[test]
fn fixtures_section_binds_with_reasons() {
    let brief = parse_brief(
        "---\nstack: [Rust]\n---\n\n# G\n\n## Fixtures\n\
         - cassette: one-v1 — em dash reason\n\
         - cassette: `two-v1` -- double hyphen reason\n\
         - cassette: three-v1\n",
    )
    .unwrap();
    assert_eq!(
        cassette_bindings(&brief),
        vec![
            binding("one-v1", Some("em dash reason")),
            binding("two-v1", Some("double hyphen reason")),
            binding("three-v1", None),
        ]
    );
}

#[test]
fn frontmatter_and_section_merge_without_duplicates() {
    let brief = parse_brief(BOUND).unwrap();
    assert_eq!(
        cassette_bindings(&brief),
        vec![
            // Named in both places: one binding, and the section's reason.
            binding(
                "rust-reviewer-v1",
                Some("pinned eval-replay set for the rust-reviewer role")
            ),
            binding(
                "rust-reviewer-edge",
                Some("edge cases the reviewer must flag")
            ),
        ]
    );
}

#[test]
fn fixtures_section_still_passes_through_as_unknown_section() {
    // Every other emitter keeps rendering the section exactly as before.
    let brief = parse_brief(BOUND).unwrap();
    let section = brief
        .unknown_sections
        .iter()
        .find(|s| s.heading == "Fixtures")
        .expect("## Fixtures stays an unknown section");
    assert!(section.content.contains("cassette: rust-reviewer-v1"));
}

#[test]
fn fixtures_items_without_the_cassette_prefix_are_malformed() {
    let brief = parse_brief(
        "---\nstack: [Rust]\n---\n\n# G\n\n## Fixtures\n\
         - cassette: good-v1\n\
         - rust-reviewer-v1 — no prefix\n",
    )
    .unwrap();
    assert_eq!(cassette_bindings(&brief), vec![binding("good-v1", None)]);
    assert_eq!(
        malformed_fixture_items(&brief),
        vec!["rust-reviewer-v1 — no prefix"]
    );
}

#[test]
fn no_binding_anywhere_is_empty() {
    let brief = parse_brief("---\nstack: [Rust]\n---\n\n# G\n").unwrap();
    assert!(cassette_bindings(&brief).is_empty());
}

// -- 48A: validation --

fn diagnostics(src: &str) -> Vec<(Severity, String)> {
    let brief = parse_brief(src).unwrap();
    let dir = tempdir().unwrap();
    validate(&brief, dir.path())
        .into_iter()
        .map(|d| (d.severity, d.message))
        .collect()
}

#[test]
fn validate_warns_on_malformed_fixture_item() {
    let diags = diagnostics(
        "---\nstack: [Rust]\nmodel: m\n---\n\n# G\n\n## Fixtures\n- rust-reviewer-v1\n",
    );
    assert!(
        diags.iter().any(|(s, m)| *s == Severity::Warning
            && m.contains("Malformed fixture entry")
            && m.contains("rust-reviewer-v1")),
        "{diags:?}"
    );
}

#[test]
fn validate_rejects_a_set_name_that_is_not_a_plain_directory_name() {
    for bad in ["../escape", "has space", "a/b", ".hidden"] {
        let src = format!("---\nstack: [Rust]\nmodel: m\ncassettes: [\"{bad}\"]\n---\n\n# G\n");
        let diags = diagnostics(&src);
        assert!(
            diags.iter().any(|(s, m)| *s == Severity::Error
                && m.contains("Invalid cassette set name")
                && m.contains(bad)),
            "{bad}: {diags:?}"
        );
    }
}

#[test]
fn validate_warns_when_cassettes_have_no_model() {
    let diags = diagnostics("---\nstack: [Rust]\ncassettes: [a-v1]\n---\n\n# G\n");
    assert!(
        diags
            .iter()
            .any(|(s, m)| *s == Severity::Warning && m.contains("`model`")),
        "{diags:?}"
    );
}

#[test]
fn validate_is_quiet_for_a_well_formed_binding() {
    let diags = diagnostics(BOUND);
    assert!(
        !diags
            .iter()
            .any(|(_, m)| m.contains("cassette") || m.contains("fixture")),
        "{diags:?}"
    );
}

// -- 48B: emit --

const EXPECTED: &str = r#"# promptfoo providers that replay astrophage cassette sets.
# Generated by `brief emit promptfoo`: edit the brief, not this file.
#
# Save it and load it from promptfooconfig.yaml:
#   brief emit promptfoo > brief-providers.yaml
#   providers:
#     - file://brief-providers.yaml
#
# Start the gateway before `promptfoo eval`. It replays offline and fails any
# request the cassette does not hold, so no call reaches a live provider:
#   astrophage serve --mode cassette --cassettes-dir cassettes

# pinned eval-replay set for the rust-reviewer role
- id: "openai:chat:gpt-4o-mini"
  label: rust-reviewer-v1
  config:
    apiBaseUrl: "http://127.0.0.1:8080/v1"
    apiKeyRequired: false
    headers:
      x-aichat-cache-mode: cassette
      x-aichat-cassette: rust-reviewer-v1

# edge cases the reviewer must flag
- id: "openai:chat:gpt-4o-mini"
  label: rust-reviewer-edge
  config:
    apiBaseUrl: "http://127.0.0.1:8080/v1"
    apiKeyRequired: false
    headers:
      x-aichat-cache-mode: cassette
      x-aichat-cassette: rust-reviewer-edge
"#;

#[test]
fn emit_promptfoo_renders_one_provider_per_set() {
    let brief = parse_brief(BOUND).unwrap();
    assert_eq!(emit_promptfoo(&brief), EXPECTED);
}

#[test]
fn emit_promptfoo_output_is_a_yaml_list_of_providers() {
    let brief = parse_brief(BOUND).unwrap();
    let value: serde_json::Value = serde_saphyr::from_str(&emit_promptfoo(&brief)).unwrap();
    let providers = value.as_array().expect("a bare list, loadable via file://");
    assert_eq!(providers.len(), 2);
    let first = &providers[0];
    assert_eq!(first["id"], "openai:chat:gpt-4o-mini");
    assert_eq!(first["config"]["apiBaseUrl"], "http://127.0.0.1:8080/v1");
    assert_eq!(first["config"]["apiKeyRequired"], false);
    assert_eq!(
        first["config"]["headers"]["x-aichat-cache-mode"],
        "cassette"
    );
    assert_eq!(
        first["config"]["headers"]["x-aichat-cassette"],
        "rust-reviewer-v1"
    );
}

#[test]
fn emit_promptfoo_is_format_only() {
    // Acceptance criterion 3: the seam adds no runtime or network code.
    let manifest = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml")).unwrap();
    for crate_name in ["tokio", "reqwest", "hyper", "ureq"] {
        assert!(
            !manifest.contains(crate_name),
            "brief must not depend on {crate_name}"
        );
    }
}

fn write_brief(dir: &std::path::Path, src: &str) -> std::path::PathBuf {
    let path = dir.join(".brief.md");
    fs::write(&path, src).unwrap();
    path
}

fn brief_cmd(file: &std::path::Path) -> Command {
    let mut cmd = Command::cargo_bin("brief").unwrap();
    cmd.arg("--file").arg(file);
    cmd
}

#[test]
fn cli_emit_promptfoo_prints_the_providers() {
    let dir = tempdir().unwrap();
    let file = write_brief(dir.path(), BOUND);
    brief_cmd(&file)
        .args(["emit", "promptfoo"])
        .assert()
        .success()
        .stdout(EXPECTED);
}

#[test]
fn cli_emit_promptfoo_fails_without_a_binding() {
    let dir = tempdir().unwrap();
    let file = write_brief(dir.path(), "---\nstack: [Rust]\nmodel: m\n---\n\n# G\n");
    brief_cmd(&file)
        .args(["emit", "promptfoo"])
        .assert()
        .failure()
        .stdout("")
        .stderr(predicate::str::contains("no cassette binding"));
}

#[test]
fn cli_emit_promptfoo_fails_without_a_model() {
    let dir = tempdir().unwrap();
    let file = write_brief(
        dir.path(),
        "---\nstack: [Rust]\ncassettes: [a-v1]\n---\n\n# G\n",
    );
    brief_cmd(&file)
        .args(["emit", "promptfoo"])
        .assert()
        .failure()
        .stdout("")
        .stderr(predicate::str::contains("`model`"));
}

#[test]
fn cli_emit_promptfoo_fails_on_an_invalid_set_name() {
    let dir = tempdir().unwrap();
    let file = write_brief(
        dir.path(),
        "---\nstack: [Rust]\nmodel: m\ncassettes: [\"../escape\"]\n---\n\n# G\n",
    );
    brief_cmd(&file)
        .args(["emit", "promptfoo"])
        .assert()
        .failure()
        .stdout("")
        .stderr(predicate::str::contains("Invalid cassette set name"));
}

#[test]
fn cli_emit_promptfoo_rejects_install_and_compact() {
    let dir = tempdir().unwrap();
    let file = write_brief(dir.path(), BOUND);
    brief_cmd(&file)
        .args(["emit", "promptfoo", "--install"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--install"));
    // --compact drops frontmatter and unknown sections, which is where the
    // bindings live; refusing beats emitting an empty provider list.
    brief_cmd(&file)
        .args(["emit", "promptfoo", "--compact"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--compact"));
}

#[test]
fn json_emit_carries_frontmatter_cassettes_only_when_set() {
    let dir = tempdir().unwrap();
    let file = write_brief(dir.path(), BOUND);
    let out = brief_cmd(&file).args(["emit", "json"]).output().unwrap();
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["frontmatter"]["cassettes"][0], "rust-reviewer-v1");

    let plain = write_brief(dir.path(), "---\nstack: [Rust]\n---\n\n# G\n");
    let out = brief_cmd(&plain).args(["emit", "json"]).output().unwrap();
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(value["frontmatter"].get("cassettes").is_none());
}

#[test]
fn json_emit_of_a_bound_brief_matches_the_parsed_brief_schema() {
    let schema_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/docs/schema/brief-v1.schema.json"
    );
    let schema: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(schema_path).unwrap()).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    let instance: serde_json::Value =
        serde_json::from_str(&brief_cli::emit::emit_json(&parse_brief(BOUND).unwrap())).unwrap();
    let errors: Vec<String> = validator
        .iter_errors(&instance)
        .map(|e| format!("{}: {e}", e.instance_path))
        .collect();
    assert!(errors.is_empty(), "{errors:?}");
}
