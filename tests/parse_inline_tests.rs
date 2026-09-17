//! `docs/bugs.md`: the body parser rebuilt text from a whitelist of
//! `pulldown-cmark` events, so every event it did not name was discarded with
//! no diagnostic. The table below is the symptom table in
//! `bugfix-parser-inline-content.brief.md`, row for row.
//!
//! The rule: **parsed text is the author's source markdown for that span**, minus
//! the block syntax the grammar itself consumes (`#`, `- `, `- [ ] `).

use brief_cli::parse::parse_brief;

fn brief(body: &str) -> brief_cli::model::Brief {
    parse_brief(&format!("---\nstack: [Rust]\n---\n\n{body}")).expect("should parse")
}

// ---------------------------------------------------------------------------
// the five reported symptoms
// ---------------------------------------------------------------------------

/// 1: an `Event::Code` arm existed for `in_item` but not for `in_heading`, so a
/// code span in a heading was deleted along with its contents.
#[test]
fn a_code_span_in_the_h1_survives() {
    let b = brief("# Add `brief emit xml` to the tool\n");
    assert_eq!(b.goal, "Add `brief emit xml` to the tool");
}

/// 2: the same hole, in the H2 that names an unknown section.
#[test]
fn a_code_span_in_an_h2_survives() {
    let b = brief("# Goal\n\n## Notes on `src/check.rs`\n\nProse.\n");
    assert_eq!(
        b.unknown_sections.first().map(|s| s.heading.as_str()),
        Some("Notes on `src/check.rs`")
    );
}

/// 3: link URLs were dropped and only the label kept, in the goal, all three
/// constraint tiers, sacred reasons, assumptions and the deliverable.
#[test]
fn a_link_keeps_its_url_in_every_section() {
    let b = brief(concat!(
        "# See [the plan](./docs/plan.md)\n\n",
        "## Constraints\n\n",
        "### Hard\n\n",
        "- See [the design doc](./docs/design.md) for context\n\n",
        "### Soft\n\n",
        "- Follow [the style guide](./STYLE.md)\n\n",
        "### Ask First\n\n",
        "- Anything in [the schema](./schema.json)\n\n",
        "## Sacred\n\n",
        "- `docs/schema/*` — see [the spec](./SPEC.md)\n\n",
        "## Assumptions\n\n",
        "- [ ] [The gateway](./gw.md) scales\n\n",
        "## Deliverable\n\n",
        "Ship [the thing](./thing.md).\n",
    ));
    assert_eq!(b.goal, "See [the plan](./docs/plan.md)");
    assert_eq!(
        b.constraints.hard[0].to_string(),
        "See [the design doc](./docs/design.md) for context"
    );
    assert_eq!(
        b.constraints.soft[0].to_string(),
        "Follow [the style guide](./STYLE.md)"
    );
    assert_eq!(
        b.constraints.ask_first[0].to_string(),
        "Anything in [the schema](./schema.json)"
    );
    assert_eq!(b.sacred[0].reason, "see [the spec](./SPEC.md)");
    assert_eq!(b.assumptions[0].text, "[The gateway](./gw.md) scales");
    assert_eq!(
        b.deliverable.as_deref(),
        Some("Ship [the thing](./thing.md).")
    );
}

/// 4: emphasis markers were dropped, so a constraint lost the word it stressed.
#[test]
fn emphasis_markers_survive_with_the_authors_spelling() {
    let b = brief(concat!(
        "# Goal\n\n## Constraints\n\n### Hard\n\n",
        "- Use `serde` and **never** `unwrap`\n",
        "- Prefer _composition_ over __inheritance__\n",
        "- Use *this* spelling too\n",
    ));
    assert_eq!(
        b.constraints.hard[0].to_string(),
        "Use `serde` and **never** `unwrap`"
    );
    assert_eq!(
        b.constraints.hard[1].to_string(),
        "Prefer _composition_ over __inheritance__"
    );
    assert_eq!(b.constraints.hard[2].to_string(), "Use *this* spelling too");
}

