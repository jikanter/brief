//! `docs/bugs.md`: `brief emit aider --install` rewrote a user's
//! `.aider.conf.yml` by deserializing and re-emitting it, so comments, key
//! order and quoting style were all lost. `.aider.conf.yml` is a file brief
//! does not own; it may only splice the lines it is responsible for.

use brief_cli::emit::aider::merge_aider_conf;

fn merged(existing: &str, model: Option<&str>) -> String {
    merge_aider_conf(Some(existing), model).expect("should merge")
}

// ---------------------------------------------------------------------------
// the reported bug
// ---------------------------------------------------------------------------

#[test]
fn keeps_every_comment_in_the_users_file() {
    let existing = "# my comment\nread: other.md # keep\nauto-commits: false\n";
    let out = merged(existing, None);
    assert_eq!(
        out, "# my comment\nread:\n  - other.md # keep\n  - CONVENTIONS.md\nauto-commits: false\n",
        "got:\n{out}"
    );
}

/// Every byte outside the entry brief owns survives: key order, quoting style,
/// blank lines, indentation, and comments anywhere in the file.
#[test]
fn leaves_every_byte_it_does_not_own_alone() {
    let existing = concat!(
        "# Aider configuration\n",
        "\n",
        "zeta: 1        # trailing\n",
        "alpha: \"quoted value\"\n",
        "\n",
        "nested:\n",
        "  deep:\n",
        "    key: value\n",
        "\n",
        "# the end\n",
    );
    let out = merged(existing, None);
    assert!(out.starts_with(existing), "prefix changed:\n{out}");
    assert_eq!(out, format!("{existing}read: CONVENTIONS.md\n"));
}

// ---------------------------------------------------------------------------
// each shape of an existing `read`
// ---------------------------------------------------------------------------

#[test]
fn appends_read_when_it_is_absent() {
    assert_eq!(
        merged("auto-commits: false\n", None),
        "auto-commits: false\nread: CONVENTIONS.md\n"
    );
}

#[test]
fn promotes_a_scalar_read_to_a_block_list() {
    assert_eq!(
        merged("read: OTHER.md\n", None),
        "read:\n  - OTHER.md\n  - CONVENTIONS.md\n"
    );
}

#[test]
fn keeps_the_scalars_quoting_style_when_promoting() {
    assert_eq!(
        merged("read: \"OTHER.md\"\n", None),
        "read:\n  - \"OTHER.md\"\n  - CONVENTIONS.md\n"
    );
}

#[test]
fn appends_to_a_block_list_at_the_siblings_indentation() {
    assert_eq!(
        merged("read:\n    - a.md\n    - b.md\n", None),
        "read:\n    - a.md\n    - b.md\n    - CONVENTIONS.md\n"
    );
}

#[test]
fn inserts_into_a_flow_list_before_the_bracket() {
    assert_eq!(
        merged("read: [a.md, b.md]\n", None),
        "read: [a.md, b.md, CONVENTIONS.md]\n"
    );
    assert_eq!(merged("read: []\n", None), "read: [CONVENTIONS.md]\n");
}

#[test]
fn fills_in_a_read_key_with_no_value() {
    assert_eq!(merged("read:\n", None), "read:\n  - CONVENTIONS.md\n");
}

// ---------------------------------------------------------------------------
// already satisfied: byte-identical
// ---------------------------------------------------------------------------

#[test]
fn a_satisfied_file_comes_back_byte_identical() {
    // `read` already includes the conventions, in each shape it can take.
    for existing in [
        "read: CONVENTIONS.md\n",
        "# c\nread:\n  - CONVENTIONS.md\n  - other.md\n",
        "read: [CONVENTIONS.md]\n",
        "read: \"CONVENTIONS.md\"\n",
        "read:\n    - CONVENTIONS.md\n",
    ] {
        assert_eq!(
            merge_aider_conf(Some(existing), None).unwrap(),
            existing,
            "changed a satisfied file:\n{existing}"
        );
    }

    // Both keys satisfied: the brief's model is ignored, nothing moves.
    let both = "read: CONVENTIONS.md\nmodel: gpt-4o\n";
    assert_eq!(
        merge_aider_conf(Some(both), Some("claude-opus-5")).unwrap(),
        both
    );
}

