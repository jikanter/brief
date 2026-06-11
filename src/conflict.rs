//! Constraint conflict detection for `--install` (P6).
//!
//! When brief injects its section into an existing CLAUDE.md / AGENTS.md /
//! CONVENTIONS.md, a brief constraint can directly contradict an instruction the
//! file already carries — institutional knowledge developed by trial and error.
//! Full semantic contradiction detection is hard, but a cheap, low-false-positive
//! heuristic is tractable: a real conflict is two rules about the *same subject*
//! with *opposite polarity* — "use tabs" in the brief vs. "never use tabs" in the
//! host file.
//!
//! This is the linker's symbol-redefinition warning: same name, incompatible
//! definition. We reuse [`crate::framing::classify`] for polarity so the check
//! only fires when one side prohibits and the other asserts — which keeps the
//! signal high. It warns; it never blocks the install.

use std::collections::HashSet;

use crate::emit::markers::find_marker_pairs;
use crate::framing::{Polarity, classify};
use crate::model::Brief;

/// A detected potential conflict between a brief constraint and a line already
/// present in the host file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Conflict {
    pub brief_constraint: String,
    pub existing_line: String,
}

/// Words that carry polarity or are too generic to count as a shared *subject*.
/// Excluding them means token overlap measures what a rule is *about*, leaving
/// polarity to [`classify`].
const SKIP_WORDS: &[&str] = &[
    "never", "avoid", "must", "should", "shall", "always", "prefer", "follow", "ensure", "require",
    "required", "using", "used", "this", "that", "with", "when", "your", "their", "from", "have",
    "into", "onto", "only", "also", "than", "then", "where", "which", "while", "they", "them",
    "would", "could", "before", "after", "under", "over",
];

/// Detect potential conflicts between the brief's hard + ask-first constraints
/// and the instructions already in `host_text`.
///
/// brief's own previously-installed section (between `<brief:generated>` markers)
/// is excluded so re-installs never flag the brief against itself.
pub fn detect_conflicts(brief: &Brief, host_text: &str) -> Vec<Conflict> {
    let scannable = strip_brief_region(host_text);

    // Candidate host lines: bullet/instruction lines, not headings or prose
    // paragraphs. Each carries its subject tokens and polarity.
    let host_lines: Vec<(String, HashSet<String>, Polarity)> = scannable
        .lines()
        .map(str::trim)
        .filter(|l| is_instruction_line(l))
        .map(|l| {
            let clean = clean_line(l);
            (clean.clone(), subject_tokens(&clean), classify(&clean))
        })
        .filter(|(_, toks, _)| !toks.is_empty())
        .collect();

    let mut conflicts = Vec::new();
    let candidates = brief
        .constraints
        .hard
        .iter()
        .chain(brief.constraints.ask_first.iter());

    for constraint in candidates {
        let c_tokens = subject_tokens(constraint);
        if c_tokens.is_empty() {
            continue;
        }
        let c_polarity = classify(constraint);

        for (line, l_tokens, l_polarity) in &host_lines {
            if opposite_polarity(c_polarity, *l_polarity)
                && c_tokens.intersection(l_tokens).count() >= 1
            {
                conflicts.push(Conflict {
                    brief_constraint: constraint.clone(),
                    existing_line: line.clone(),
                });
            }
        }
    }

    conflicts
}

/// Two rules conflict only when exactly one of them is a prohibition: "use X"
/// vs "never use X". Two positives (or two prohibitions) about the same subject
/// reinforce rather than contradict.
fn opposite_polarity(a: Polarity, b: Polarity) -> bool {
    (a == Polarity::Prohibition) ^ (b == Polarity::Prohibition)
}

