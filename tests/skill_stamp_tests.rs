//! Phase 4 of `docs/design/provenance-schema.md`: brief stamps its own
//! provenance on a generated `SKILL.md`, under `metadata.brief.*` and nowhere
//! else, and only when the content actually changed.

use brief_cli::skill::{
    BRIEF_DATE_MODIFIED_KEY, BRIEF_SOURCE_KEY, get_brief_metadata, set_brief_metadata, stamp_is_due,
};

// ---------------------------------------------------------------------------
// the generalized metadata.brief.* setter
// ---------------------------------------------------------------------------

const SKILL: &str = "---\nname: review\ndescription: Review code\nmetadata:\n  brief.source: ../../x.brief.md\n---\n\nBody\n";

#[test]
fn reads_any_brief_namespaced_key() {
    assert_eq!(
        get_brief_metadata(SKILL, BRIEF_SOURCE_KEY).as_deref(),
        Some("../../x.brief.md")
    );
    assert_eq!(get_brief_metadata(SKILL, BRIEF_DATE_MODIFIED_KEY), None);
}

#[test]
fn adds_a_second_key_without_disturbing_the_first() {
    let out = set_brief_metadata(SKILL, BRIEF_DATE_MODIFIED_KEY, "2026-04-11T14:22:00Z");
    assert_eq!(
        get_brief_metadata(&out, BRIEF_SOURCE_KEY).as_deref(),
        Some("../../x.brief.md")
    );
    assert_eq!(
        get_brief_metadata(&out, BRIEF_DATE_MODIFIED_KEY).as_deref(),
        Some("2026-04-11T14:22:00Z")
    );
    assert!(out.contains("name: review"));
    assert!(out.ends_with("Body\n"));
}

#[test]
fn replaces_a_key_in_place_rather_than_appending_a_duplicate() {
    let once = set_brief_metadata(SKILL, BRIEF_DATE_MODIFIED_KEY, "2026-01-01T00:00:00Z");
    let twice = set_brief_metadata(&once, BRIEF_DATE_MODIFIED_KEY, "2026-04-11T14:22:00Z");
    assert_eq!(twice.matches(BRIEF_DATE_MODIFIED_KEY).count(), 1);
    assert_eq!(
        get_brief_metadata(&twice, BRIEF_DATE_MODIFIED_KEY).as_deref(),
        Some("2026-04-11T14:22:00Z")
    );
}

/// A timestamp is quoted, because the Agent Skills spec describes `metadata`
/// as a map of string keys to string *values* and a bare YAML timestamp is not
/// a string.
#[test]
fn a_written_timestamp_is_quoted() {
    let out = set_brief_metadata(SKILL, BRIEF_DATE_MODIFIED_KEY, "2026-04-11T14:22:00Z");
    assert!(
        out.contains("brief.dateModified: \"2026-04-11T14:22:00Z\""),
        "{out}"
    );
}

#[test]
fn every_other_line_survives_byte_for_byte() {
    let messy = "---\nname: x\ndescription: d\nlicense: MIT\nmetadata:\n  author: someone\n  brief.source: ./a.brief.md\n  zzz: last\n---\n\n# Body\n\nProse.\n";
    let out = set_brief_metadata(messy, BRIEF_DATE_MODIFIED_KEY, "2026-04-11T00:00:00Z");
    for line in [
        "license: MIT",
        "  author: someone",
        "  zzz: last",
        "# Body",
        "Prose.",
    ] {
        assert!(out.contains(line), "lost `{line}`:\n{out}");
    }
}

// ---------------------------------------------------------------------------
// rule 5: stamp only when the content changed
// ---------------------------------------------------------------------------

#[test]
fn a_stamp_is_due_when_there_is_no_file_yet() {
    let fresh =
        "---\nname: x\ndescription: d\nmetadata:\n  brief.source: ./a.brief.md\n---\n\nBody\n";
    assert!(stamp_is_due(None, fresh));
}

#[test]
fn a_stamp_is_not_due_when_only_the_stamp_differs() {
    let on_disk = "---\nname: x\ndescription: d\nmetadata:\n  brief.source: ./a.brief.md\n  brief.dateModified: \"2026-01-01T00:00:00Z\"\n---\n\nBody\n";
    let regenerated =
        "---\nname: x\ndescription: d\nmetadata:\n  brief.source: ./a.brief.md\n---\n\nBody\n";
    assert!(!stamp_is_due(Some(on_disk), regenerated));
}

#[test]
fn a_stamp_is_due_when_the_body_changed() {
    let on_disk = "---\nname: x\ndescription: d\nmetadata:\n  brief.source: ./a.brief.md\n  brief.dateModified: \"2026-01-01T00:00:00Z\"\n---\n\nOld body\n";
    let regenerated =
        "---\nname: x\ndescription: d\nmetadata:\n  brief.source: ./a.brief.md\n---\n\nNew body\n";
    assert!(stamp_is_due(Some(on_disk), regenerated));
}

