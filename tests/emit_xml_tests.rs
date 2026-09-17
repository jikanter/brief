//! `brief emit xml` — the hard-boundary envelope target.
//!
//! These tests hold the line the emitter exists for: output that a real XML
//! parser accepts, with every authored value surviving byte-exact, so a briefing
//! concatenated with untrusted or bulky content still has unambiguous section
//! termini.

use assert_cmd::prelude::*;
use brief_cli::budget;
use brief_cli::emit::{emit_prompt, emit_xml};
use brief_cli::framing::frame_hard;
use brief_cli::model::Brief;
use brief_cli::parse::parse_brief;
use quick_xml::Reader;
use quick_xml::events::Event;

/// Every fixture whose XML output must parse.
const FIXTURES: &[&str] = &[
    "tests/fixtures/minimal.brief.md",
    "tests/fixtures/full.brief.md",
    "tests/fixtures/skill.brief.md",
    "tests/fixtures/rich-unknown.brief.md",
    "tests/fixtures/xml-escaping.brief.md",
    "examples/sample.brief.md",
    ".brief.md",
];

const ESCAPING: &str = "tests/fixtures/xml-escaping.brief.md";

fn repo_path(rel: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel)
}

fn brief(rel: &str) -> Brief {
    let raw = std::fs::read_to_string(repo_path(rel))
        .unwrap_or_else(|e| panic!("failed to read {rel}: {e}"));
    parse_brief(&raw).unwrap_or_else(|e| panic!("{rel} must parse: {e}"))
}

fn xml(rel: &str) -> String {
    emit_xml(&brief(rel))
}

/// One parsed element: its tag, its attributes, and the text directly inside it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Element {
    name: String,
    attrs: Vec<(String, String)>,
    text: String,
}

/// Parse with a real XML parser, panicking on any ill-formedness (unbalanced
/// tags, stray `<`, bad entity, junk after the root). Returns every element with
/// its unescaped attribute values and text — which is what the round-trip
/// assertions compare against the authored source.
fn parse(xml: &str) -> Vec<Element> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().check_end_names = true;
    let mut stack: Vec<Element> = Vec::new();
    let mut done: Vec<Element> = Vec::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let name =
                    String::from_utf8(e.name().as_ref().to_vec()).expect("tag name is utf-8");
                let attrs = e
                    .attributes()
                    .map(|a| {
                        let a = a.expect("attribute must parse");
                        (
                            String::from_utf8(a.key.as_ref().to_vec()).expect("attr name is utf-8"),
                            a.unescape_value()
                                .expect("attribute value must unescape")
                                .into_owned(),
                        )
                    })
                    .collect();
                stack.push(Element {
                    name,
                    attrs,
                    text: String::new(),
                });
            }
            Ok(Event::Text(e)) => {
                if let Some(top) = stack.last_mut() {
                    let raw = e.decode().expect("text must decode");
                    let text = quick_xml::escape::unescape(&raw).expect("text must unescape");
                    top.text.push_str(&text);
                }
            }
            Ok(Event::CData(e)) => {
                if let Some(top) = stack.last_mut() {
                    top.text.push_str(&e.decode().expect("cdata must decode"));
                }
            }
            // quick-xml surfaces each entity reference as its own event rather
            // than inlining it in the surrounding text, so `&lt;` etc. have to
            // be resolved back here — otherwise the round-trip assertions would
            // silently compare against text with the entities dropped.
            Ok(Event::GeneralRef(e)) => {
                if let Some(top) = stack.last_mut() {
                    let raw = e.decode().expect("entity must decode");
                    let resolved = match raw.as_ref() {
                        "amp" => "&".to_string(),
                        "lt" => "<".to_string(),
                        "gt" => ">".to_string(),
                        "quot" => "\"".to_string(),
                        "apos" => "'".to_string(),
                        _ => e
                            .resolve_char_ref()
                            .expect("entity must resolve")
                            .unwrap_or_else(|| panic!("unknown entity `&{raw};` in output"))
                            .to_string(),
                    };
                    top.text.push_str(&resolved);
                }
            }
            Ok(Event::End(_)) => {
                let el = stack.pop().expect("end tag without a matching start");
                done.push(el);
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(e) => panic!(
                "emitted XML is not well-formed at position {}: {e}",
                reader.buffer_position()
            ),
        }
    }

    assert!(
        stack.is_empty(),
        "unclosed elements: {:?}",
        stack.iter().map(|e| &e.name).collect::<Vec<_>>()
    );
    done
}

