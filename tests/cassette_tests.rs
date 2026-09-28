//! Cassette bindings: a brief names the astrophage eval-replay sets its role
//! replays, in `cassettes:` frontmatter or a `## Fixtures` section, and
//! `brief emit promptfoo` compiles them into promptfoo providers that point at
//! the astrophage gateway. Brief only parses and prints; it never runs the
//! gateway or makes a request.

use assert_cmd::prelude::*;
use brief_cli::emit::emit_promptfoo;
use brief_cli::emit::promptfoo::{
    DEFAULT_GATEWAY_BASE_URL, PromptfooOptions, emit_promptfoo_at, emit_promptfoo_with,
    gateway_base_url,
};
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

#[test]
fn parsed_brief_schema_carries_the_set_name_rule() {
    // Same rule as `brief validate`, the way `skill_name` carries its pattern.
    let schema_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/docs/schema/brief-v1.schema.json"
    );
    let schema: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(schema_path).unwrap()).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    let json_for = |set: &str| -> serde_json::Value {
        let src = format!("---\nstack: [Rust]\nmodel: m\ncassettes: [\"{set}\"]\n---\n\n# G\n");
        serde_json::from_str(&brief_cli::emit::emit_json(&parse_brief(&src).unwrap())).unwrap()
    };
    assert!(validator.is_valid(&json_for("rust-reviewer-v1.2_x")));
    for bad in ["../escape", "a/b", ".hidden", "has space"] {
        assert!(!validator.is_valid(&json_for(bad)), "{bad}");
    }
}

// -- gateway address --

#[test]
fn gateway_base_url_normalizes_what_astrophage_listens_on() {
    for (input, want) in [
        ("127.0.0.1:9090", "http://127.0.0.1:9090/v1"),
        ("http://gw.internal:8080", "http://gw.internal:8080/v1"),
        ("http://gw.internal:8080/", "http://gw.internal:8080/v1"),
        ("https://gw.example.com/v1", "https://gw.example.com/v1"),
        ("  localhost:8081  ", "http://localhost:8081/v1"),
    ] {
        assert_eq!(gateway_base_url(input).as_deref(), Ok(want), "{input}");
    }
}

#[test]
fn gateway_base_url_rejects_what_promptfoo_cannot_reach() {
    for input in ["", "http://", "ftp://gw:21", "gw host:80", "https:///v1"] {
        assert!(gateway_base_url(input).is_err(), "{input:?}");
    }
}

#[test]
fn emit_promptfoo_at_default_gateway_is_the_plain_emit() {
    let brief = parse_brief(BOUND).unwrap();
    assert_eq!(
        emit_promptfoo_at(&brief, DEFAULT_GATEWAY_BASE_URL),
        emit_promptfoo(&brief)
    );
}

#[test]
fn cli_emit_promptfoo_gateway_sets_base_url_and_listen_address() {
    let dir = tempdir().unwrap();
    let file = write_brief(dir.path(), BOUND);
    let out = brief_cmd(&file)
        .args(["emit", "promptfoo", "--gateway", "127.0.0.1:9090"])
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(
        stdout,
        EXPECTED
            .replace(
                "--cassettes-dir cassettes\n",
                "--cassettes-dir cassettes --listen 127.0.0.1:9090\n"
            )
            .replace("127.0.0.1:8080/v1", "127.0.0.1:9090/v1")
    );
}

#[test]
fn cli_emit_promptfoo_gateway_without_a_port_names_the_url() {
    let dir = tempdir().unwrap();
    let file = write_brief(dir.path(), BOUND);
    brief_cmd(&file)
        .args(["emit", "promptfoo", "--gateway", "https://gw.example.com"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "#   astrophage serve --mode cassette --cassettes-dir cassettes\n\
             # and reach it at https://gw.example.com/v1\n",
        ))
        .stdout(predicate::str::contains(
            "apiBaseUrl: \"https://gw.example.com/v1\"",
        ));
}

#[test]
fn cli_emit_promptfoo_rejects_a_bad_gateway() {
    let dir = tempdir().unwrap();
    let file = write_brief(dir.path(), BOUND);
    brief_cmd(&file)
        .args(["emit", "promptfoo", "--gateway", "ftp://gw:21"])
        .assert()
        .failure()
        .stdout("")
        .stderr(predicate::str::contains("--gateway"));
}

#[test]
fn cli_gateway_is_promptfoo_only() {
    let dir = tempdir().unwrap();
    let file = write_brief(dir.path(), BOUND);
    brief_cmd(&file)
        .args(["emit", "json", "--gateway", "127.0.0.1:9090"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "--gateway is only supported for the promptfoo target",
        ));
}

// -- session id (--session-id) --

