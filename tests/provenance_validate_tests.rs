//! `brief validate`'s provenance checks — section 6 of
//! `docs/design/provenance-schema.md`.
//!
//! Every check here is a warning or a hint. Nothing in this vocabulary is an
//! error, and nothing changes the exit code.

use std::path::Path;
use std::process::Command;

use brief_cli::model::Severity;
use brief_cli::parse::parse_brief;
use brief_cli::validate::{ValidateOptions, validate_with};
use tempfile::TempDir;

fn write(dir: &Path, rel: &str, body: &str) {
    let path = dir.join(rel);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, body).unwrap();
}

/// A brief whose `context:` lists `entries`.
fn brief_with_context(entries: &[&str]) -> String {
    format!(
        "---\nstack: [Rust]\ncontext: [{}]\n---\n\n# Goal\n\n## Deliverable\nDone.\n",
        entries.join(", ")
    )
}

fn run(dir: &TempDir, brief_src: &str, opts: ValidateOptions) -> Vec<brief_cli::model::Diagnostic> {
    write(dir.path(), ".brief.md", brief_src);
    let brief = parse_brief(brief_src).unwrap();
    validate_with(&brief, dir.path(), Path::new(".brief.md"), opts)
}

fn messages(diags: &[brief_cli::model::Diagnostic]) -> String {
    diags
        .iter()
        .map(|d| d.message.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

// ---------------------------------------------------------------------------
// reference existence
// ---------------------------------------------------------------------------

#[test]
fn a_dangling_is_based_on_warns() {
    let dir = TempDir::new().unwrap();
    write(
        dir.path(),
        "doc.md",
        "---\nisBasedOn: ./gone.md\n---\n\n# Doc\n",
    );
    let diags = run(
        &dir,
        &brief_with_context(&["./doc.md"]),
        ValidateOptions::default(),
    );
    let m = messages(&diags);
    assert!(m.contains("isBasedOn"), "{m}");
    assert!(m.contains("gone.md"), "{m}");
    assert!(diags.iter().all(|d| d.severity != Severity::Error), "{m}");
}

#[test]
fn a_resolvable_is_based_on_is_silent() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "base.md", "# Base\n");
    write(
        dir.path(),
        "doc.md",
        "---\nisBasedOn: ./base.md\n---\n\n# Doc\n",
    );
    let diags = run(
        &dir,
        &brief_with_context(&["./doc.md"]),
        ValidateOptions::default(),
    );
    assert!(
        !messages(&diags).contains("isBasedOn"),
        "{}",
        messages(&diags)
    );
}

/// A path is relative to the document that carries it, not to the brief.
#[test]
fn a_reference_resolves_against_its_own_document() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "notes/base.md", "# Base\n");
    write(
        dir.path(),
        "notes/doc.md",
        "---\nisBasedOn: ./base.md\n---\n\n# Doc\n",
    );
    let diags = run(
        &dir,
        &brief_with_context(&["./notes/doc.md"]),
        ValidateOptions::default(),
    );
    assert!(
        !messages(&diags).contains("isBasedOn"),
        "{}",
        messages(&diags)
    );
}

#[test]
fn a_url_reference_is_never_checked() {
    let dir = TempDir::new().unwrap();
    write(
        dir.path(),
        "doc.md",
        "---\nisBasedOn: https://example.com/x\nsuperseded_by: ace://prj/00000123\n---\n\n# Doc\n",
    );
    let diags = run(
        &dir,
        &brief_with_context(&["./doc.md"]),
        ValidateOptions::default(),
    );
    let m = messages(&diags);
    assert!(!m.contains("example.com"), "{m}");
    assert!(!m.contains("ace://"), "{m}");
}

// ---------------------------------------------------------------------------
// superseded_by
// ---------------------------------------------------------------------------

#[test]
fn a_context_doc_that_is_superseded_warns_and_names_the_successor() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "new.md", "# New\n");
    write(
        dir.path(),
        "old.md",
        "---\nsuperseded_by: ./new.md\n---\n\n# Old\n",
    );
    let diags = run(
        &dir,
        &brief_with_context(&["./old.md"]),
        ValidateOptions::default(),
    );
    let m = messages(&diags);
    assert!(m.contains("old.md"), "{m}");
    assert!(m.contains("new.md"), "{m}");
}

/// The warning names the end of the chain, not the next hop.
#[test]
fn a_supersession_chain_is_followed_to_its_end() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "c.md", "# C\n");
    write(
        dir.path(),
        "b.md",
        "---\nsuperseded_by: ./c.md\n---\n\n# B\n",
    );
    write(
        dir.path(),
        "a.md",
        "---\nsuperseded_by: ./b.md\n---\n\n# A\n",
    );
    let diags = run(
        &dir,
        &brief_with_context(&["./a.md"]),
        ValidateOptions::default(),
    );
    let m = messages(&diags);
    assert!(m.contains("c.md"), "chain should end at c.md: {m}");
}