fn text_of<'a>(els: &'a [Element], name: &str) -> Vec<&'a str> {
    els.iter()
        .filter(|e| e.name == name)
        .map(|e| e.text.as_str())
        .collect()
}

fn attr_of<'a>(els: &'a [Element], name: &str, attr: &str) -> Vec<&'a str> {
    els.iter()
        .filter(|e| e.name == name)
        .filter_map(|e| {
            e.attrs
                .iter()
                .find(|(k, _)| k == attr)
                .map(|(_, v)| v.as_str())
        })
        .collect()
}

#[test]
fn every_fixture_emits_well_formed_xml() {
    for rel in FIXTURES {
        let out = xml(rel);
        let els = parse(&out);
        assert!(
            els.iter().any(|e| e.name == "brief"),
            "{rel}: output must have a <brief> root"
        );
    }
}

#[test]
fn even_a_malformed_brief_emits_well_formed_xml() {
    // The parser is tolerant, so a brief that fails `brief validate` still
    // reaches the emitter. Invalid *content* must never become invalid *XML*.
    let els = parse(&xml("tests/fixtures/malformed.brief.md"));
    assert!(els.iter().any(|e| e.name == "brief"));
}

#[test]
fn markup_in_authored_text_survives_byte_exact() {
    let els = parse(&xml(ESCAPING));

    assert_eq!(
        text_of(&els, "goal"),
        vec!["Harden the parser against <script> tags & \"quoted\" input"],
        "angle brackets, ampersand and quotes must round-trip"
    );

    let deliverable = text_of(&els, "deliverable");
    assert_eq!(
        deliverable,
        vec![
            "Well-formed XML whose text keeps A & B < C > D and 'single' and \"double\" quotes intact."
        ]
    );
}

#[test]
fn a_fenced_code_block_survives_byte_exact() {
    let els = parse(&xml(ESCAPING));
    let notes = els
        .iter()
        .find(|e| e.name == "section" && e.attrs.iter().any(|(_, v)| v == "Notes"))
        .expect("the Notes section must be emitted");

    for line in [
        "```rust",
        "    if a < b && c > d {",
        "        println!(\"{}\", \"<tag attr='v'>\");",
    ] {
        assert!(
            notes.text.contains(line),
            "fenced block line missing from <section name=\"Notes\">: {line}\ngot:\n{}",
            notes.text
        );
    }
}

#[test]
fn sacred_globs_emit_verbatim() {
    let els = parse(&xml(ESCAPING));
    let paths = attr_of(&els, "region", "path");
    assert_eq!(
        paths,
        vec!["src/**/*.rs", "docs/a&b/**"],
        "sacred globs are normative: emitted verbatim, unexpanded"
    );
}

#[test]
fn scope_globs_emit_verbatim() {
    let els = parse(&xml(ESCAPING));
    let scopes = attr_of(&els, "rule", "scope");
    assert_eq!(
        scopes,
        vec!["src/**/*.rs"],
        "constraint scope globs round-trip through the attribute"
    );
}

#[test]
fn output_is_byte_identical_across_runs() {
    for rel in FIXTURES {
        let brief = brief(rel);
        assert_eq!(
            emit_xml(&brief),
            emit_xml(&brief),
            "{rel}: emit must be deterministic"
        );
        // And across a re-parse of the same source, not just a re-render.
        assert_eq!(xml(rel), xml(rel), "{rel}: emit must be deterministic");
    }
}

