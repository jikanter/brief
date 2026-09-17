//! Machine schema for `.brief.md` **frontmatter**, derived from the Rust types.
//!
//! The schema is generated from [`Frontmatter`] with `schemars`, committed at
//! [`FRONTMATTER_SCHEMA_PATH`] so external tools can consume it without building
//! this crate, and embedded here via `include_str!`. `tests/frontmatter_schema_tests.rs`
//! fails if the committed copy drifts from the derived one.
//!
//! Scope: frontmatter only. Body structure (headings, constraint tiers, Sacred,
//! Assumptions) is not modeled in JSON Schema — it stays in the parser and
//! `brief validate`. The canonical JSON of a *parsed* brief is a separate,
//! hand-maintained contract: `docs/schema/brief-v1.schema.json` (see
//! `docs/schema/SPEC.md`).

use crate::model::Frontmatter;

/// Repo-relative path of the committed frontmatter schema.
pub const FRONTMATTER_SCHEMA_PATH: &str = "docs/schema/brief-frontmatter-v1.schema.json";

/// The committed frontmatter schema, embedded at build time.
pub const FRONTMATTER_SCHEMA_JSON: &str =
    include_str!("../docs/schema/brief-frontmatter-v1.schema.json");

/// Derive the frontmatter JSON Schema from the Rust types.
///
/// Pretty-printed with a trailing newline — byte-identical to the committed
/// file, which is what the drift test compares.
pub fn frontmatter_schema() -> String {
    let schema = schemars::schema_for!(Frontmatter);
    let mut out =
        serde_json::to_string_pretty(&schema).expect("schema serialization should never fail");
    out.push('\n');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derived_schema_documents_the_frontmatter_keys() {
        let schema: serde_json::Value = serde_json::from_str(&frontmatter_schema()).unwrap();
        let props = &schema["properties"];
        for key in [
            "stack",
            "context",
            "model",
            "brief_version",
            "skill_name",
            "skill_description",
        ] {
            assert!(props[key].is_object(), "missing `{key}` in derived schema");
        }
    }

    #[test]
    fn derived_schema_has_no_required_keys() {
        // Every field carries a serde default: the parser is tolerant, and
        // validity (non-empty `stack`, etc.) is `brief validate`'s job.
        let schema: serde_json::Value = serde_json::from_str(&frontmatter_schema()).unwrap();
        assert!(
            schema.get("required").is_none(),
            "frontmatter keys are all optional at parse time, got: {}",
            schema["required"]
        );
    }
}
