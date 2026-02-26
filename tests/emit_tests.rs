use brief_cli::emit;
use brief_cli::parse::parse_brief;

fn fixture(name: &str) -> String {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap()
}

// -- Claude emitter --

#[test]
fn emit_claude_from_full_fixture() {
    let brief = parse_brief(&fixture("full.brief.md")).unwrap();
    let output = emit::emit_claude(&brief);

    assert!(output.contains("# Briefing: Build real-time collaborative document editor"));
    assert!(output.contains("TypeScript 5.4"));
    assert!(output.contains("Non-negotiable"));
    assert!(output.contains("WebSocket"));
    assert!(output.contains("Preferred"));
    assert!(output.contains("Yjs"));
    assert!(output.contains("Requires approval"));
    assert!(output.contains("shared state schema"));
    assert!(output.contains("Sacred Regions"));
    assert!(output.contains("`src/core/crdt-engine/**`"));
    assert!(output.contains("Assumptions"));
    assert!(output.contains("[ ] Redis pub/sub"));
    assert!(output.contains("[x] Existing REST API"));
    assert!(output.contains("Deliverable"));
}

#[test]
fn emit_claude_from_minimal_fixture() {
    let brief = parse_brief(&fixture("minimal.brief.md")).unwrap();
    let output = emit::emit_claude(&brief);

    assert!(output.contains("Fix the login bug"));
    assert!(output.contains("Do not break existing tests"));
    assert!(output.contains("`src/auth.rs`"));
}

// -- Prompt emitter --

#[test]
fn emit_prompt_from_full_fixture() {
    let brief = parse_brief(&fixture("full.brief.md")).unwrap();
    let output = emit::emit_prompt(&brief);

    assert!(output.starts_with("GOAL:"));
    assert!(output.contains("STACK: TypeScript 5.4"));
    assert!(output.contains("HARD CONSTRAINTS:"));
    assert!(output.contains("SOFT CONSTRAINTS:"));
    assert!(output.contains("ASK BEFORE PROCEEDING:"));
    assert!(output.contains("DO NOT MODIFY:"));
    assert!(output.contains("ASSUMPTIONS (UNVALIDATED):"));
    assert!(output.contains("ASSUMPTIONS (VALIDATED):"));
    assert!(output.contains("DELIVERABLE:"));
}

// -- AGENTS.md emitter --

#[test]
fn emit_agents_md_from_full_fixture() {
    let brief = parse_brief(&fixture("full.brief.md")).unwrap();
    let output = emit::emit_agents_md(&brief);

    assert!(output.starts_with("# Build real-time"));
    assert!(output.contains("## Instructions"));
    assert!(output.contains("**(REQUIRED)**"));
    assert!(output.contains("*(preferred)*"));
    assert!(output.contains("**(ASK FIRST)**"));
    assert!(output.contains("## Protected Files"));
    assert!(output.contains("`src/core/crdt-engine/**`"));
}

// -- JSON emitter --

#[test]
fn emit_json_from_full_fixture_is_valid() {
    let brief = parse_brief(&fixture("full.brief.md")).unwrap();
    let json_str = emit::emit_json(&brief);
    let value: serde_json::Value = serde_json::from_str(&json_str).unwrap();

    assert_eq!(
        value["goal"],
        "Build real-time collaborative document editor"
    );
    assert_eq!(value["frontmatter"]["stack"][0], "TypeScript 5.4");
    assert_eq!(value["frontmatter"]["stack"].as_array().unwrap().len(), 5);
    assert_eq!(value["constraints"]["hard"].as_array().unwrap().len(), 4);
    assert_eq!(value["constraints"]["soft"].as_array().unwrap().len(), 3);
    assert_eq!(
        value["constraints"]["ask_first"].as_array().unwrap().len(),
        4
    );
    assert_eq!(value["sacred"].as_array().unwrap().len(), 4);
    assert_eq!(value["assumptions"].as_array().unwrap().len(), 4);
    assert!(value["deliverable"].is_string());
}

// -- Round-trip test --

#[test]
fn round_trip_json_preserves_structure() {
    let brief = parse_brief(&fixture("full.brief.md")).unwrap();
    let json_str = emit::emit_json(&brief);
    let value: serde_json::Value = serde_json::from_str(&json_str).unwrap();

    // Verify all top-level keys exist
    assert!(value.get("frontmatter").is_some());
    assert!(value.get("goal").is_some());
    assert!(value.get("constraints").is_some());
    assert!(value.get("sacred").is_some());
    assert!(value.get("assumptions").is_some());
    assert!(value.get("deliverable").is_some());

    // Verify nested constraint structure
    let constraints = &value["constraints"];
    assert!(constraints.get("hard").is_some());
    assert!(constraints.get("soft").is_some());
    assert!(constraints.get("ask_first").is_some());

    // Verify sacred entry structure
    let first_sacred = &value["sacred"][0];
    assert!(first_sacred.get("path").is_some());
    assert!(first_sacred.get("reason").is_some());
    assert!(first_sacred.get("well_formed").is_some());
}
