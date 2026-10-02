//! `brief.id`: a content hash of the brief's objective (goal, deliverable,
//! constraints), so a consumer can join a brief to the work it produced.
//! aichat records the id on a session and exports it with a trajectory; the id
//! must therefore be reproducible outside this crate, from the bytes alone.
//! The contract is docs/schema/SPEC.md §7.

use assert_cmd::prelude::*;
use brief_cli::emit::emit_json;
use brief_cli::id::{brief_id, canonical_json, id_of_objective, objective, objective_json};
use brief_cli::parse::parse_brief;
use serde_json::{Value, json};
use std::fs;
use std::process::Command;
use tempfile::tempdir;

const MINIMAL: &str = "---\nstack: [Rust]\n---\n\n# G\n";
/// sha256 of [`MINIMAL_OBJECTIVE`], computed with `shasum -a 256`.
const MINIMAL_ID: &str = "sha256:519052810012bfa20a99c6495e9cdbeed9ad8187f06b5e985af38f685d1f98f7";
const MINIMAL_OBJECTIVE: &str =
    r#"{"constraints":{"ask_first":[],"hard":[],"soft":[]},"deliverable":null,"goal":"G"}"#;

/// sha256 of tests/fixtures/golden/brief-id.objective.json, computed with
/// `shasum -a 256`. aichat mirrors this pair.
const GOLDEN_ID: &str = "sha256:2b939f8f5f0c3337e269db05822810adb2402b3d2405c43f161f9983f294851a";

const FULL: &str = "---
stack: [Rust]
---

# Review Rust changes

## Constraints

### Hard
- Never touch generated code
- [`src/ui/**`] WCAG 2.1 AA

### Soft
- Prefer small diffs

### Ask First
- Adding a dependency

## Deliverable
A review comment per finding.
";

fn fixture(rel: &str) -> String {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

fn id_of(src: &str) -> String {
    brief_id(&parse_brief(src).unwrap())
}

// -- the id --

#[test]
fn id_is_sha256_then_64_lowercase_hex() {
    let id = id_of(FULL);
    let hex = id.strip_prefix("sha256:").expect("sha256: prefix");
    assert_eq!(hex.len(), 64, "{id}");
    assert!(
        hex.chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)),
        "{id}"
    );
}

#[test]
fn golden_fixture_pins_the_objective_bytes_and_the_id() {
    let brief = parse_brief(&fixture("tests/fixtures/brief-id.brief.md")).unwrap();
    assert_eq!(
        objective_json(&brief),
        fixture("tests/fixtures/golden/brief-id.objective.json")
    );
    assert_eq!(brief_id(&brief), GOLDEN_ID);
}

#[test]
fn same_brief_gives_the_same_id_on_every_run() {
    assert_eq!(id_of(FULL), id_of(FULL));
}

// -- missing fields: the objective is the three fields as `emit json` prints them --

#[test]
fn a_bare_brief_still_states_all_three_fields() {
    // No `## Deliverable` is `null`; no `## Constraints` is three empty tiers.
    let brief = parse_brief(MINIMAL).unwrap();
    assert_eq!(objective_json(&brief), MINIMAL_OBJECTIVE);
    assert_eq!(brief_id(&brief), MINIMAL_ID);
}

#[test]
fn a_missing_h1_hashes_as_the_empty_goal() {
    // The parser stores "" for a missing H1 (`brief validate` reports it), and
    // the id follows the parsed brief rather than inventing a second spelling.
    let brief = parse_brief("---\nstack: [Rust]\n---\n\nNo heading here.\n").unwrap();
    assert_eq!(
        objective_json(&brief),
        r#"{"constraints":{"ask_first":[],"hard":[],"soft":[]},"deliverable":null,"goal":""}"#
    );
    assert_eq!(
        brief_id(&brief),
        "sha256:fd2968d7e1bf4c58a615038af8bce4d4baea98812f521997133f9f3987a3fdfa"
    );
}