/// Remove the `<brief:generated>` region(s) so a re-install doesn't compare the
/// brief against its own prior output.
fn strip_brief_region(text: &str) -> String {
    let pairs = find_marker_pairs(text);
    if pairs.is_empty() {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    let mut cursor = 0;
    for pair in pairs {
        out.push_str(&text[cursor..pair.pair_start]);
        cursor = pair.pair_end;
    }
    out.push_str(&text[cursor..]);
    out
}

/// Is this a line that states a rule (a bullet or a short imperative), as
/// opposed to a heading or a long prose paragraph?
fn is_instruction_line(line: &str) -> bool {
    if line.is_empty() || line.starts_with('#') || line.starts_with('<') {
        return false;
    }
    // Long lines read as prose, not rules; skip to avoid noise.
    if line.chars().count() > 160 {
        return false;
    }
    line.starts_with('-') || line.starts_with('*') || line.len() <= 120
}

/// Strip Markdown bullet markers and brief's own imperative prefixes so the
/// subject and polarity are read from the rule text itself.
fn clean_line(line: &str) -> String {
    let mut s = line.trim();
    for lead in ['-', '*', '+'] {
        s = s.trim_start_matches(lead).trim_start();
    }
    for prefix in [
        "**IMPORTANT:**",
        "IMPORTANT:",
        "NEVER:",
        "MUST:",
        "PREFER:",
        "STOP and confirm with the user before:",
        "STOP before:",
    ] {
        if let Some(rest) = s.strip_prefix(prefix) {
            s = rest.trim_start();
        }
    }
    s.trim().to_string()
}

/// The set of subject tokens for a rule: lowercase alphanumeric words of length
/// ≥4, excluding polarity and generic words.
fn subject_tokens(text: &str) -> HashSet<String> {
    let skip: HashSet<&str> = SKIP_WORDS.iter().copied().collect();
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() >= 4 && !skip.contains(w))
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;

    fn brief_with_hard(hard: Vec<&str>) -> Brief {
        Brief {
            frontmatter: Frontmatter::default(),
            goal: "Goal".into(),
            identity: None,
            constraints: Constraints {
                hard: hard.into_iter().map(String::from).collect(),
                soft: vec![],
                ask_first: vec![],
            },
            sacred: vec![],
            assumptions: vec![],
            deliverable: None,
            unknown_sections: vec![],
        }
    }

    #[test]
    fn flags_opposite_polarity_same_subject() {
        let brief = brief_with_hard(vec!["Use tabs for indentation"]);
        let host = "# Style\n\n- Never use tabs for indentation\n";
        let conflicts = detect_conflicts(&brief, host);
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].brief_constraint, "Use tabs for indentation");
    }

    #[test]
    fn does_not_flag_same_polarity() {
        // Both positive about the same subject — reinforcing, not conflicting.
        let brief = brief_with_hard(vec!["Use Result for error handling"]);
        let host = "- Use Result types everywhere\n";
        assert!(detect_conflicts(&brief, host).is_empty());
    }

    #[test]
    fn does_not_flag_unrelated_subjects() {
        let brief = brief_with_hard(vec!["Use tabs for indentation"]);
        let host = "- Never deploy on Fridays\n";
        assert!(detect_conflicts(&brief, host).is_empty());
    }

    #[test]
    fn ignores_brief_generated_region() {
        // A re-install must not flag the brief against its own prior output.
        let brief = brief_with_hard(vec!["Use tabs for indentation"]);
        let host = "<brief:generated>\n- NEVER: use tabs for indentation\n</brief:generated>\n";
        assert!(detect_conflicts(&brief, host).is_empty());
    }

    #[test]
    fn flags_ask_first_constraints_too() {
        let mut brief = brief_with_hard(vec![]);
        brief.constraints.ask_first = vec!["Use feature flags for rollout".into()];
        let host = "- Never use feature flags in this repo\n";
        assert_eq!(detect_conflicts(&brief, host).len(), 1);
    }

    #[test]
    fn skips_headings_and_prose() {
        let brief = brief_with_hard(vec!["Use tabs for indentation"]);
        // Heading line mentioning tabs must not be treated as a rule.
        let host = "## Never use tabs anywhere in the section about tabs\n";
        assert!(detect_conflicts(&brief, host).is_empty());
    }
}