#[test]
fn a_supersession_cycle_warns_instead_of_hanging() {
    let dir = TempDir::new().unwrap();
    write(
        dir.path(),
        "a.md",
        "---\nsuperseded_by: ./b.md\n---\n\n# A\n",
    );
    write(
        dir.path(),
        "b.md",
        "---\nsuperseded_by: ./a.md\n---\n\n# B\n",
    );
    let diags = run(
        &dir,
        &brief_with_context(&["./a.md"]),
        ValidateOptions::default(),
    );
    let m = messages(&diags);
    assert!(m.to_lowercase().contains("cycl"), "{m}");
    assert!(diags.iter().all(|d| d.severity != Severity::Error), "{m}");
}

#[test]
fn a_dangling_superseded_by_warns() {
    let dir = TempDir::new().unwrap();
    write(
        dir.path(),
        "doc.md",
        "---\nsuperseded_by: ./gone.md\n---\n\n# Doc\n",
    );
    let diags = run(
        &dir,
        &brief_with_context(&["./doc.md"]),
        ValidateOptions::default(),
    );
    assert!(messages(&diags).contains("gone.md"));
}

// ---------------------------------------------------------------------------
// git drift
// ---------------------------------------------------------------------------

/// `%cs` reads the *committer* date, so a fixed commit day needs
/// `GIT_COMMITTER_DATE`; `--date` would only move the author date.
const COMMIT_DAY: &str = "2026-04-11T12:00:00+00:00";

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_COMMITTER_DATE", COMMIT_DAY)
        .env("GIT_AUTHOR_DATE", COMMIT_DAY)
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?} failed");
}

fn git_repo_with_committed_doc(date_modified: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    git(dir.path(), &["init", "-q"]);
    git(dir.path(), &["config", "user.email", "t@example.com"]);
    git(dir.path(), &["config", "user.name", "T"]);
    write(
        dir.path(),
        "doc.md",
        &format!("---\ndateModified: {date_modified}\n---\n\n# Doc\n"),
    );
    git(dir.path(), &["add", "doc.md"]);
    git(
        dir.path(),
        &[
            "-c",
            "commit.gpgsign=false",
            "commit",
            "-q",
            "-m",
            "add doc",
        ],
    );
    dir
}

#[test]
fn a_date_modified_older_than_the_last_commit_warns() {
    let dir = git_repo_with_committed_doc("2026-01-01");
    let diags = run(
        &dir,
        &brief_with_context(&["./doc.md"]),
        ValidateOptions::default(),
    );
    let m = messages(&diags);
    assert!(m.contains("dateModified"), "{m}");
    assert!(m.contains("doc.md"), "{m}");
}

/// A human sets the date, then commits, so the commit is always a little
/// later. Comparing days absorbs that; comparing instants would warn on
/// every file.
#[test]
fn a_date_modified_on_the_commit_day_is_silent() {
    let dir = git_repo_with_committed_doc("2026-04-11");
    let diags = run(
        &dir,
        &brief_with_context(&["./doc.md"]),
        ValidateOptions::default(),
    );
    assert!(
        !messages(&diags).contains("dateModified"),
        "{}",
        messages(&diags)
    );
}

#[test]
fn a_date_modified_newer_than_the_commit_is_silent() {
    let dir = git_repo_with_committed_doc("2026-06-01");
    assert!(
        !messages(&run(
            &dir,
            &brief_with_context(&["./doc.md"]),
            ValidateOptions::default()
        ))
        .contains("dateModified")
    );
}

#[test]
fn outside_a_git_repo_the_drift_check_is_skipped() {
    let dir = TempDir::new().unwrap();
    write(
        dir.path(),
        "doc.md",
        "---\ndateModified: 1999-01-01\n---\n\n# Doc\n",
    );
    let diags = run(
        &dir,
        &brief_with_context(&["./doc.md"]),
        ValidateOptions::default(),
    );
    assert!(
        !messages(&diags).contains("dateModified"),
        "{}",
        messages(&diags)
    );
}

#[test]
fn an_untracked_file_is_skipped() {
    let dir = git_repo_with_committed_doc("2026-04-11");
    write(
        dir.path(),
        "untracked.md",
        "---\ndateModified: 1999-01-01\n---\n\n# U\n",
    );
    let diags = run(
        &dir,
        &brief_with_context(&["./doc.md", "./untracked.md"]),
        ValidateOptions::default(),
    );
    assert!(
        !messages(&diags).contains("untracked.md"),
        "{}",
        messages(&diags)
    );
}

// ---------------------------------------------------------------------------
// the opt-in hint
// ---------------------------------------------------------------------------

#[test]
fn a_context_doc_newer_than_the_brief_is_silent_by_default() {
    let dir = TempDir::new().unwrap();
    write(
        dir.path(),
        "doc.md",
        "---\ndateModified: 2026-06-01\n---\n\n# Doc\n",
    );
    let src = "---\nstack: [Rust]\ncontext: [./doc.md]\ndateModified: 2026-01-01\n---\n\n# Goal\n";
    let diags = run(&dir, src, ValidateOptions::default());
    assert!(
        diags.iter().all(|d| d.severity != Severity::Hint),
        "{}",
        messages(&diags)
    );
}