/// 5: deliverable paragraphs were concatenated with no separator.
#[test]
fn deliverable_paragraphs_are_separated_by_a_blank_line() {
    let b = brief("# Goal\n\n## Deliverable\n\nPara one.\n\nPara two.\n");
    assert_eq!(b.deliverable.as_deref(), Some("Para one.\n\nPara two."));
}

// ---------------------------------------------------------------------------
// the rule, in the corners
// ---------------------------------------------------------------------------

/// A bare autolink and a reference-style link keep their source spelling
/// rather than being normalized to an inline link.
#[test]
fn autolinks_and_reference_links_keep_their_source_spelling() {
    let b = brief(concat!(
        "# Goal\n\n## Constraints\n\n### Hard\n\n",
        "- Read <https://example.com/spec>\n",
        "- Read [the spec][spec]\n\n",
        "[spec]: https://example.com/spec\n",
    ));
    assert_eq!(
        b.constraints.hard[0].to_string(),
        "Read <https://example.com/spec>"
    );
    assert_eq!(b.constraints.hard[1].to_string(), "Read [the spec][spec]");
}

/// A soft break inside one paragraph stays a single newline, and inside a list
/// item stays a space. Unchanged behavior, pinned because the collector now
/// decides it in one place.
#[test]
fn soft_breaks_keep_their_context_dependent_meaning() {
    let b = brief(concat!(
        "# Goal\n\n## Constraints\n\n### Hard\n\n",
        "- One constraint\n  wrapped over two lines\n\n",
        "## Deliverable\n\nOne paragraph\nwrapped over two lines.\n",
    ));
    assert_eq!(
        b.constraints.hard[0].to_string(),
        "One constraint wrapped over two lines"
    );
    assert_eq!(
        b.deliverable.as_deref(),
        Some("One paragraph\nwrapped over two lines.")
    );
}

/// The sacred parse path finds its path by its own rule — the first span is a
/// code span — and must keep doing so now that the reason can carry markup.
#[test]
fn sacred_paths_still_parse_when_the_reason_carries_markup() {
    let b = brief(concat!(
        "# Goal\n\n## Sacred\n\n",
        "- `src/auth/**` — **audited**, see [the review](./review.md)\n",
    ));
    assert_eq!(b.sacred[0].path, "src/auth/**");
    assert_eq!(
        b.sacred[0].reason,
        "**audited**, see [the review](./review.md)"
    );
    assert!(b.sacred[0].well_formed);
}

/// A constraint's `[glob]` scope prefix is block syntax the grammar consumes;
/// it must not be mistaken for a link now that links are kept.
#[test]
fn a_scope_prefix_is_still_a_scope_and_not_a_link() {
    let b = brief(concat!(
        "# Goal\n\n## Constraints\n\n### Hard\n\n",
        "- [`src/ui/**`] Use [the tokens](./tokens.md)\n",
    ));
    assert_eq!(b.constraints.hard[0].scope, vec!["src/ui/**".to_string()]);
    assert_eq!(
        b.constraints.hard[0].to_string(),
        "Use [the tokens](./tokens.md)"
    );
}

/// Nested markup: the whole span comes back as the author wrote it.
#[test]
fn nested_inline_markup_comes_back_whole() {
    let b = brief(concat!(
        "# Goal\n\n## Constraints\n\n### Hard\n\n",
        "- See [**the** `design` doc](./d.md) first\n",
    ));
    assert_eq!(
        b.constraints.hard[0].to_string(),
        "See [**the** `design` doc](./d.md) first"
    );
}

/// The identity section runs through the same collector.
#[test]
fn identity_keeps_its_markup() {
    let b = brief("# Goal\n\n## Identity\n\n- You maintain [brief](./README.md)\n");
    assert_eq!(
        b.identity.map(|i| i.content),
        Some("You maintain [brief](./README.md)".to_string())
    );
}
