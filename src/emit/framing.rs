//! Constraint framing — the emit layer is a prompt-engineering problem, not a
//! template-rendering one. This is the single source of truth for how brief's
//! human taxonomy (Hard / Soft / Ask First) is reframed into the imperative
//! verbs LLMs have strong priors on (NEVER / MUST / AVOID / PREFER / STOP). The
//! `prompt`, `claude`, and `xml` targets all render through these functions so
//! the framing convention stays identical across them.
//!
//! Three principles, drawn from `docs/analysis/phase2-synthesis.md` §P0 and
//! `docs/analysis/emit-quality-refinements.md` §1–2:
//!
//! 1. **Idempotent.** Text that already opens with an imperative verb
//!    (`MUST:`, `NEVER:`, `Do not …`, `Prefer …`, `STOP …`) is returned
//!    unchanged — never double-prefixed. This lets authors hand-write framing
//!    and lets `--install` round-trip already-emitted content safely.
//! 2. **Polarity-aware.** Hard constraints are not uniform. Prohibitions
//!    ("no lodash") map to `NEVER:`; positive requirements ("all endpoints
//!    return JSON") map to `MUST:`.
//! 3. **Convention bucket.** A standing convention ("follow the existing
//!    module layout") has no implied adversary, so it renders as a plain
//!    imperative with no prefix — applying `MUST:`/`NEVER:` inflates apparent
//!    weight and trains readers to skim. The trigger set is deliberately
//!    conservative (declarative verbs only), because a keyword scan cannot
//!    separate convention-"use" ("use thiserror") from requirement-"use"
//!    ("use `Result<T, AppError>`"); ambiguous leads fall through to `MUST:`.

/// Preamble emitted above a sacred-region list. "Under any circumstances"
/// removes exception ambiguity; "STOP and report" gives the model an action to
/// take instead of pure suppression, which is more reliable than prohibition
/// alone.
pub const SACRED_PREAMBLE: &str = "The following files and directories must not be modified under any circumstances. If a task requires changes to these paths, STOP and report the conflict.";

/// Semantic polarity of a hard constraint, inferred from its text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Polarity {
    /// "never use `unsafe`", "no lodash" — an alternative is being suppressed.
    Prohibition,
    /// "all public functions must return `Result<T, AppError>`" — the default.
    Requirement,
    /// "follow the existing module layout" — a standing convention.
    Convention,
}

/// Classify a hard constraint by polarity via a leading-keyword scan.
pub fn classify(constraint: &str) -> Polarity {
    if is_prohibition(constraint) {
        Polarity::Prohibition
    } else if is_convention(constraint) {
        Polarity::Convention
    } else {
        Polarity::Requirement
    }
}

/// Frame a Hard constraint with the imperative verb matching its polarity.
///
/// Already-imperative text is passed through unchanged; conventions render
/// plain; prohibitions get `NEVER:`; everything else gets `MUST:`.
pub fn frame_hard(constraint: &str) -> String {
    if has_imperative_prefix(constraint) {
        return constraint.to_string();
    }
    match classify(constraint) {
        Polarity::Prohibition => format!("NEVER: {constraint}"),
        Polarity::Convention => constraint.to_string(),
        Polarity::Requirement => format!("MUST: {constraint}"),
    }
}

/// Frame a Soft constraint as a weighted preference.
///
/// Already-imperative text passes through; soft prohibitions get `AVOID:`;
/// everything else gets `PREFER:`.
pub fn frame_soft(constraint: &str) -> String {
    if has_imperative_prefix(constraint) {
        return constraint.to_string();
    }
    if is_prohibition(constraint) {
        format!("AVOID: {constraint}")
    } else {
        format!("PREFER: {constraint}")
    }
}

/// Frame an Ask-First constraint as an interruption directive.
pub fn frame_ask_first(constraint: &str) -> String {
    if constraint
        .trim_start()
        .to_ascii_lowercase()
        .starts_with("stop")
    {
        return constraint.to_string();
    }
    format!("STOP and confirm before: {constraint}")
}