// -- canonical form --

#[test]
fn canonical_json_sorts_keys_at_every_depth_and_drops_whitespace() {
    let value: Value = serde_json::from_str(
        r#"{ "goal": "G", "constraints": { "soft": [], "hard": [ { "text": "T", "scope": ["a/**"] } ] } }"#,
    )
    .unwrap();
    assert_eq!(
        canonical_json(&value),
        r#"{"constraints":{"hard":[{"scope":["a/**"],"text":"T"}],"soft":[]},"goal":"G"}"#
    );
}

#[test]
fn canonical_json_writes_strings_as_utf8_with_minimal_escapes() {
    // Non-ASCII stays literal UTF-8; `"`, `\` and control characters are
    // escaped (short forms, else lowercase \u00xx); `/` is not.
    let value = json!({ "s": "é — \"q\" \\ / \n \t \u{1f} \u{7f}" });
    assert_eq!(
        canonical_json(&value),
        "{\"s\":\"é — \\\"q\\\" \\\\ / \\n \\t \\u001f \u{7f}\"}"
    );
}

#[test]
fn key_order_permutation_gives_the_same_id() {
    let a: Value = serde_json::from_str(
        r#"{"goal":"G","deliverable":"D","constraints":{"hard":[{"text":"T","scope":["a/**"]}],"soft":[],"ask_first":[]}}"#,
    )
    .unwrap();
    let b: Value = serde_json::from_str(
        r#"{
            "constraints": {
                "ask_first": [],
                "soft": [],
                "hard": [ { "scope": ["a/**"], "text": "T" } ]
            },
            "deliverable": "D",
            "goal": "G"
        }"#,
    )
    .unwrap();
    assert_eq!(id_of_objective(&a), id_of_objective(&b));
}

// -- stability across everything that is not the objective --

#[test]
fn section_order_and_formatting_do_not_change_the_id() {
    let reordered = "---
stack: [Rust]
---

# Review Rust changes

## Deliverable

A review comment per finding.

## Constraints

### Ask First

* Adding a dependency

### Soft

* Prefer small diffs

### Hard

* Never touch generated code
* [`src/ui/**`] WCAG 2.1 AA
";
    assert_eq!(id_of(reordered), id_of(FULL));
}

#[test]
fn non_objective_fields_do_not_change_the_id() {
    let decorated = FULL
        .replace(
            "stack: [Rust]",
            "stack: [Rust, PostgreSQL 16]\ncontext: [./README.md]\nmodel: gpt-4o-mini\ncassettes: [set-v1]",
        )
        .replace(
            "## Deliverable",
            "## Identity\nA careful reviewer.\n\n## Sacred\n- `src/auth/**` — reviewed\n\n\
             ## Assumptions\n- [x] CI is green\n\n## Notes\nAnything.\n\n## Deliverable",
        );
    assert_ne!(decorated, FULL);
    assert_eq!(id_of(&decorated), id_of(FULL));
}

// -- sensitivity to the objective --

#[test]
fn changing_a_constraint_changes_the_id() {
    let edited = FULL.replace("Prefer small diffs", "Prefer smaller diffs");
    assert_ne!(id_of(&edited), id_of(FULL));
}

#[test]
fn changing_the_goal_or_the_deliverable_changes_the_id() {
    let goal = FULL.replace("# Review Rust changes", "# Review Go changes");
    let deliverable = FULL.replace("per finding", "per file");
    assert_ne!(id_of(&goal), id_of(FULL));
    assert_ne!(id_of(&deliverable), id_of(FULL));
    assert_ne!(id_of(&goal), id_of(&deliverable));
}