#[test]
fn re_running_on_its_own_output_changes_nothing() {
    for existing in [
        "",
        "auto-commits: false\n",
        "# my comment\nread: other.md # keep\nauto-commits: false\n",
        "read: [a.md]\n",
        "read:\n    - a.md\n",
        "read:\n",
    ] {
        let once = merge_aider_conf(Some(existing), Some("claude-opus-5")).unwrap();
        let twice = merge_aider_conf(Some(&once), Some("claude-opus-5")).unwrap();
        assert_eq!(once, twice, "not idempotent for:\n{existing}");
    }
}

/// A file with no trailing newline keeps that shape.
#[test]
fn a_file_without_a_trailing_newline_does_not_gain_one() {
    assert_eq!(
        merged("auto-commits: false", None),
        "auto-commits: false\nread: CONVENTIONS.md"
    );
}

// ---------------------------------------------------------------------------
// model
// ---------------------------------------------------------------------------

#[test]
fn adds_the_briefs_model_only_when_the_file_has_none() {
    assert_eq!(
        merged("read: CONVENTIONS.md\n", Some("claude-opus-5")),
        "read: CONVENTIONS.md\nmodel: claude-opus-5\n"
    );
    assert_eq!(
        merged(
            "model: gpt-4o\nread: CONVENTIONS.md\n",
            Some("claude-opus-5")
        ),
        "model: gpt-4o\nread: CONVENTIONS.md\n"
    );
}

/// A `model` mentioned in a comment or nested under another key is not the
/// top-level key, and must not suppress the one the brief names.
#[test]
fn a_commented_or_nested_model_does_not_count() {
    assert_eq!(
        merged(
            "# model: gpt-4o\nread: CONVENTIONS.md\n",
            Some("claude-opus-5")
        ),
        "# model: gpt-4o\nread: CONVENTIONS.md\nmodel: claude-opus-5\n"
    );
    assert_eq!(
        merged(
            "other:\n  model: gpt-4o\nread: CONVENTIONS.md\n",
            Some("claude-opus-5")
        ),
        "other:\n  model: gpt-4o\nread: CONVENTIONS.md\nmodel: claude-opus-5\n"
    );
}

/// Insertion order, not alphabetical: a file brief creates from scratch reads
/// the way it was written.
#[test]
fn a_file_created_from_scratch_is_written_in_insertion_order() {
    assert_eq!(
        merge_aider_conf(None, Some("claude-opus-5")).unwrap(),
        "read: CONVENTIONS.md\nmodel: claude-opus-5\n"
    );
    assert_eq!(
        merge_aider_conf(None, None).unwrap(),
        "read: CONVENTIONS.md\n"
    );
}

// ---------------------------------------------------------------------------
// shapes the splicer refuses, rather than re-emitting
// ---------------------------------------------------------------------------

#[test]
fn refuses_a_shape_it_cannot_edit_safely() {
    let cases = [
        // an anchor on the read value
        ("read: &conv OTHER.md\n", "anchor"),
        // an alias as the read value
        ("base: OTHER.md\nread: *base\n", "alias"),
        // an alias inside the read list
        ("base: OTHER.md\nread:\n  - *base\n", "alias"),
        // more than one document
        ("read: a.md\n---\nread: b.md\n", "document"),
        // tabs for indentation
        ("read:\n\t- a.md\n", "tab"),
        // read nested under another key, with no top-level read
        ("session:\n  read: a.md\n", "top level"),
    ];
    for (existing, needle) in cases {
        let err = merge_aider_conf(Some(existing), None)
            .expect_err(&format!("should refuse:\n{existing}"))
            .to_string();
        assert!(
            err.to_lowercase().contains(needle),
            "error for `{existing}` should mention `{needle}`, got: {err}"
        );
    }
}

/// Refusing means writing nothing, so `install_aider` must leave the file as it
/// found it.
#[test]
fn a_refused_shape_leaves_the_file_untouched() {
    use brief_cli::emit::aider::install_aider;
    use brief_cli::parse::parse_brief;

    let dir = tempfile::TempDir::new().unwrap();
    let conf = dir.path().join(".aider.conf.yml");
    let original = "read: &conv OTHER.md\n";
    std::fs::write(&conf, original).unwrap();

    let brief = parse_brief("---\nstack: [Rust]\n---\n\n# Goal\n").unwrap();
    let err = install_aider(&brief, dir.path()).expect_err("should refuse");

    assert_eq!(std::fs::read_to_string(&conf).unwrap(), original);
    assert!(
        err.to_string().contains(".aider.conf.yml"),
        "the error should name the file, got: {err}"
    );
}