#[test]
fn a_context_doc_newer_than_the_brief_hints_when_asked() {
    let dir = TempDir::new().unwrap();
    write(
        dir.path(),
        "doc.md",
        "---\ndateModified: 2026-06-01\n---\n\n# Doc\n",
    );
    let src = "---\nstack: [Rust]\ncontext: [./doc.md]\ndateModified: 2026-01-01\n---\n\n# Goal\n";
    let diags = run(&dir, src, ValidateOptions { hints: true });
    let hints: Vec<_> = diags
        .iter()
        .filter(|d| d.severity == Severity::Hint)
        .collect();
    assert_eq!(hints.len(), 1, "{}", messages(&diags));
    assert!(hints[0].message.contains("doc.md"));
}

#[test]
fn the_hint_is_skipped_when_neither_side_has_a_date() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "doc.md", "# Doc\n");
    let diags = run(
        &dir,
        &brief_with_context(&["./doc.md"]),
        ValidateOptions { hints: true },
    );
    assert!(diags.iter().all(|d| d.severity != Severity::Hint));
}

// ---------------------------------------------------------------------------
// disagreement, format complaints, and silence
// ---------------------------------------------------------------------------

#[test]
fn two_addresses_that_disagree_warn() {
    let dir = TempDir::new().unwrap();
    write(
        dir.path(),
        "doc.md",
        "---\ndateModified: 2026-04-11\nupdated: 2026-01-01\n---\n\n# Doc\n",
    );
    let diags = run(
        &dir,
        &brief_with_context(&["./doc.md"]),
        ValidateOptions::default(),
    );
    assert!(messages(&diags).contains("updated"), "{}", messages(&diags));
}

/// Strict timestamp complaints apply only to files brief wrote. On anyone
/// else's document a date-only value is the most common real spelling.
#[test]
fn a_date_only_value_is_silent_on_a_foreign_document() {
    let dir = TempDir::new().unwrap();
    write(
        dir.path(),
        "doc.md",
        "---\ndateModified: 2026-04-11\n---\n\n# Doc\n",
    );
    let diags = run(
        &dir,
        &brief_with_context(&["./doc.md"]),
        ValidateOptions::default(),
    );
    assert!(
        !messages(&diags).to_lowercase().contains("offset"),
        "{}",
        messages(&diags)
    );
}

#[test]
fn a_date_only_value_warns_on_a_brief_written_document() {
    let dir = TempDir::new().unwrap();
    write(
        dir.path(),
        "doc.md",
        "---\nmetadata:\n  brief.source: ./x.brief.md\n  brief.dateModified: \"2026-04-11\"\n---\n\n# Doc\n",
    );
    let diags = run(
        &dir,
        &brief_with_context(&["./doc.md"]),
        ValidateOptions::default(),
    );
    assert!(
        messages(&diags).to_lowercase().contains("offset"),
        "{}",
        messages(&diags)
    );
}

#[test]
fn foreign_keys_produce_no_diagnostic_at_all() {
    let dir = TempDir::new().unwrap();
    write(
        dir.path(),
        "doc.md",
        "---\nstatus: draft\nsize: 12\ntags: [a]\nlayout: post\n---\n\n# Doc\n",
    );
    let diags = run(
        &dir,
        &brief_with_context(&["./doc.md"]),
        ValidateOptions::default(),
    );
    assert!(diags.is_empty(), "{}", messages(&diags));
}

#[test]
fn no_provenance_check_ever_produces_an_error() {
    let dir = TempDir::new().unwrap();
    write(
        dir.path(),
        "doc.md",
        "---\nisBasedOn: ./gone.md\nsuperseded_by: ./also-gone.md\ndateModified: nonsense\n---\n\n# Doc\n",
    );
    let diags = run(
        &dir,
        &brief_with_context(&["./doc.md"]),
        ValidateOptions::default(),
    );
    assert!(
        diags.iter().all(|d| d.severity != Severity::Error),
        "{}",
        messages(&diags)
    );
}

/// A `context:` entry brief cannot open is already reported by the existing
/// "Context file not found" check; provenance must not pile on.
#[test]
fn a_missing_context_file_gets_no_extra_provenance_noise() {
    let dir = TempDir::new().unwrap();
    let diags = run(
        &dir,
        &brief_with_context(&["./nope.md"]),
        ValidateOptions::default(),
    );
    assert_eq!(
        diags
            .iter()
            .filter(|d| d.message.contains("nope.md"))
            .count(),
        1,
        "{}",
        messages(&diags)
    );
}

#[test]
fn the_brief_s_own_provenance_is_checked_too() {
    let dir = TempDir::new().unwrap();
    let src = "---\nstack: [Rust]\nisBasedOn: ./gone.md\n---\n\n# Goal\n";
    let diags = run(&dir, src, ValidateOptions::default());
    assert!(messages(&diags).contains("gone.md"), "{}", messages(&diags));
}
