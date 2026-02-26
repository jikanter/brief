use brief_cli::model::Severity;
use brief_cli::parse::parse_brief;
use brief_cli::validate::validate;

fn fixture(name: &str) -> String {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap()
}

#[test]
fn minimal_fixture_is_valid() {
    let brief = parse_brief(&fixture("minimal.brief.md")).unwrap();
    let base_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let diags = validate(&brief, base_dir);
    let errors: Vec<_> = diags
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(
        errors.is_empty(),
        "Minimal fixture should have no errors, got: {errors:?}"
    );
}

#[test]
fn full_fixture_has_no_errors() {
    let brief = parse_brief(&fixture("full.brief.md")).unwrap();
    let base_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let diags = validate(&brief, base_dir);
    let errors: Vec<_> = diags
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(
        errors.is_empty(),
        "Full fixture should have no errors, got: {errors:?}"
    );
}

#[test]
fn full_fixture_warns_about_missing_context_files() {
    let brief = parse_brief(&fixture("full.brief.md")).unwrap();
    let base_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let diags = validate(&brief, base_dir);
    let warnings: Vec<_> = diags
        .iter()
        .filter(|d| d.severity == Severity::Warning)
        .collect();
    // Context files like ./docs/architecture.md don't exist in the test dir
    assert!(
        warnings.iter().any(|d| d.message.contains("Context file")),
        "Should warn about missing context files"
    );
}

#[test]
fn malformed_fixture_has_errors() {
    let brief = parse_brief(&fixture("malformed.brief.md")).unwrap();
    let base_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let diags = validate(&brief, base_dir);

    let errors: Vec<_> = diags
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();

    // Should have errors for: missing stack, missing goal, malformed sacred, missing checkbox
    assert!(
        errors.len() >= 3,
        "Malformed fixture should have at least 3 errors, got {}: {errors:?}",
        errors.len()
    );

    assert!(
        errors.iter().any(|d| d.message.contains("stack")),
        "Should flag missing stack"
    );
    assert!(
        errors.iter().any(|d| d.message.contains("H1 goal")),
        "Should flag missing goal"
    );
    assert!(
        errors.iter().any(|d| d.message.contains("backticks")),
        "Should flag malformed sacred entry"
    );
    assert!(
        errors.iter().any(|d| d.message.contains("checkbox")),
        "Should flag missing checkbox"
    );
}
