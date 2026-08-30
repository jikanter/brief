use brief_cli::emit::emit_json;
use brief_cli::parse::parse_brief;
use jsonschema::Validator;
use serde_json::Value;

fn schema_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("docs/schema/brief-v1.schema.json")
}

fn load_schema() -> Validator {
    let raw = std::fs::read_to_string(schema_path())
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", schema_path().display()));
    let schema: Value = serde_json::from_str(&raw).expect("schema must be valid JSON");
    jsonschema::validator_for(&schema).expect("schema must compile as JSON Schema")
}

fn load_md(rel: &str) -> String {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

fn emit_parsed(rel: &str) -> Value {
    let brief = parse_brief(&load_md(rel)).expect("fixture must parse");
    serde_json::from_str(&emit_json(&brief)).expect("emit json must parse")
}

fn error_paths(validator: &Validator, instance: &Value) -> Vec<String> {
    validator
        .iter_errors(instance)
        .map(|e| e.instance_path.to_string())
        .collect()
}

#[test]
fn schema_file_is_draft_2020_12() {
    let raw = std::fs::read_to_string(schema_path()).expect("schema file must exist");
    let schema: Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(
        schema["$schema"],
        "https://json-schema.org/draft/2020-12/schema"
    );
}

#[test]
fn well_formed_fixtures_validate() {
    let validator = load_schema();
    for rel in [
        "tests/fixtures/minimal.brief.md",
        "tests/fixtures/full.brief.md",
        "tests/fixtures/skill.brief.md",
        "tests/fixtures/rich-unknown.brief.md",
        "examples/sample.brief.md",
        ".brief.md",
    ] {
        let instance = emit_parsed(rel);
        let errors: Vec<String> = validator
            .iter_errors(&instance)
            .map(|e| format!("{}: {}", e.instance_path, e))
            .collect();
        assert!(
            errors.is_empty(),
            "{rel} should validate against brief-v1 schema, got:\n{}",
            errors.join("\n")
        );
    }
}

#[test]
fn malformed_fixture_fails_validity_schema() {
    let validator = load_schema();
    let instance = emit_parsed("tests/fixtures/malformed.brief.md");
    let paths = error_paths(&validator, &instance);
    assert!(
        !paths.is_empty(),
        "malformed.brief.md must not validate as a valid Brief"
    );

    let joined = paths.join(" ");
    assert!(
        joined.contains("stack") || paths.iter().any(|p| p.contains("stack")),
        "missing stack should be reported, got: {paths:?}"
    );
    assert!(
        paths.iter().any(|p| p == "/goal" || p.ends_with("goal")),
        "missing H1 goal should be reported, got: {paths:?}"
    );
    assert!(
        paths
            .iter()
            .any(|p| p.contains("sacred") && p.contains("well_formed")),
        "malformed sacred entry should be reported, got: {paths:?}"
    );
    assert!(
        paths
            .iter()
            .any(|p| p.contains("assumptions") && p.contains("has_checkbox")),
        "assumption without checkbox should be reported, got: {paths:?}"
    );
}
