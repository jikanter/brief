//! The floor under `docs/design/body-tags.md`.
//!
//! Rule R1 there: a brief with no `<brief:*>` tag emits, for every target,
//! exactly the bytes it emits today. These goldens are blessed from the
//! pre-tag emitters, so any parser or emitter change for tags that moves a
//! single byte of tag-free output shows up here as a diff, not as a still
//! passing `contains()`.
//!
//! Regenerate with: BLESS_GOLDEN=1 cargo test --test body_tags_tests
//!
//! The second test pins the other half of the rule: today a `brief:` tag is
//! plain text to the parser, which is the degradation a version-1 reader must
//! keep once tags mean something.

use brief_cli::emit;
use brief_cli::model::Brief;
use brief_cli::parse::parse_brief;

/// Every tag-free fixture the goldens cover.
const FIXTURES: &[&str] = &[
    "tests/fixtures/minimal.brief.md",
    "tests/fixtures/full.brief.md",
    "tests/fixtures/skill.brief.md",
    "tests/fixtures/rich-unknown.brief.md",
    "tests/fixtures/xml-escaping.brief.md",
    "tests/fixtures/brief-id.brief.md",
    "examples/sample.brief.md",
];

/// Every stdout target whose bytes the rule covers. `promptfoo` is left out:
/// it needs a cassette binding no fixture has, and its output carries no
/// body text a tag could change.
type Emitter = fn(&Brief) -> String;

const TARGETS: &[(&str, Emitter)] = &[
    ("claude", emit::emit_claude),
    ("agents-md", emit::emit_agents_md),
    ("cursor", emit::emit_cursor),
    ("copilot", emit::emit_copilot),
    ("windsurf", emit::emit_windsurf),
    ("aider", emit::emit_aider),
    ("readme", emit::emit_readme),
    ("prompt", emit::emit_prompt),
    ("anchor", emit::emit_anchor),
    ("json", emit::emit_json),
    ("jsonl", emit::emit_jsonl),
    ("xml", emit::emit_xml),
];

fn repo_path(rel: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel)
}

fn brief(rel: &str) -> Brief {
    let raw = std::fs::read_to_string(repo_path(rel))
        .unwrap_or_else(|e| panic!("failed to read {rel}: {e}"));
    parse_brief(&raw).unwrap_or_else(|e| panic!("{rel} must parse: {e}"))
}

fn golden_path(fixture: &str, target: &str) -> std::path::PathBuf {
    let stem = std::path::Path::new(fixture)
        .file_name()
        .unwrap()
        .to_string_lossy()
        .trim_end_matches(".brief.md")
        .to_string();
    repo_path(&format!("tests/fixtures/golden/untagged/{stem}.{target}"))
}

#[test]
fn fixtures_carry_no_brief_tags() {
    // The goldens only mean something for tag-free input. If a fixture grows
    // a tag, it moves out of this set and gets its own test.
    for fixture in FIXTURES {
        let raw = std::fs::read_to_string(repo_path(fixture)).unwrap();
        assert!(
            !raw.contains("<brief:"),
            "{fixture} contains a brief: tag; it no longer belongs in the untagged golden set"
        );
    }
}

#[test]
fn tag_free_briefs_emit_todays_bytes_for_every_target() {
    let bless = std::env::var_os("BLESS_GOLDEN").is_some();
    let mut diffs = Vec::new();

    for fixture in FIXTURES {
        let brief = brief(fixture);
        for (target, emit_fn) in TARGETS {
            let out = emit_fn(&brief);
            let path = golden_path(fixture, target);

            if bless {
                std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                std::fs::write(&path, &out).expect("failed to write golden file");
            }

            let golden = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
            if out != golden {
                diffs.push(format!("{fixture} -> {target}"));
            }
        }
    }

    assert!(
        diffs.is_empty(),
        "tag-free output changed for:\n  {}\nIf intended, regenerate with \
         `BLESS_GOLDEN=1 cargo test --test body_tags_tests` and review the diff.",
        diffs.join("\n  ")
    );
}

#[test]
fn a_brief_tag_is_plain_text_to_the_version_one_parser() {
    // Rule R4's degradation, as it stands before tags mean anything: the
    // parser keeps the tag in the item text and every emitter passes it
    // through. This is the behaviour a reader that predates the vocabulary
    // must keep, so it is pinned now rather than assumed later.
    let raw = "---\nstack: [Rust]\n---\n\n# Tagged\n\n## Constraints\n\n### Hard\n- <brief:anchor/> No network crates\n";
    let brief = parse_brief(raw).expect("a tag must not break parsing");

    assert_eq!(brief.constraints.hard.len(), 1);
    assert_eq!(
        brief.constraints.hard[0].text,
        "<brief:anchor/> No network crates"
    );

    for (target, emit_fn) in TARGETS {
        let out = emit_fn(&brief);
        let survives = out.contains("<brief:anchor/>") || out.contains("&lt;brief:anchor/&gt;");
        assert!(survives, "{target} dropped or rewrote the tag: {out}");
    }
}