#[test]
fn nothing_the_prompt_target_carries_is_dropped() {
    for rel in FIXTURES {
        let b = brief(rel);
        let prompt = emit_prompt(&b);
        let els = parse(&emit_xml(&b));
        let rules = text_of(&els, "rule");

        for c in &b.constraints.hard {
            // The prompt target is the parity baseline; both targets reframe
            // through the same `framing` pass, so compare the framed form.
            let framed = frame_hard(c);
            assert!(
                prompt.contains(&framed),
                "{rel}: precondition — prompt target must carry: {framed}"
            );
            assert!(
                rules.iter().any(|r| *r == framed),
                "{rel}: hard constraint dropped from XML: {framed}"
            );
        }

        let paths = attr_of(&els, "region", "path");
        for entry in &b.sacred {
            assert!(
                paths.contains(&entry.path.as_str()),
                "{rel}: sacred path dropped from XML: {}",
                entry.path
            );
        }

        if let Some(deliverable) = &b.deliverable {
            let emitted = text_of(&els, "deliverable");
            assert!(
                emitted
                    .iter()
                    .any(|d| d.trim() == deliverable.trim_end_matches('\n').trim()),
                "{rel}: deliverable dropped from XML"
            );
        }
    }
}

#[test]
fn identity_is_carried_into_the_envelope() {
    let els = parse(&xml(ESCAPING));
    let identity = text_of(&els, "identity");
    assert_eq!(
        identity,
        vec!["The `<brief>` envelope team — owners of A & B."],
        "an authored ## Identity section must not be silently dropped"
    );
}

#[test]
fn compact_output_is_well_formed_and_keeps_hard_and_sacred() {
    for rel in FIXTURES {
        let b = brief(rel);
        let compacted = budget::compact(&b);
        let els = parse(&emit_xml(&compacted));
        let rules = text_of(&els, "rule");
        let paths = attr_of(&els, "region", "path");

        for c in &b.constraints.hard {
            let framed = frame_hard(c);
            assert!(
                rules.iter().any(|r| *r == framed),
                "{rel}: --compact dropped a hard constraint: {framed}"
            );
        }
        for entry in &b.sacred {
            assert!(
                paths.contains(&entry.path.as_str()),
                "{rel}: --compact dropped a sacred path: {}",
                entry.path
            );
        }
    }
}

#[test]
fn golden_envelope_for_the_full_fixture() {
    // The one byte-for-byte snapshot: it makes an unintended shape change show
    // up as a diff rather than as a still-passing `contains()`.
    // Regenerate with: BLESS_GOLDEN=1 cargo test --test emit_xml_tests
    let out = xml("tests/fixtures/full.brief.md");
    let path = repo_path("tests/fixtures/golden/full.brief.xml");

    if std::env::var_os("BLESS_GOLDEN").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &out).expect("failed to write golden file");
    }

    let golden = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    assert_eq!(
        out, golden,
        "xml output changed. If intended, regenerate with \
         `BLESS_GOLDEN=1 cargo test --test emit_xml_tests`."
    );
}

#[test]
fn install_is_refused_with_an_explanation_and_writes_nothing() {
    // There is no canonical on-disk location for an XML envelope: it is a
    // stdout/pipe target for a CI step. The refusal has to say that, and point
    // somewhere useful, rather than listing target names.
    let dir = tempfile::tempdir().unwrap();
    let brief_path = dir.path().join(".brief.md");
    std::fs::copy(repo_path("tests/fixtures/minimal.brief.md"), &brief_path).unwrap();

    let before: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();

    let out = std::process::Command::cargo_bin("brief")
        .unwrap()
        .current_dir(dir.path())
        .arg("--file")
        .arg(&brief_path)
        .arg("emit")
        .arg("xml")
        .arg("--install")
        .output()
        .unwrap();

    assert!(
        !out.status.success(),
        "--install must fail for the xml target"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("xml"),
        "the message must name the target, got: {stderr}"
    );
    assert!(
        stderr.contains("stdout") || stderr.contains("pipe"),
        "the message must say why there is no install location, got: {stderr}"
    );
    assert!(
        stderr.contains("emit claude --install"),
        "the message must point at the installable target, got: {stderr}"
    );

    let after: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(before, after, "a refused --install must write nothing");
}
