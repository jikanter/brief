//! Constraint framing: the prompt-engineering canonicalization pass (P6).
//!
//! Phase 2's core emit thesis is that the emitter is a prompt-engineering
//! problem, not a templating one — *how* a constraint is framed changes how an
//! agent complies. This module is the canonicalization step: it reads the
//! polarity of a constraint from its own words and routes it to the imperative
//! verb that matches.
//!
//! The refinement over "prefix every hard constraint with NEVER" (see
//! docs/analysis/emit-quality-refinements.md §1–§2) is that hard constraints are
//! not semantically uniform:
//!
//! - **Prohibition** ("do not break the API", "never use `unsafe`") → `NEVER:` —
//!   there is an alternative being actively suppressed.
//! - **Requirement** ("all public fns return `Result`", "must pass CI") →
//!   `MUST:` — a positive obligation.
//! - **Convention** ("use `thiserror`", "prefer small commits") → rendered
//!   plain — a standing decision with no implied adversary; dressing it as
//!   `NEVER`/`MUST` inflates its apparent weight and trains readers to skim.
//!
//! Rendering a requirement as `NEVER:` is actively misleading; matching the verb
//! to the polarity is the whole point.

/// Preamble emitted above a sacred-region list. "Under any circumstances"
/// removes exception ambiguity; "STOP and report" gives the model an action to
/// take instead of pure suppression, which is more reliable than prohibition
/// alone.
pub const SACRED_PREAMBLE: &str = "The following files and directories must not be modified under any circumstances. If a task requires changes to these paths, STOP and report the conflict.";

/// Canonical imperative prefixes the emitter itself produces. Text that already
/// opens with one of these is passed through unchanged so authors can hand-write
/// framing and so already-emitted content round-trips through `--install`
/// without being double-prefixed ("MUST: NEVER: ...").
const FRAMED_PREFIXES: [&str; 5] = ["never:", "must:", "avoid:", "prefer:", "stop:"];

/// True if the constraint already opens with a canonical imperative prefix.
fn already_framed(text: &str) -> bool {
    let lower = text.trim_start().to_lowercase();
    FRAMED_PREFIXES.iter().any(|p| lower.starts_with(p))
}

/// The semantic polarity of a hard constraint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Polarity {
    /// An alternative is being suppressed ("do not", "never", "avoid", "no X").
    Prohibition,
    /// A positive obligation ("must", "all/every X must", "ensure", "require").
    Requirement,
    /// A standing decision stated declaratively ("use", "prefer", "follow").
    Convention,
}

/// Leading tokens that mark a prohibition. Order matters only for stripping.
const PROHIBITION_LEADS: [&str; 5] = ["do not ", "don't ", "never ", "avoid ", "no "];

/// Leading verbs that mark a declarative convention (no implied adversary).
const CONVENTION_LEADS: [&str; 8] = [
    "use ",
    "prefer ",
    "follow ",
    "keep ",
    "favor ",
    "stick to ",
    "adopt ",
    "default to ",
];

/// Classify a hard constraint's polarity from its wording.
pub fn classify(constraint: &str) -> Polarity {
    let lower = constraint.trim().to_lowercase();

    if PROHIBITION_LEADS.iter().any(|p| lower.starts_with(p)) {
        return Polarity::Prohibition;
    }
    if CONVENTION_LEADS.iter().any(|c| lower.starts_with(c)) {
        return Polarity::Convention;
    }
    Polarity::Requirement
}

/// Render a hard constraint with the imperative verb matching its polarity.
///
/// - Prohibition → `NEVER: <rest>` (the leading negation is folded into the
///   `NEVER:` so we never produce "NEVER: never use ...").
/// - Requirement → `MUST: <rest>` (a leading "must " is likewise folded in).
/// - Convention → the text unchanged.
pub fn frame_hard(constraint: &str) -> String {
    let trimmed = constraint.trim();
    if already_framed(trimmed) {
        return trimmed.to_string();
    }
    match classify(trimmed) {
        Polarity::Prohibition => {
            let rest = strip_leading_ci(trimmed, &PROHIBITION_LEADS);
            if rest.is_empty() {
                format!("NEVER: {trimmed}")
            } else {
                format!("NEVER: {rest}")
            }
        }
        Polarity::Requirement => {
            let rest = strip_leading_ci(trimmed, &["must ", "must: "]);
            if rest.is_empty() {
                format!("MUST: {trimmed}")
            } else {
                format!("MUST: {rest}")
            }
        }
        Polarity::Convention => trimmed.to_string(),
    }
}

/// Render a soft constraint as a weighted preference (`PREFER: <rest>`).
///
/// A leading "prefer " is folded in so we never produce "PREFER: prefer ...".
pub fn frame_soft(constraint: &str) -> String {
    let trimmed = constraint.trim();
    if already_framed(trimmed) {
        return trimmed.to_string();
    }
    let rest = strip_leading_ci(trimmed, &["prefer ", "prefer: "]);
    if rest.is_empty() {
        format!("PREFER: {trimmed}")
    } else {
        format!("PREFER: {rest}")
    }
}

/// Render an ask-first constraint as an interruption directive.
pub fn frame_ask_first(constraint: &str) -> String {
    let trimmed = constraint.trim();
    if already_framed(trimmed) {
        return trimmed.to_string();
    }
    let c = trimmed.trim_end_matches('.');
    format!("STOP and confirm with the user before: {c}")
}

/// A compact ask-first form for the attention anchor (line budget is tight).
pub fn frame_ask_first_short(constraint: &str) -> String {
    let c = constraint.trim().trim_end_matches('.');
    format!("STOP before: {c}")
}