#[test]
fn a_constraints_tier_scope_and_position_are_part_of_the_objective() {
    let retiered = FULL.replace(
        "### Soft\n- Prefer small diffs\n\n### Ask First\n",
        "### Soft\n\n### Ask First\n- Prefer small diffs\n",
    );
    let rescoped = FULL.replace("[`src/ui/**`] ", "[`src/web/**`] ");
    let unscoped = FULL.replace("[`src/ui/**`] ", "");
    let swapped = FULL.replace(
        "- Never touch generated code\n- [`src/ui/**`] WCAG 2.1 AA\n",
        "- [`src/ui/**`] WCAG 2.1 AA\n- Never touch generated code\n",
    );
    let ids = [
        id_of(FULL),
        id_of(&retiered),
        id_of(&rescoped),
        id_of(&unscoped),
        id_of(&swapped),
    ];
    for (i, a) in ids.iter().enumerate() {
        for b in &ids[i + 1..] {
            assert_ne!(a, b, "{ids:?}");
        }
    }
}

// -- emit json --

#[test]
fn emit_json_carries_the_id() {
    let brief = parse_brief(FULL).unwrap();
    let value: Value = serde_json::from_str(&emit_json(&brief)).unwrap();
    assert_eq!(value["id"], brief_id(&brief));
    // The rest of the object is unchanged.
    assert_eq!(value["goal"], "Review Rust changes");
    assert_eq!(value["frontmatter"]["stack"][0], "Rust");
}

#[test]
fn the_id_is_recomputable_from_emit_json_output_alone() {
    // What a mirror does: take the three objective fields from `emit json`,
    // write them in canonical form, hash the bytes.
    let brief = parse_brief(&fixture("tests/fixtures/brief-id.brief.md")).unwrap();
    let value: Value = serde_json::from_str(&emit_json(&brief)).unwrap();
    let picked = json!({
        "goal": value["goal"],
        "deliverable": value["deliverable"],
        "constraints": value["constraints"],
    });
    assert_eq!(picked, objective(&brief));
    assert_eq!(id_of_objective(&picked), value["id"]);
    assert_eq!(value["id"], GOLDEN_ID);
}

#[test]
fn parsed_brief_schema_requires_a_well_formed_id() {
    let schema: Value = serde_json::from_str(&fixture("docs/schema/brief-v1.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    let brief = parse_brief(&fixture("tests/fixtures/brief-id.brief.md")).unwrap();
    let emitted: Value = serde_json::from_str(&emit_json(&brief)).unwrap();
    assert!(validator.is_valid(&emitted));

    let with_id = |id: Value| {
        let mut value = emitted.clone();
        value["id"] = id;
        value
    };
    for bad in [
        json!("2b939f8f5f0c3337e269db05822810adb2402b3d2405c43f161f9983f294851a"),
        json!("sha256:2B939F8F5F0C3337E269DB05822810ADB2402B3D2405C43F161F9983F294851A"),
        json!("sha256:2b939f"),
        json!("md5:2b939f8f5f0c3337e269db05822810adb2402b3d2405c43f161f9983f294851a"),
        Value::Null,
    ] {
        assert!(!validator.is_valid(&with_id(bad.clone())), "{bad}");
    }
    let mut without = emitted.clone();
    without.as_object_mut().unwrap().remove("id");
    assert!(!validator.is_valid(&without), "id is required");
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

fn stdout_of(file: &std::path::Path, args: &[&str]) -> String {
    let out = brief_cmd(file).args(args).output().unwrap();
    assert!(out.status.success(), "{out:?}");
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn cli_emit_json_id_survives_compact() {
    // --compact drops frontmatter, assumptions and unknown sections, none of
    // which is in the objective.
    let dir = tempdir().unwrap();
    let file = write_brief(dir.path(), &fixture("tests/fixtures/brief-id.brief.md"));
    let full: Value = serde_json::from_str(&stdout_of(&file, &["emit", "json"])).unwrap();
    let compact: Value =
        serde_json::from_str(&stdout_of(&file, &["emit", "json", "--compact"])).unwrap();
    assert_eq!(full["id"], GOLDEN_ID);
    assert_eq!(compact["id"], GOLDEN_ID);
}
