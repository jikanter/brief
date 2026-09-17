//! `brief validate`'s provenance pass — section 6 of
//! `docs/design/provenance-schema.md`.
//!
//! Everything here is a warning or a hint. Nothing in this vocabulary is an
//! error and nothing changes the exit code: these keys live in documents brief
//! did not write, and a tool that fails someone's build over a date in their
//! vault has overreached.
//!
//! The resolver in [`crate::provenance`] stays pure; this module is where the
//! filesystem and git come in.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::model::{Brief, Diagnostic, Severity};
use crate::provenance::{Provenance, Reference, resolve};

/// How far a `superseded_by` chain is followed before brief gives up. Deep
/// chains are a documentation smell; the cap is here so a malformed tree
/// cannot spin.
const MAX_SUPERSESSION_HOPS: usize = 8;

/// Collect the provenance diagnostics for a brief and its local context docs.
///
/// `base_dir` is the brief's directory; `brief_rel` is the brief's own path
/// relative to it, so the brief's frontmatter is checked like any other
/// document.
pub fn check(brief: &Brief, base_dir: &Path, brief_rel: &Path, hints: bool) -> Vec<Diagnostic> {
    let mut out = Vec::new();

    // The brief's own provenance, then each context doc that exists locally.
    let mut documents: Vec<PathBuf> = vec![base_dir.join(brief_rel)];
    documents.extend(
        brief
            .frontmatter
            .context
            .iter()
            .filter(|entry| !is_url(entry))
            .map(|entry| base_dir.join(entry)),
    );

    let brief_prov = read(&documents[0]);

    for path in &documents {
        // A context path that does not resolve is already reported by the
        // existing "Context file not found" check. Do not pile on.
        let Some(prov) = read(path) else {
            continue;
        };
        check_one(&prov, path, base_dir, &mut out);

        if hints && path != &documents[0] {
            hint_newer_than_brief(
                &prov,
                path,
                brief_prov.as_ref(),
                &documents[0],
                base_dir,
                &mut out,
            );
        }
    }

    out
}

/// Read and resolve one document, or `None` when it is not a readable file.
fn read(path: &Path) -> Option<Provenance> {
    if !path.is_file() {
        return None;
    }
    std::fs::read_to_string(path).ok().map(|c| resolve(&c))
}

fn check_one(prov: &Provenance, path: &Path, base_dir: &Path, out: &mut Vec<Diagnostic>) {
    let name = display(path, base_dir);

    for note in &prov.disagreements {
        warn(out, format!("{name}: {note}"));
    }

    if prov.modified_needs_offset {
        warn(
            out,
            format!(
                "{name}: brief wrote `metadata.brief.dateModified` without an explicit \
                 UTC offset; that is a bug in brief, not in the document"
            ),
        );
    }

    if let Some(Reference::Path(target)) = &prov.is_based_on
        && !resolve_sibling(path, target).is_file()
    {
        warn(
            out,
            format!("{name}: `isBasedOn` points at a missing file: {target}"),
        );
    }

    check_supersession(prov, path, base_dir, out);
    check_git_drift(prov, path, base_dir, out);
}

/// Follow `superseded_by` to the end of its chain, reporting a dangling hop or
/// a cycle on the way.
fn check_supersession(prov: &Provenance, path: &Path, base_dir: &Path, out: &mut Vec<Diagnostic>) {
    let Some(Reference::Path(first)) = &prov.superseded_by else {
        return;
    };
    let name = display(path, base_dir);

    let mut seen = vec![path.to_path_buf()];
    let mut current = resolve_sibling(path, first);
    let mut hops = 0;

    loop {
        if seen.contains(&current) {
            warn(
                out,
                format!(
                    "{name}: `superseded_by` forms a cycle at {}",
                    display(&current, base_dir)
                ),
            );
            return;
        }
        if !current.is_file() {
            warn(
                out,
                format!(
                    "{name}: `superseded_by` points at a missing file: {}",
                    display(&current, base_dir)
                ),
            );
            return;
        }

        hops += 1;
        if hops > MAX_SUPERSESSION_HOPS {
            warn(
                out,
                format!(
                    "{name}: `superseded_by` chain is longer than {MAX_SUPERSESSION_HOPS} hops; \
                     stopping at {}",
                    display(&current, base_dir)
                ),
            );
            return;
        }

        seen.push(current.clone());
        match read(&current).and_then(|p| match p.superseded_by {
            Some(Reference::Path(next)) => Some(resolve_sibling(&current, &next)),
            _ => None,
        }) {
            Some(next) => current = next,
            None => break,
        }
    }

    warn(
        out,
        format!(
            "{name} is superseded; the current document is {}",
            display(&current, base_dir)
        ),
    );
}

/// Warn when a human-set `dateModified` is an earlier calendar day than the
/// file's last commit — the field has gone stale against the content.
///
/// Days, not instants: a human sets the date and then commits, so the commit is
/// always slightly later. Skipped outside a work tree and for untracked files.
/// Brief's own stamp is governed by the content-change rule instead.
fn check_git_drift(prov: &Provenance, path: &Path, base_dir: &Path, out: &mut Vec<Diagnostic>) {
    if prov.brief_written {
        return;
    }
    let Some(declared) = prov.modified_day() else {
        return;
    };
    let Some(committed) = last_commit_day(path) else {
        return;
    };
    if declared < committed {
        warn(
            out,
            format!(
                "{}: `dateModified` says {declared} but the file was last committed {committed}",
                display(path, base_dir)
            ),
        );
    }
}

/// The last commit day for a file, or `None` outside a work tree, without git,
/// or for a path git does not track.
fn last_commit_day(path: &Path) -> Option<String> {
    let dir = path.parent()?;
    let out = Command::new("git")
        .args(["log", "-1", "--format=%cs", "--"])
        .arg(path.file_name()?)
        .current_dir(dir)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let day = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (!day.is_empty()).then_some(day)
}

/// Opt-in: the brief may rest on a doc that has moved since it was written.
///
/// Off by default because a long-lived brief trips it on every doc edit. The
/// brief's side of the comparison is its own `dateModified` if set, else its
/// last commit day; with neither, there is nothing to compare.
fn hint_newer_than_brief(
    prov: &Provenance,
    path: &Path,
    brief_prov: Option<&Provenance>,
    brief_path: &Path,
    base_dir: &Path,
    out: &mut Vec<Diagnostic>,
) {
    let Some(doc_day) = prov.modified_day() else {
        return;
    };
    let Some(brief_day) = brief_prov
        .and_then(Provenance::modified_day)
        .or_else(|| last_commit_day(brief_path))
    else {
        return;
    };

    if doc_day > brief_day {
        out.push(Diagnostic {
            severity: Severity::Hint,
            message: format!(
                "{} changed {doc_day}, after this brief ({brief_day}); its assumptions may be stale",
                display(path, base_dir)
            ),
        });
    }
}

fn warn(out: &mut Vec<Diagnostic>, message: String) {
    out.push(Diagnostic {
        severity: Severity::Warning,
        message,
    });
}

/// A reference is relative to the document that carries it.
fn resolve_sibling(document: &Path, target: &str) -> PathBuf {
    match document.parent() {
        Some(dir) => dir.join(target),
        None => PathBuf::from(target),
    }
}

/// Paths are reported relative to the brief where possible, so messages match
/// what the author typed.
fn display(path: &Path, base_dir: &Path) -> String {
    path.strip_prefix(base_dir)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn is_url(raw: &str) -> bool {
    raw.split_once("://").is_some_and(|(scheme, rest)| {
        !rest.is_empty()
            && scheme
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '.' | '-'))
    })
}
