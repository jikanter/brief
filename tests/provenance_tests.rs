//! Unit tests for the provenance resolver (`docs/design/provenance-schema.md`).
//!
//! The resolver is pure: frontmatter text in, resolved values out. No
//! filesystem, no git, no diagnostics about paths — those belong to
//! `brief validate`.

use brief_cli::provenance::{Reference, resolve};

fn doc(frontmatter: &str) -> String {
    format!("---\n{frontmatter}\n---\n\n# Title\n\nBody.\n")
}

// ---------------------------------------------------------------------------
// canonical keys
// ---------------------------------------------------------------------------

#[test]
fn resolves_canonical_keys() {
    let p = resolve(&doc("dateModified: 2026-04-11T14:22:00-05:00\n\
         dateCreated: 2026-02-23\n\
         isBasedOn: ./architecture.brief.md\n\
         superseded_by: ./newer.md"));
    assert_eq!(
        p.date_modified.as_deref(),
        Some("2026-04-11T14:22:00-05:00")
    );
    assert_eq!(p.date_created.as_deref(), Some("2026-02-23"));
    assert_eq!(
        p.is_based_on,
        Some(Reference::Path("./architecture.brief.md".into()))
    );
    assert_eq!(p.superseded_by, Some(Reference::Path("./newer.md".into())));
}

#[test]
fn absent_keys_resolve_to_none_without_complaint() {
    let p = resolve(&doc("stack: [Rust]"));
    assert_eq!(p.date_modified, None);
    assert_eq!(p.date_created, None);
    assert_eq!(p.is_based_on, None);
    assert_eq!(p.superseded_by, None);
    assert!(p.disagreements.is_empty());
    assert!(!p.brief_written);
}

#[test]
fn explicit_null_superseded_by_is_the_same_as_absent() {
    let p = resolve(&doc("superseded_by: null"));
    assert_eq!(p.superseded_by, None);
}

// ---------------------------------------------------------------------------
// aliases — a closed list, in the spec's resolution order
// ---------------------------------------------------------------------------

#[test]
fn registered_date_modified_aliases_all_resolve() {
    for alias in [
        "date_modified",
        "last_modified",
        "lastmod",
        "modified",
        "updated",
        "updated_at",
    ] {
        let p = resolve(&doc(&format!("{alias}: 2026-04-11")));
        assert_eq!(
            p.date_modified.as_deref(),
            Some("2026-04-11"),
            "alias `{alias}` should resolve"
        );
    }
}

#[test]
fn registered_date_created_aliases_all_resolve() {
    for alias in ["date_created", "created", "created_at"] {
        let p = resolve(&doc(&format!("{alias}: 2026-02-23")));
        assert_eq!(
            p.date_created.as_deref(),
            Some("2026-02-23"),
            "alias `{alias}` should resolve"
        );
    }
}

#[test]
fn superseded_by_accepts_its_pre_v0_1_camel_case_spelling() {
    let p = resolve(&doc("supersededBy: ./newer.md"));
    assert_eq!(p.superseded_by, Some(Reference::Path("./newer.md".into())));
}

#[test]
fn is_based_on_accepts_its_snake_case_alias() {
    let p = resolve(&doc("is_based_on: ./base.md"));
    assert_eq!(p.is_based_on, Some(Reference::Path("./base.md".into())));
}

/// The closed list is the whole list. A near-miss spelling is a foreign key,
/// conserved in silence, not fuzzily matched onto a canonical name.
#[test]
fn unregistered_spellings_are_foreign_keys() {
    for foreign in ["datemodified", "date-modified", "DateModified", "moddate"] {
        let p = resolve(&doc(&format!("{foreign}: 2026-04-11")));
        assert_eq!(
            p.date_modified, None,
            "`{foreign}` must not resolve as dateModified"
        );
    }
}

#[test]
fn canonical_wins_over_an_alias_and_the_clash_is_reported() {
    let p = resolve(&doc("dateModified: 2026-04-11\nupdated: 2026-01-01"));
    assert_eq!(p.date_modified.as_deref(), Some("2026-04-11"));
    assert_eq!(p.disagreements.len(), 1);
    assert!(p.disagreements[0].contains("dateModified"));
    assert!(p.disagreements[0].contains("updated"));
}

#[test]
fn earlier_alias_wins_over_later_alias() {
    // `modified` precedes `updated` in the spec's resolution order.
    let p = resolve(&doc("updated: 2026-01-01\nmodified: 2026-04-11"));
    assert_eq!(p.date_modified.as_deref(), Some("2026-04-11"));
}

// ---------------------------------------------------------------------------
// metadata.brief.* — the namespaced address
// ---------------------------------------------------------------------------

#[test]
fn falls_through_to_metadata_brief_when_no_top_level_key() {
    let p = resolve(&doc(
        "metadata:\n  brief.dateModified: \"2026-04-11T00:00:00Z\"\n  brief.source: ../../x.brief.md",
    ));
    assert_eq!(p.date_modified.as_deref(), Some("2026-04-11T00:00:00Z"));
    assert_eq!(
        p.is_based_on,
        Some(Reference::Path("../../x.brief.md".into()))
    );
}

