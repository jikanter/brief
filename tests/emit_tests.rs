use brief_cli::emit;
use brief_cli::parse::parse_brief;
use std::path::Path;

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
    assert!(output.contains("**IMPORTANT:** WebSocket"));
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
    // Context files as @ references
    assert!(output.contains("@docs/architecture.md"));
    assert!(output.contains("@docs/api-spec.yaml"));
    // Unknown sections emitted
    assert!(output.contains("## Commands"));
    assert!(output.contains("- Build: `npm run build`"));
    assert!(output.contains("## Code Style"));
    assert!(output.contains("- Use TypeScript strict mode"));
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
    // Unknown sections emitted with uppercase labels
    assert!(output.contains("COMMANDS:"));
    assert!(output.contains("CODE STYLE:"));
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
    // Unknown sections emitted
    assert!(output.contains("## Commands"));
    assert!(output.contains("## Code Style"));
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
    // Unknown sections preserved in JSON
    let unknown = value["unknown_sections"].as_array().unwrap();
    assert_eq!(unknown.len(), 2);
    assert_eq!(unknown[0]["heading"], "Commands");
    assert_eq!(unknown[1]["heading"], "Code Style");
}

// -- Skill emitter --

#[test]
fn emit_skill_from_skill_fixture() {
    let brief = parse_brief(&fixture("skill.brief.md")).unwrap();
    let output = emit::emit_skill(&brief, None);

    // Frontmatter
    assert!(output.starts_with("---\n"));
    assert!(output.contains("name: review\n"));
    assert!(output.contains("description: Review code changes following team standards\n"));

    // Stack woven into opening
    assert!(output.contains("Python 3.12, PostgreSQL 16"));

    // Context
    assert!(output.contains("`./docs/api-spec.yaml`"));

    // Rules
    assert!(output.contains("## Rules"));
    assert!(output.contains("You MUST"));
    assert!(output.contains("All SQL must target PostgreSQL 16"));

    // Preferences
    assert!(output.contains("Prefer async patterns"));

    // Ask first
    assert!(output.contains("Ask the user before"));
    assert!(output.contains("Database schema changes"));

    // Protected regions
    assert!(output.contains("## Protected regions"));
    assert!(output.contains("`src/auth/**`"));

    // Verification (only unvalidated)
    assert!(output.contains("## Verify before proceeding"));
    assert!(output.contains("Current tests cover critical paths"));
    assert!(!output.contains("CI pipeline runs on every PR"));

    // Deliverable
    assert!(output.contains("## Expected output"));
    assert!(output.contains("Clear review comments"));
}

#[test]
fn emit_skill_from_full_fixture_derives_name() {
    let brief = parse_brief(&fixture("full.brief.md")).unwrap();
    let output = emit::emit_skill(&brief, None);

    // Should slugify the goal since no skill_name in frontmatter
    assert!(output.contains("name: build-real-time-collaborative-document-editor\n"));
    assert!(output.contains("description: Build real-time collaborative document editor\n"));
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

// -- install_claude tests with CLAUDE.md fixtures --

/// Copy a fixture CLAUDE.md into a temp directory and return the path to it.
fn setup_claude_md(fixture_name: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let src = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(fixture_name);
    let dest = dir.path().join("CLAUDE.md");
    std::fs::copy(&src, &dest).unwrap();
    (dir, dest)
}

#[test]
fn install_claude_into_one_empty_tag() {
    let (_dir, claude_md) = setup_claude_md("claude-one-empty-tag.md");
    let brief = parse_brief(&fixture("minimal.brief.md")).unwrap();

    emit::install_claude(&brief, &claude_md).unwrap();

    let result = std::fs::read_to_string(&claude_md).unwrap();
    // Briefing inserted inside the markers
    assert!(result.contains("# Briefing: Fix the login bug"));
    assert!(result.contains("`src/auth.rs`"));
    // Surrounding content preserved
    assert!(result.contains("# My Project"));
    assert!(result.contains("Some project documentation."));
    assert!(result.contains("## Other Section"));
    assert!(result.contains("More content here."));
    // Exactly one marker pair, in the new format.
    assert_eq!(result.matches("<brief:generated>").count(), 1);
    assert_eq!(result.matches("</brief:generated>").count(), 1);
}

#[test]
fn install_claude_into_two_empty_tags() {
    let (_dir, claude_md) = setup_claude_md("claude-two-empty-tags.md");
    let brief = parse_brief(&fixture("minimal.brief.md")).unwrap();

    emit::install_claude(&brief, &claude_md).unwrap();

    let result = std::fs::read_to_string(&claude_md).unwrap();
    // Briefing inserted into the first marker pair
    assert!(result.contains("# Briefing: Fix the login bug"));
    // Second empty pair stripped
    assert_eq!(result.matches("<brief:generated>").count(), 1);
    assert_eq!(result.matches("</brief:generated>").count(), 1);
    // All surrounding content preserved
    assert!(result.contains("# My Project"));
    assert!(result.contains("## Middle Section"));
    assert!(result.contains("Some middle content."));
    assert!(result.contains("## Final Section"));
    assert!(result.contains("End content."));
}

#[test]
fn install_claude_into_full_tag_replaces_content() {
    let (_dir, claude_md) = setup_claude_md("claude-full-tag.md");
    let brief = parse_brief(&fixture("minimal.brief.md")).unwrap();

    emit::install_claude(&brief, &claude_md).unwrap();

    let result = std::fs::read_to_string(&claude_md).unwrap();
    // New briefing replaces the old content
    assert!(result.contains("# Briefing: Fix the login bug"));
    assert!(result.contains("`src/auth.rs`"));
    // Old briefing content gone
    assert!(!result.contains("Old task"));
    assert!(!result.contains("Python 3.11"));
    assert!(!result.contains("Do not modify database schema"));
    // Surrounding content preserved
    assert!(result.contains("# My Project"));
    assert!(result.contains("Some project documentation."));
    assert!(result.contains("## Other Section"));
    assert!(result.contains("More content here."));
    // Exactly one marker pair, in the new format.
    assert_eq!(result.matches("<brief:generated>").count(), 1);
    assert_eq!(result.matches("</brief:generated>").count(), 1);
}

#[test]
fn install_claude_migrates_legacy_html_comment_markers() {
    let (_dir, claude_md) = setup_claude_md("claude-legacy-html-comment-tag.md");
    let brief = parse_brief(&fixture("minimal.brief.md")).unwrap();

    emit::install_claude(&brief, &claude_md).unwrap();

    let result = std::fs::read_to_string(&claude_md).unwrap();
    // Legacy HTML-comment markers are gone; new tag markers took their place.
    assert!(!result.contains("<!-- brief:start -->"));
    assert!(!result.contains("<!-- brief:end -->"));
    assert_eq!(result.matches("<brief:generated>").count(), 1);
    assert_eq!(result.matches("</brief:generated>").count(), 1);
    // New briefing present, old briefing content gone.
    assert!(result.contains("# Briefing: Fix the login bug"));
    assert!(!result.contains("Old legacy task"));
    // Surrounding content preserved.
    assert!(result.contains("# My Project"));
    assert!(result.contains("## Other Section"));
}
