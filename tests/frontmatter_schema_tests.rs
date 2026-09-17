//! The frontmatter JSON Schema is *derived* from the Rust types (schemars) and
//! committed to the repo so external tools can consume it without building the
//! crate. These tests keep the committed copy honest.
//!
//! Regenerate after changing `Frontmatter`:
//!
//! ```text
//! BLESS_SCHEMA=1 cargo test --test frontmatter_schema_tests
//! ```

use brief_cli::schema::{FRONTMATTER_SCHEMA_JSON, FRONTMATTER_SCHEMA_PATH, frontmatter_schema};
use jsonschema::Validator;
use serde_json::Value;

fn repo_path(rel: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel)
}

fn validator() -> Validator {
    let schema: Value =
        serde_json::from_str(FRONTMATTER_SCHEMA_JSON).expect("embedded schema must be valid JSON");
    jsonschema::validator_for(&schema).expect("embedded schema must compile as JSON Schema")
}

/// The YAML block at the top of a `.brief.md`, as JSON — the authoring shape,
/// before the parser applies defaults.
fn frontmatter_as_json(rel: &str) -> Value {
    let raw = std::fs::read_to_string(repo_path(rel)).expect("fixture must exist");
    let body = raw
        .trim_start()
        .strip_prefix("---")
        .expect("fixture must open with `---`");
    let end = body.find("\n---").expect("fixture must close frontmatter");
    let yaml: serde_yaml::Value =
        serde_yaml::from_str(&body[..end]).expect("frontmatter must be valid YAML");
    serde_json::to_value(yaml).expect("YAML frontmatter must convert to JSON")
}

#[test]
fn committed_frontmatter_schema_matches_derived() {
    let derived = frontmatter_schema();
    let path = repo_path(FRONTMATTER_SCHEMA_PATH);

    if std::env::var_os("BLESS_SCHEMA").is_some() {
        std::fs::write(&path, &derived).expect("failed to write schema");
    }

    let committed = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));

    assert_eq!(
        committed, derived,
        "{FRONTMATTER_SCHEMA_PATH} has drifted from the Rust types. \
         Regenerate with `BLESS_SCHEMA=1 cargo test --test frontmatter_schema_tests`."
    );
    assert_eq!(
        committed, FRONTMATTER_SCHEMA_JSON,
        "the embedded schema must be the committed file"
    );
}

#[test]
fn frontmatter_schema_is_draft_2020_12() {
    let schema: Value = serde_json::from_str(FRONTMATTER_SCHEMA_JSON).unwrap();
    assert_eq!(
        schema["$schema"], "https://json-schema.org/draft/2020-12/schema",
        "frontmatter schema must declare Draft 2020-12"
    );
    assert!(
        schema["properties"]["brief_version"].is_object(),
        "brief_version must be a documented property, got: {}",
        schema["properties"]
    );
}

#[test]
fn well_formed_fixture_frontmatter_validates() {
    let validator = validator();
    for rel in [
        "tests/fixtures/minimal.brief.md",
        "tests/fixtures/full.brief.md",
        "tests/fixtures/skill.brief.md",
        "tests/fixtures/rich-unknown.brief.md",
        "examples/sample.brief.md",
        ".brief.md",
    ] {
        let instance = frontmatter_as_json(rel);
        let errors: Vec<String> = validator
            .iter_errors(&instance)
            .map(|e| format!("{}: {}", e.instance_path, e))
            .collect();
        assert!(
            errors.is_empty(),
            "{rel} frontmatter should validate, got:\n{}",
            errors.join("\n")
        );
    }
}

#[test]
fn legacy_version_key_still_validates() {
    // Unknown keys stay legal (forward-compatible authoring); `version:` is the
    // deprecated spelling of `brief_version` and must not hard-fail.
    let instance: Value = serde_json::json!({ "stack": ["Rust"], "version": "1" });
    assert!(
        validator().is_valid(&instance),
        "legacy `version:` must still validate"
    );
}

#[test]
fn non_string_brief_version_is_rejected() {
    let instance: Value = serde_json::json!({ "stack": ["Rust"], "brief_version": 1 });
    assert!(
        !validator().is_valid(&instance),
        "brief_version must be a string"
    );
}

#[test]
fn stack_must_be_a_string_array() {
    let instance: Value = serde_json::json!({ "stack": "Rust" });
    assert!(
        !validator().is_valid(&instance),
        "a bare string `stack:` must not validate"
    );
}