#[test]
fn top_level_beats_metadata_brief_and_the_clash_is_reported() {
    let p = resolve(&doc(
        "dateModified: 2026-04-11\nmetadata:\n  brief.dateModified: \"2026-01-01\"",
    ));
    assert_eq!(p.date_modified.as_deref(), Some("2026-04-11"));
    assert_eq!(p.disagreements.len(), 1);
}

#[test]
fn any_metadata_brief_key_marks_the_file_brief_written() {
    assert!(resolve(&doc("metadata:\n  brief.source: ./x.brief.md")).brief_written);
    assert!(resolve(&doc("metadata:\n  brief.dateModified: \"2026-04-11\"")).brief_written);
    assert!(!resolve(&doc("metadata:\n  author: someone")).brief_written);
    assert!(!resolve(&doc("dateModified: 2026-04-11")).brief_written);
}

/// Aliases are a top-level courtesy. Inside `metadata` a key is byte-exact or
/// it is somebody else's.
#[test]
fn aliases_never_apply_inside_metadata() {
    let p = resolve(&doc("metadata:\n  brief.updated: \"2026-04-11\""));
    assert_eq!(p.date_modified, None);
}

// ---------------------------------------------------------------------------
// paths and URLs
// ---------------------------------------------------------------------------

#[test]
fn a_scheme_prefixed_reference_is_a_url() {
    let p = resolve(&doc(
        "isBasedOn: https://example.com/doc\nsuperseded_by: ace://prj/00000123",
    ));
    assert_eq!(
        p.is_based_on,
        Some(Reference::Url("https://example.com/doc".into()))
    );
    assert_eq!(
        p.superseded_by,
        Some(Reference::Url("ace://prj/00000123".into()))
    );
}

#[test]
fn a_windows_drive_letter_is_a_path_not_a_url() {
    let p = resolve(&doc("isBasedOn: ./C:/notes.md"));
    assert!(matches!(p.is_based_on, Some(Reference::Path(_))));
}

// ---------------------------------------------------------------------------
// calendar days
// ---------------------------------------------------------------------------

#[test]
fn calendar_day_is_the_leading_date_of_any_timestamp() {
    let cases = [
        ("2026-04-11", "2026-04-11"),
        ("2026-04-11T14:22:00-05:00", "2026-04-11"),
        ("2026-04-11T23:59:59Z", "2026-04-11"),
        ("2026-04-11 14:22:00", "2026-04-11"),
    ];
    for (input, want) in cases {
        let p = resolve(&doc(&format!("dateModified: \"{input}\"")));
        assert_eq!(p.modified_day().as_deref(), Some(want), "for `{input}`");
    }
}

/// An offset is never applied. A day is the date the author wrote, so a laptop
/// and a UTC runner read the same file the same way.
#[test]
fn calendar_day_ignores_the_offset() {
    let p = resolve(&doc("dateModified: \"2026-04-11T23:00:00-05:00\""));
    assert_eq!(p.modified_day().as_deref(), Some("2026-04-11"));
}

#[test]
fn an_unparseable_date_resolves_to_no_day() {
    for junk in ["yesterday", "11/04/2026", "2026-4-1", ""] {
        let p = resolve(&doc(&format!("dateModified: \"{junk}\"")));
        assert_eq!(p.modified_day(), None, "for `{junk}`");
    }
}

#[test]
fn a_date_only_value_is_flagged_for_brief_written_files_only() {
    let human = resolve(&doc("dateModified: 2026-04-11"));
    assert!(!human.modified_needs_offset);

    let ours = resolve(&doc(
        "metadata:\n  brief.dateModified: \"2026-04-11\"\n  brief.source: ./x.brief.md",
    ));
    assert!(ours.modified_needs_offset);
}

#[test]
fn a_full_offset_timestamp_is_never_flagged() {
    let ours = resolve(&doc(
        "metadata:\n  brief.dateModified: \"2026-04-11T14:22:00-05:00\"\n  brief.source: ./x.brief.md",
    ));
    assert!(!ours.modified_needs_offset);
}

// ---------------------------------------------------------------------------
// tolerance — other people's files must never blow up
// ---------------------------------------------------------------------------

#[test]
fn a_non_string_value_resolves_to_none() {
    let p = resolve(&doc(
        "dateModified: [2026-04-11]\nisBasedOn:\n  nested: yes",
    ));
    assert_eq!(p.date_modified, None);
    assert_eq!(p.is_based_on, None);
}

#[test]
fn a_document_without_frontmatter_resolves_empty() {
    let p = resolve("# Just a heading\n\nNo frontmatter here.\n");
    assert_eq!(p.date_modified, None);
    assert!(p.disagreements.is_empty());
}

#[test]
fn unclosed_or_invalid_frontmatter_resolves_empty_and_never_panics() {
    for bad in [
        "---\ndateModified: 2026-04-11\n\nno closing fence\n",
        "---\n\tthis: is\n  not: valid yaml\n---\n",
        "---\n---\n",
        "",
    ] {
        let p = resolve(bad);
        assert_eq!(p.date_modified, None);
    }
}

#[test]
fn foreign_keys_are_resolved_around_not_tripped_over() {
    let p = resolve(&doc("title: Some vault note\n\
         status: draft\n\
         tags: [a, b]\n\
         aliases: [x]\n\
         dateModified: 2026-04-11\n\
         cssclasses: []"));
    assert_eq!(p.date_modified.as_deref(), Some("2026-04-11"));
    assert!(p.disagreements.is_empty());
}