/// True if the constraint already begins with one of the imperative verbs the
/// emitter would otherwise prepend — so it should not be double-prefixed.
fn has_imperative_prefix(text: &str) -> bool {
    let lower = text.trim_start().to_ascii_lowercase();
    const PREFIXES: &[&str] = &[
        "never:", "never ", "must:", "must ", "do not ", "don't ", "dont ", "avoid:", "avoid ",
        "prefer:", "prefer ", "stop:", "stop ",
    ];
    PREFIXES.iter().any(|p| lower.starts_with(p))
}

/// Heuristic polarity check: does this constraint read as a prohibition?
fn is_prohibition(text: &str) -> bool {
    let lower = text.trim_start().to_ascii_lowercase();
    const PROHIBITION_PREFIXES: &[&str] = &[
        "no ",
        "not ",
        "never ",
        "don't ",
        "dont ",
        "do not ",
        "avoid ",
        "disallow ",
        "cannot ",
        "can not ",
        "must not ",
    ];
    PROHIBITION_PREFIXES.iter().any(|p| lower.starts_with(p))
}

/// Heuristic: does this constraint read as a standing convention (declarative,
/// no implied adversary)? Deliberately narrow to avoid misclassifying
/// requirements.
fn is_convention(text: &str) -> bool {
    let lower = text.trim_start().to_ascii_lowercase();
    const CONVENTION_PREFIXES: &[&str] = &["follow ", "stick to ", "adhere to ", "conform to "];
    CONVENTION_PREFIXES.iter().any(|c| lower.starts_with(c))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_prohibitions() {
        assert_eq!(classify("No lodash"), Polarity::Prohibition);
        assert_eq!(classify("Never use `unsafe`"), Polarity::Prohibition);
        assert_eq!(classify("Do not break compat"), Polarity::Prohibition);
        assert_eq!(classify("Avoid global state"), Polarity::Prohibition);
    }

    #[test]
    fn classifies_requirements() {
        assert_eq!(
            classify("All public functions must return Result<T, AppError>"),
            Polarity::Requirement
        );
        assert_eq!(
            classify("Format takes less than sixty seconds to author"),
            Polarity::Requirement
        );
        // Ambiguous "use" falls through to requirement, not convention.
        assert_eq!(classify("Use `Result<T, AppError>`"), Polarity::Requirement);
    }

    #[test]
    fn classifies_conventions() {
        assert_eq!(
            classify("Follow the existing module layout"),
            Polarity::Convention
        );
        assert_eq!(classify("Adhere to the style guide"), Polarity::Convention);
        assert_eq!(classify("Stick to stdlib only"), Polarity::Convention);
    }

    #[test]
    fn frame_hard_prohibition_uses_never() {
        assert_eq!(frame_hard("No lodash"), "NEVER: No lodash");
    }

    #[test]
    fn frame_hard_requirement_uses_must() {
        assert_eq!(
            frame_hard("All public functions must return Result"),
            "MUST: All public functions must return Result"
        );
    }

    #[test]
    fn frame_hard_convention_has_no_prefix() {
        assert_eq!(
            frame_hard("Follow the existing module layout"),
            "Follow the existing module layout"
        );
    }

    #[test]
    fn frame_hard_passes_through_already_imperative_text() {
        // Already-imperative phrasings are not double-prefixed.
        assert_eq!(frame_hard("Do not push to main"), "Do not push to main");
        assert_eq!(frame_hard("MUST: ship it"), "MUST: ship it");
        assert_eq!(
            frame_hard("Never block the event loop"),
            "Never block the event loop"
        );
    }

    #[test]
    fn frame_soft_uses_prefer_or_avoid() {
        assert_eq!(
            frame_soft("async over threads"),
            "PREFER: async over threads"
        );
        assert_eq!(
            frame_soft("No global mutable state"),
            "AVOID: No global mutable state"
        );
        // Already-imperative soft text passes through.
        assert_eq!(frame_soft("Prefer Yjs"), "Prefer Yjs");
    }

    #[test]
    fn frame_ask_first_uses_stop() {
        assert_eq!(
            frame_ask_first("Schema changes"),
            "STOP and confirm before: Schema changes"
        );
        // Already starts with STOP — passed through.
        assert_eq!(
            frame_ask_first("STOP: schema changes"),
            "STOP: schema changes"
        );
    }

    #[test]
    fn framing_is_case_insensitive() {
        assert_eq!(classify("FOLLOW the layout"), Polarity::Convention);
        assert_eq!(frame_hard("DO NOT panic"), "DO NOT panic");
    }
}