/// Render the scope of a constraint as a backtick-wrapped, comma-joined clause
/// (`` `src/ui/**`, `src/api/**` ``), or `None` when the constraint is
/// project-wide. Used by emitters with no native glob-frontmatter home to keep
/// scoped constraints non-destructive — the scope survives as prose.
pub fn scope_clause(c: &crate::model::Constraint) -> Option<String> {
    if c.scope.is_empty() {
        return None;
    }
    Some(
        c.scope
            .iter()
            .map(|g| format!("`{g}`"))
            .collect::<Vec<_>>()
            .join(", "),
    )
}

/// Prefix an already-rendered constraint line with its scope clause, when the
/// constraint is scoped. Project-wide constraints pass through unchanged so
/// existing emit output is byte-for-byte stable.
pub fn with_scope(rendered: &str, c: &crate::model::Constraint) -> String {
    match scope_clause(c) {
        Some(clause) => format!("When working in {clause}: {rendered}"),
        None => rendered.to_string(),
    }
}

/// Strip the first matching prefix (case-insensitively) and return the
/// remainder, preserving the remainder's original casing. Returns the input
/// unchanged if no prefix matches.
fn strip_leading_ci(text: &str, prefixes: &[&str]) -> String {
    let lower = text.to_lowercase();
    for p in prefixes {
        if lower.starts_with(p) {
            return text[p.len()..].trim_start().to_string();
        }
    }
    text.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_prohibitions() {
        assert_eq!(classify("Do not break the API"), Polarity::Prohibition);
        assert_eq!(classify("never use unsafe"), Polarity::Prohibition);
        assert_eq!(classify("Avoid global state"), Polarity::Prohibition);
        assert_eq!(classify("No lodash"), Polarity::Prohibition);
        assert_eq!(classify("Don't refactor auth"), Polarity::Prohibition);
    }

    #[test]
    fn classifies_conventions() {
        assert_eq!(classify("Use thiserror for errors"), Polarity::Convention);
        assert_eq!(classify("Prefer small commits"), Polarity::Convention);
        assert_eq!(classify("Follow the existing style"), Polarity::Convention);
    }

    #[test]
    fn classifies_requirements() {
        assert_eq!(
            classify("All public functions return Result"),
            Polarity::Requirement
        );
        assert_eq!(classify("Must pass CI"), Polarity::Requirement);
        assert_eq!(classify("Ensure idempotency"), Polarity::Requirement);
    }

    #[test]
    fn frame_prohibition_folds_negation_into_never() {
        assert_eq!(frame_hard("Do not break the API"), "NEVER: break the API");
        assert_eq!(frame_hard("never use unsafe"), "NEVER: use unsafe");
        assert_eq!(frame_hard("Avoid global state"), "NEVER: global state");
    }

    #[test]
    fn frame_requirement_uses_must_without_doubling() {
        assert_eq!(frame_hard("Must pass CI"), "MUST: pass CI");
        assert_eq!(
            frame_hard("All public functions return Result"),
            "MUST: All public functions return Result"
        );
    }

    #[test]
    fn frame_convention_is_left_plain() {
        // The whole point: a standing convention is not dressed as adversarial.
        assert_eq!(
            frame_hard("Use thiserror for errors"),
            "Use thiserror for errors"
        );
        assert!(!frame_hard("Use thiserror for errors").contains("NEVER"));
        assert!(!frame_hard("Use thiserror for errors").contains("MUST"));
    }

    #[test]
    fn frame_soft_uses_prefer_without_doubling() {
        assert_eq!(
            frame_soft("Yjs over Automerge"),
            "PREFER: Yjs over Automerge"
        );
        assert_eq!(frame_soft("Prefer small commits"), "PREFER: small commits");
    }

    #[test]
    fn already_framed_text_is_not_double_prefixed() {
        // Canonical prefixes the emitter produces round-trip unchanged.
        assert_eq!(frame_hard("MUST: ship it"), "MUST: ship it");
        assert_eq!(frame_hard("NEVER: leak PII"), "NEVER: leak PII");
        assert_eq!(frame_soft("PREFER: small modules"), "PREFER: small modules");
        assert_eq!(
            frame_ask_first("STOP: schema changes"),
            "STOP: schema changes"
        );
    }

    #[test]
    fn scope_clause_backticks_and_joins() {
        use crate::model::Constraint;
        let c = Constraint::scoped("x", vec!["src/ui/**".into(), "src/api/**".into()]);
        assert_eq!(scope_clause(&c).unwrap(), "`src/ui/**`, `src/api/**`");
    }

    #[test]
    fn unscoped_constraint_passes_through_unchanged() {
        use crate::model::Constraint;
        let c = Constraint::new("MUST: pass CI");
        assert_eq!(scope_clause(&c), None);
        assert_eq!(with_scope("MUST: pass CI", &c), "MUST: pass CI");
    }

    #[test]
    fn with_scope_prefixes_scoped_constraint() {
        use crate::model::Constraint;
        let c = Constraint::scoped("WCAG 2.1 AA", vec!["src/ui/**".into()]);
        assert_eq!(
            with_scope("WCAG 2.1 AA", &c),
            "When working in `src/ui/**`: WCAG 2.1 AA"
        );
    }

    #[test]
    fn frame_ask_first_is_an_interruption() {
        assert_eq!(
            frame_ask_first("Changing the DB schema."),
            "STOP and confirm with the user before: Changing the DB schema"
        );
        assert_eq!(
            frame_ask_first_short("Changing the DB schema"),
            "STOP before: Changing the DB schema"
        );
    }
}