const EXPECTED_SESSION: &str = r#"# promptfoo providers that replay astrophage cassette sets.
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
#
# Every request carries X-Eridian-Session-Id, one ID per test case, which
# astrophage copies into its trace. promptfoo mints the ID only when
# promptfooconfig.yaml has:
#   defaultTest:
#     options:
#       transformVars: "{ ...vars, sessionId: context.uuid }"
# Record and replay a set with the same provider shape: these http providers
# send a plain-text prompt as one user message.

# pinned eval-replay set for the rust-reviewer role
- id: http
  label: rust-reviewer-v1
  config:
    url: "http://127.0.0.1:8080/v1/chat/completions"
    method: POST
    headers:
      content-type: application/json
      x-aichat-cache-mode: cassette
      x-aichat-cassette: rust-reviewer-v1
      x-eridian-session-id: "{{sessionId}}"
    body:
      model: gpt-4o-mini
      messages:
        - role: user
          content: "{{prompt}}"
    transformResponse: "json.choices[0].message.content"
    maxRetries: 0

# edge cases the reviewer must flag
- id: http
  label: rust-reviewer-edge
  config:
    url: "http://127.0.0.1:8080/v1/chat/completions"
    method: POST
    headers:
      content-type: application/json
      x-aichat-cache-mode: cassette
      x-aichat-cassette: rust-reviewer-edge
      x-eridian-session-id: "{{sessionId}}"
    body:
      model: gpt-4o-mini
      messages:
        - role: user
          content: "{{prompt}}"
    transformResponse: "json.choices[0].message.content"
    maxRetries: 0
"#;

fn session_options(base_url: &str) -> PromptfooOptions {
    PromptfooOptions {
        base_url: base_url.to_string(),
        session_id: true,
    }
}

#[test]
fn emit_promptfoo_session_id_renders_http_providers() {
    let brief = parse_brief(BOUND).unwrap();
    assert_eq!(
        emit_promptfoo_with(&brief, &session_options(DEFAULT_GATEWAY_BASE_URL)),
        EXPECTED_SESSION
    );
}

#[test]
fn emit_promptfoo_session_id_output_is_the_verified_http_shape() {
    // The shape a real `promptfoo eval` was run with against astrophage: two
    // tests recorded, then replayed offline, each with its own session ID.
    let brief = parse_brief(BOUND).unwrap();
    let out = emit_promptfoo_with(&brief, &session_options(DEFAULT_GATEWAY_BASE_URL));
    let value: serde_json::Value = serde_saphyr::from_str(&out).unwrap();
    let first = &value.as_array().unwrap()[0];
    assert_eq!(first["id"], "http");
    let config = &first["config"];
    assert_eq!(config["url"], "http://127.0.0.1:8080/v1/chat/completions");
    assert_eq!(config["method"], "POST");
    assert_eq!(config["headers"]["x-eridian-session-id"], "{{sessionId}}");
    assert_eq!(config["headers"]["x-aichat-cassette"], "rust-reviewer-v1");
    assert_eq!(config["body"]["model"], "gpt-4o-mini");
    assert_eq!(config["body"]["messages"][0]["role"], "user");
    assert_eq!(config["body"]["messages"][0]["content"], "{{prompt}}");
    assert_eq!(
        config["transformResponse"],
        "json.choices[0].message.content"
    );
    // A cassette miss is deterministic: retrying it only delays the failure.
    assert_eq!(config["maxRetries"], 0);
}

#[test]
fn emit_promptfoo_without_session_id_is_unchanged() {
    let brief = parse_brief(BOUND).unwrap();
    let options = PromptfooOptions {
        base_url: DEFAULT_GATEWAY_BASE_URL.to_string(),
        session_id: false,
    };
    assert_eq!(emit_promptfoo_with(&brief, &options), EXPECTED);
}

#[test]
fn cli_emit_promptfoo_session_id_with_gateway() {
    let dir = tempdir().unwrap();
    let file = write_brief(dir.path(), BOUND);
    let out = brief_cmd(&file)
        .args([
            "emit",
            "promptfoo",
            "--session-id",
            "--gateway",
            "127.0.0.1:9090",
        ])
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        EXPECTED_SESSION
            .replace(
                "--cassettes-dir cassettes\n",
                "--cassettes-dir cassettes --listen 127.0.0.1:9090\n"
            )
            .replace("127.0.0.1:8080/v1", "127.0.0.1:9090/v1")
    );
}

#[test]
fn cli_session_id_is_promptfoo_only() {
    let dir = tempdir().unwrap();
    let file = write_brief(dir.path(), BOUND);
    brief_cmd(&file)
        .args(["emit", "json", "--session-id"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "--session-id is only supported for the promptfoo target",
        ));
}
