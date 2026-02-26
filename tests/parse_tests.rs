use brief_cli::parse::parse_brief;

fn fixture(name: &str) -> String {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("Failed to read fixture {path}: {e}"))
}

#[test]
fn parse_minimal_fixture() {
    let brief = parse_brief(&fixture("minimal.brief.md")).unwrap();

    // Frontmatter
    assert_eq!(brief.frontmatter.stack, vec!["Rust"]);
    assert!(brief.frontmatter.context.is_empty());
    assert_eq!(brief.frontmatter.version, "1");

    // Goal
    assert_eq!(brief.goal, "Fix the login bug");

    // Constraints
    assert_eq!(brief.constraints.hard.len(), 1);
    assert_eq!(brief.constraints.hard[0], "Do not break existing tests");
    assert!(brief.constraints.soft.is_empty());
    assert!(brief.constraints.ask_first.is_empty());

    // Sacred
    assert_eq!(brief.sacred.len(), 1);
    assert_eq!(brief.sacred[0].path, "src/auth.rs");
    assert_eq!(
        brief.sacred[0].reason,
        "Authentication logic, do not refactor"
    );
    assert!(brief.sacred[0].well_formed);

    // No assumptions or deliverable in minimal
    assert!(brief.assumptions.is_empty());
    assert!(brief.deliverable.is_none());
}

#[test]
fn parse_full_fixture() {
    let brief = parse_brief(&fixture("full.brief.md")).unwrap();

    // Frontmatter
    assert_eq!(
        brief.frontmatter.stack,
        vec![
            "TypeScript 5.4",
            "React 18",
            "PostgreSQL 16",
            "Redis 7",
            "AWS ECS"
        ]
    );
    assert_eq!(
        brief.frontmatter.context,
        vec![
            "./docs/architecture.md",
            "./docs/api-spec.yaml",
            "./README.md"
        ]
    );
    assert_eq!(
        brief.frontmatter.model,
        Some("claude-sonnet-4-20250514".to_string())
    );
    assert_eq!(brief.frontmatter.version, "1");

    // Goal
    assert_eq!(brief.goal, "Build real-time collaborative document editor");

    // Hard constraints
    assert_eq!(brief.constraints.hard.len(), 4);
    assert!(brief.constraints.hard[0].contains("WebSocket"));
    assert!(brief.constraints.hard[1].contains("event sourcing"));
    assert!(brief.constraints.hard[2].contains("WCAG"));
    assert!(brief.constraints.hard[3].contains("E2E test suite"));

    // Soft constraints
    assert_eq!(brief.constraints.soft.len(), 3);
    assert!(brief.constraints.soft[0].contains("Yjs"));

    // Ask first constraints
    assert_eq!(brief.constraints.ask_first.len(), 4);
    assert!(brief.constraints.ask_first[0].contains("shared state schema"));

    // Sacred regions
    assert_eq!(brief.sacred.len(), 4);
    assert_eq!(brief.sacred[0].path, "src/core/crdt-engine/**");
    assert!(brief.sacred[0].reason.contains("CRDT"));
    assert!(brief.sacred[0].well_formed);

    assert_eq!(brief.sacred[1].path, "src/auth/**");
    assert!(brief.sacred[1].reason.contains("SOC2"));

    assert_eq!(brief.sacred[2].path, "migrations/**");
    assert!(brief.sacred[2].reason.contains("never be altered"));

    assert_eq!(brief.sacred[3].path, "e2e/");
    assert!(brief.sacred[3].reason.contains("adding new tests"));

    // Assumptions
    assert_eq!(brief.assumptions.len(), 4);
    assert!(!brief.assumptions[0].validated);
    assert!(brief.assumptions[0].has_checkbox);
    assert!(brief.assumptions[0].text.contains("Redis pub/sub"));

    assert!(!brief.assumptions[1].validated);
    assert!(brief.assumptions[1].text.contains("Yjs document size"));

    assert!(brief.assumptions[2].validated);
    assert!(brief.assumptions[2].text.contains("REST API"));

    assert!(!brief.assumptions[3].validated);
    assert!(brief.assumptions[3].text.contains("IndexedDB"));

    // Deliverable
    assert!(brief.deliverable.is_some());
    let deliverable = brief.deliverable.unwrap();
    assert!(deliverable.contains("collaborative editor"));
    assert!(deliverable.contains("`ENABLE_COLLAB_EDITOR`"));
}

#[test]
fn parse_malformed_fixture() {
    let brief = parse_brief(&fixture("malformed.brief.md")).unwrap();

    // Missing stack — should default to empty
    assert!(brief.frontmatter.stack.is_empty());

    // Missing H1 — goal should be empty
    assert!(brief.goal.is_empty());

    // Context file listed (even though it doesn't exist — that's validation's job)
    assert_eq!(brief.frontmatter.context, vec!["./nonexistent-file.md"]);

    // Has one hard constraint
    assert_eq!(brief.constraints.hard.len(), 1);
    assert_eq!(brief.constraints.hard[0], "A valid constraint");

    // The "Unknown Type" constraint should not end up in any known bucket
    // (it's under an unrecognized H3, so the parser skips it)
    assert!(brief.constraints.soft.is_empty());
    assert!(brief.constraints.ask_first.is_empty());

    // Sacred: one malformed, one valid
    assert_eq!(brief.sacred.len(), 2);
    assert!(!brief.sacred[0].well_formed); // no backticks
    assert!(brief.sacred[1].well_formed); // properly formatted

    // Assumptions: one without checkbox, one with
    assert_eq!(brief.assumptions.len(), 2);
    assert!(!brief.assumptions[0].has_checkbox); // missing checkbox syntax
    assert!(brief.assumptions[1].has_checkbox); // correctly formatted
}

#[test]
fn parse_sample_example() {
    let path = format!("{}/examples/sample.brief.md", env!("CARGO_MANIFEST_DIR"));
    let content = std::fs::read_to_string(&path).unwrap();
    let brief = parse_brief(&content).unwrap();

    assert_eq!(brief.goal, "Redesign event pipeline for 10M events/day");
    assert_eq!(
        brief.frontmatter.stack,
        vec!["Python 3.12", "PostgreSQL 16", "Kafka 3.7", "GCP/k8s"]
    );
    assert_eq!(brief.constraints.hard.len(), 3);
    assert_eq!(brief.constraints.soft.len(), 2);
    assert_eq!(brief.constraints.ask_first.len(), 3);
    assert_eq!(brief.sacred.len(), 3);
    assert_eq!(brief.assumptions.len(), 3);
    assert!(brief.deliverable.is_some());
}
