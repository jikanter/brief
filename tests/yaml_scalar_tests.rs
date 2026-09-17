//! Characterization tests for YAML scalar typing in frontmatter.
//!
//! These pin the contract across the `serde_yaml` → `serde-saphyr` swap. A
//! `.brief.md` routinely carries scalars that YAML would otherwise type as
//! numbers, booleans or timestamps — `Python 3.12` in a stack, a bare `no`, a
//! date-shaped string. Every one of them must arrive as a `String`.

use brief_cli::parse::frontmatter::extract_frontmatter;

fn fm(yaml: &str) -> brief_cli::model::Frontmatter {
    let input = format!("---\n{yaml}\n---\n\n# Goal\n");
    extract_frontmatter(&input)
        .expect("frontmatter must parse")
        .0
}

#[test]
fn number_like_stack_entries_stay_strings() {
    let f = fm("stack: [3.12, 1, 1.0, 0x10, 1_000]");
    assert_eq!(f.stack, vec!["3.12", "1", "1.0", "0x10", "1_000"]);
}

#[test]
fn boolean_like_stack_entries_stay_strings() {
    // YAML 1.1 types `no`/`yes`/`on`/`off` as booleans; YAML 1.2 does not.
    // Either way they must reach the model as the author's text.
    let f = fm("stack: [no, yes, on, off, true, false]");
    assert_eq!(f.stack, vec!["no", "yes", "on", "off", "true", "false"]);
}

/// The one deliberate behavior change in the `serde_yaml` -> `serde-saphyr`
/// swap. `serde_yaml` handed a plain `null` scalar to a `Vec<String>` as the
/// literal text `"null"`; `serde-saphyr` refuses it and names the line.
///
/// Refusing is the better answer -- a null inside a string list is a malformed
/// brief, and silently inventing the string "null" hides it -- but it is an
/// error where there was none, so it is pinned here rather than left to
/// surface as a surprise. Quoting the value keeps it working.
#[test]
fn a_null_inside_a_string_list_is_now_a_parse_error() {
    for spelling in ["null", "~", "Null", "NULL"] {
        let input = format!("---\nstack: [{spelling}]\n---\n\n# Goal\n");
        let err =
            extract_frontmatter(&input).expect_err("a bare null in a Vec<String> must not parse");
        assert!(
            format!("{err:#}").contains("cannot deserialize null into string"),
            "unexpected error for `{spelling}`: {err:#}"
        );
    }
    // Quoted, it is an ordinary string.
    assert_eq!(fm("stack: ['null']").stack, vec!["null"]);
}

#[test]
fn date_like_values_stay_strings() {
    let f = fm("stack: [2026-04-11]\ncontext: [2026-04-11T14:22:00-05:00]");
    assert_eq!(f.stack, vec!["2026-04-11"]);
    assert_eq!(f.context, vec!["2026-04-11T14:22:00-05:00"]);
}

#[test]
fn quoted_and_unquoted_versions_agree() {
    assert_eq!(fm("stack: [x]\nbrief_version: \"1\"").brief_version, "1");
    assert_eq!(fm("stack: [x]\nbrief_version: 1").brief_version, "1");
}

#[test]
fn legacy_version_alias_still_deserializes() {
    assert_eq!(fm("stack: [x]\nversion: \"1\"").brief_version, "1");
}

#[test]
fn unknown_keys_are_ignored_not_errors() {
    let f = fm("stack: [Rust]\ndateModified: 2026-04-11\nstatus: draft\ntags: [a, b]");
    assert_eq!(f.stack, vec!["Rust"]);
}

#[test]
fn block_scalars_and_multiline_strings_survive() {
    let f = fm("stack:\n  - Rust\n  - >-\n    a folded\n    entry");
    assert_eq!(f.stack, vec!["Rust", "a folded entry"]);
}

#[test]
fn empty_frontmatter_yields_defaults() {
    let f = fm("");
    assert!(f.stack.is_empty());
    assert!(f.context.is_empty());
    assert_eq!(f.model, None);
    assert_eq!(f.brief_version, "1");
}