#[test]
fn a_stamp_is_due_when_the_source_pointer_moved() {
    let on_disk = "---\nname: x\ndescription: d\nmetadata:\n  brief.source: ./a.brief.md\n  brief.dateModified: \"2026-01-01T00:00:00Z\"\n---\n\nBody\n";
    let regenerated =
        "---\nname: x\ndescription: d\nmetadata:\n  brief.source: ./moved.brief.md\n---\n\nBody\n";
    assert!(stamp_is_due(Some(on_disk), regenerated));
}

// ---------------------------------------------------------------------------
// the clock
// ---------------------------------------------------------------------------

/// Known Unix timestamps, so the date maths is pinned without a date crate and
/// without depending on when the suite runs.
#[test]
fn formats_a_unix_timestamp_as_iso_8601_utc() {
    use brief_cli::skill::format_utc;
    let cases = [
        (0_i64, "1970-01-01T00:00:00Z"),
        (1_000_000_000, "2001-09-09T01:46:40Z"),
        // 2024-02-29: a leap day, the classic off-by-one.
        (1_709_164_800, "2024-02-29T00:00:00Z"),
        // 2000-02-29: a leap day in a century that is also a leap year.
        (951_782_400, "2000-02-29T00:00:00Z"),
        (951_868_799, "2000-02-29T23:59:59Z"),
    ];
    for (secs, want) in cases {
        assert_eq!(format_utc(secs), want, "for {secs}");
    }
}

#[test]
fn the_clock_produces_a_value_the_resolver_accepts() {
    use brief_cli::provenance::resolve;
    use brief_cli::skill::{BRIEF_DATE_MODIFIED_KEY, now_utc, set_brief_metadata};

    let stamped = set_brief_metadata(SKILL, BRIEF_DATE_MODIFIED_KEY, &now_utc());
    let p = resolve(&stamped);
    assert!(p.brief_written);
    assert!(p.date_modified.is_some());
    // Written with an explicit offset, so it is never flagged as brief's bug.
    assert!(!p.modified_needs_offset, "{stamped}");
    assert!(p.modified_day().is_some());
}

// ---------------------------------------------------------------------------
// end to end: the install path
// ---------------------------------------------------------------------------

/// The whole point of rule 5, exercised through the CLI: a second
/// `brief skill emit --install` over unchanged content must leave the file
/// byte-identical, or an idempotent install produces a diff on every run.
#[test]
fn reinstalling_unchanged_content_rewrites_nothing() {
    use assert_cmd::Command;
    use tempfile::TempDir;

    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join(".brief.md"),
        "---\nstack: [Rust]\nskill_name: review-code\nskill_description: Review code\n---\n\n# Review code\n\n## Constraints\n\n### Hard\n- Tests must pass\n",
    )
    .unwrap();

    let skill_md = dir.path().join(".claude/skills/review-code/SKILL.md");

    let mut first = Command::cargo_bin("brief").unwrap();
    first
        .current_dir(dir.path())
        .args(["skill", "emit", "--install"])
        .assert()
        .success();
    let after_first = std::fs::read_to_string(&skill_md).unwrap();

    assert!(
        after_first.contains("brief.dateModified:"),
        "install should stamp:\n{after_first}"
    );

    let mut second = Command::cargo_bin("brief").unwrap();
    second
        .current_dir(dir.path())
        .args(["skill", "emit", "--install"])
        .assert()
        .success();
    let after_second = std::fs::read_to_string(&skill_md).unwrap();

    assert_eq!(
        after_first, after_second,
        "a re-install over unchanged content must not rewrite the file"
    );
}

#[test]
fn a_changed_brief_moves_the_stamp() {
    use assert_cmd::Command;
    use tempfile::TempDir;

    let dir = TempDir::new().unwrap();
    let brief_path = dir.path().join(".brief.md");
    let base = "---\nstack: [Rust]\nskill_name: review-code\nskill_description: Review code\n---\n\n# Review code\n\n## Constraints\n\n### Hard\n- Tests must pass\n";
    std::fs::write(&brief_path, base).unwrap();

    let skill_md = dir.path().join(".claude/skills/review-code/SKILL.md");

    Command::cargo_bin("brief")
        .unwrap()
        .current_dir(dir.path())
        .args(["skill", "emit", "--install"])
        .assert()
        .success();
    let before = std::fs::read_to_string(&skill_md).unwrap();

    // Change the content the skill is generated from.
    std::fs::write(
        &brief_path,
        base.replace("Tests must pass", "Tests and clippy must pass"),
    )
    .unwrap();

    Command::cargo_bin("brief")
        .unwrap()
        .current_dir(dir.path())
        .args(["skill", "emit", "--install"])
        .assert()
        .success();
    let after = std::fs::read_to_string(&skill_md).unwrap();

    assert_ne!(before, after, "changed content should be rewritten");
    assert!(after.contains("clippy"), "{after}");
}
