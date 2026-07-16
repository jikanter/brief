//! Context-window budget awareness for emitted briefings (phase2-synthesis P3).
//!
//! This module treats the emitter the way a model compiler treats codegen:
//!
//! - [`estimate_tokens`] is the **binary-size report** — how much of the
//!   context window an emitted briefing will occupy. Every token brief emits
//!   displaces conversation history, code, or tool output, and a bloated brief
//!   is re-paid on every turn it sits in the system prompt.
//! - [`compact`] is an **optimization pass over the Brief IR** — a dead-prose
//!   elimination that strips reference material and keeps only the load-bearing
//!   instructions, the same way `-Oz` drops everything not needed to run.
//! - [`Target`] thresholds are the **linker budget** — limits the build refuses
//!   to silently overrun. Brief warns; it never truncates.

use crate::model::{Brief, Frontmatter};

/// Estimate the token count of `text`.
///
/// Tokenizer-free by design. Brief makes no network calls and ships no model
/// files (see CLAUDE.md), so we approximate Anthropic/OpenAI BPE tokenizers
/// with the canonical ~4-characters-per-token rule. A naive whitespace split
/// would do, but it badly *under*-counts the punctuation- and symbol-dense
/// content briefs are full of — a backtick-wrapped glob like
/// `` `src/core/crdt-engine/**` `` is one "word" but many tokens. Counting
/// characters and dividing by four lands within a few percent of a real
/// tokenizer for mixed English + code + Markdown, which is all the budget
/// decision needs.
pub fn estimate_tokens(text: &str) -> usize {
    text.chars().count().div_ceil(4)
}

/// An emit target, used to select the budget thresholds that apply to its
/// output. Mirrors the CLI's emit targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Prompt,
    Readme,
    Claude,
    AgentsMd,
    Cursor,
    Copilot,
    Windsurf,
    Aider,
    Xml,
    Json,
}

impl Target {
    /// Human-facing label used in budget messages.
    pub fn label(self) -> &'static str {
        match self {
            Target::Prompt => "prompt",
            Target::Claude => "claude",
            Target::Readme => "readme",
            Target::AgentsMd => "agents-md",
            Target::Cursor => "cursor",
            Target::Copilot => "copilot",
            Target::Windsurf => "windsurf",
            Target::Aider => "aider",
            Target::Xml => "xml",
            Target::Json => "json",
        }
    }

    /// Soft token budget for this target's emitted section. Exceeding it warns
    /// but never blocks. `None` means tokens are not tracked for the target.
    ///
    /// A raw `prompt` goes straight into an API system prompt where every token
    /// is precious, so its budget is tight (500). The file-based targets
    /// (`claude`, `agents-md`, `cursor`, ...) live in a project instruction file
    /// a human also reads, where a little more prose is expected (2000). `json`
    /// is consumed by tooling, not a model context window, so it has no budget.
    pub fn token_threshold(self) -> Option<usize> {
        match self {
            Target::Prompt => Some(500),
            Target::Claude
            | Target::Readme
            | Target::AgentsMd
            | Target::Cursor
            | Target::Copilot
            | Target::Windsurf
            | Target::Aider
            | Target::Xml => Some(2000),
            Target::Json => None,
        }
    }

    /// A hard character cap imposed by the target ecosystem itself, if any.
    ///
    /// Windsurf caps a workspace rule file at 12,000 characters (per the
    /// March 2026 integration audit — see docs/design/backends/windsurf/). This
    /// is a property of *their* loader, not a brief preference, so it is tracked
    /// separately from the soft token budget. Brief warns on overrun and never
    /// silently truncates.
    pub fn char_limit(self) -> Option<usize> {
        match self {
            Target::Windsurf => Some(12_000),
            _ => None,
        }
    }
}

/// The measured budget of an emitted section against its target's limits.
#[derive(Debug, Clone, Copy)]
pub struct BudgetReport {
    pub tokens: usize,
    pub chars: usize,
    pub token_threshold: Option<usize>,
    pub char_limit: Option<usize>,
}

impl BudgetReport {
    /// True when the estimated token count exceeds the target's soft budget.
    pub fn over_tokens(&self) -> bool {
        matches!(self.token_threshold, Some(t) if self.tokens > t)
    }

    /// True when the character count exceeds the target ecosystem's hard cap.
    pub fn over_chars(&self) -> bool {
        matches!(self.char_limit, Some(l) if self.chars > l)
    }
}

/// Measure an emitted `section` against `target`'s budget thresholds.
pub fn measure(section: &str, target: Target) -> BudgetReport {
    BudgetReport {
        tokens: estimate_tokens(section),
        chars: section.chars().count(),
        token_threshold: target.token_threshold(),
        char_limit: target.char_limit(),
    }
}

/// Reduce a Brief to its load-bearing essentials for `--compact` emit.
///
/// A dead-prose-elimination pass over the Brief IR: keep the goal, every
/// constraint tier, the sacred regions, and the deliverable; drop the stack,
/// context, assumptions, identity, and unknown passthrough sections — the
/// material that reads as reference prose rather than an instruction the agent
/// must obey.
///
/// Crucially this operates on the IR, not the rendered string, so emitters run
/// the reduced Brief through their normal code path: each target's register and
/// structure are preserved automatically, and the explanatory framing lines
/// ("Read these files for background", "Refer to these files...") disappear on
/// their own because they are gated on the fields this pass empties.
pub fn compact(brief: &Brief) -> Brief {
    Brief {
        frontmatter: Frontmatter::default(),
        goal: brief.goal.clone(),
        identity: None,
        constraints: brief.constraints.clone(),
        sacred: brief.sacred.clone(),
        assumptions: Vec::new(),
        deliverable: brief.deliverable.clone(),
        unknown_sections: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;

    #[test]
    fn estimate_tokens_empty_is_zero() {
        assert_eq!(estimate_tokens(""), 0);
    }

    #[test]
    fn estimate_tokens_rounds_up() {
        // 5 chars / 4 = 1.25 -> 2 (div_ceil)
        assert_eq!(estimate_tokens("abcde"), 2);
        // exactly 4 chars -> 1
        assert_eq!(estimate_tokens("abcd"), 1);
    }

    #[test]
    fn estimate_tokens_counts_unicode_scalars_not_bytes() {
        // "héllo" is 5 chars but 6 bytes; budget should track chars.
        assert_eq!(estimate_tokens("héllo"), estimate_tokens("hello"));
    }

    #[test]
    fn prompt_budget_is_tighter_than_file_targets() {
        assert_eq!(Target::Prompt.token_threshold(), Some(500));
        assert_eq!(Target::Claude.token_threshold(), Some(2000));
        assert_eq!(Target::AgentsMd.token_threshold(), Some(2000));
    }

    #[test]
    fn json_has_no_token_budget() {
        assert_eq!(Target::Json.token_threshold(), None);
    }

    #[test]
    fn only_windsurf_has_a_char_limit() {
        assert_eq!(Target::Windsurf.char_limit(), Some(12_000));
        assert_eq!(Target::Claude.char_limit(), None);
        assert_eq!(Target::Prompt.char_limit(), None);
    }

    #[test]
    fn measure_flags_over_token_budget() {
        // 2100 chars ~= 525 tokens, over the 500 prompt budget.
        let section = "x".repeat(2100);
        let report = measure(&section, Target::Prompt);
        assert!(report.over_tokens());
        assert!(!report.over_chars());
    }

    #[test]
    fn measure_under_budget_does_not_flag() {
        let report = measure("short brief", Target::Claude);
        assert!(!report.over_tokens());
        assert!(!report.over_chars());
    }

    #[test]
    fn measure_flags_over_windsurf_char_limit() {
        let section = "x".repeat(12_001);
        let report = measure(&section, Target::Windsurf);
        assert!(report.over_chars());
    }

    fn full_brief() -> Brief {
        Brief {
            frontmatter: Frontmatter {
                stack: vec!["Rust".into()],
                context: vec!["./docs/arch.md".into()],
                ..Default::default()
            },
            goal: "Add notifications".into(),
            identity: Some(Identity {
                heading: "Identity".into(),
                content: "A project".into(),
            }),
            constraints: Constraints {
                hard: vec!["Must pass CI".into()],
                soft: vec!["Prefer WebSocket".into()],
                ask_first: vec!["Schema changes".into()],
            },
            sacred: vec![SacredEntry {
                path: "src/auth/**".into(),
                reason: "Audited".into(),
                well_formed: true,
            }],
            assumptions: vec![Assumption {
                text: "Gateway scales".into(),
                validated: false,
                has_checkbox: true,
            }],
            deliverable: Some("Working system".into()),
            unknown_sections: vec![UnknownSection {
                heading: "Commands".into(),
                content: "- Build: `cargo build`".into(),
            }],
        }
    }

    #[test]
    fn compact_keeps_essentials() {
        let reduced = compact(&full_brief());
        assert_eq!(reduced.goal, "Add notifications");
        assert_eq!(reduced.constraints.hard, vec!["Must pass CI".to_string()]);
        assert_eq!(
            reduced.constraints.soft,
            vec!["Prefer WebSocket".to_string()]
        );
        assert_eq!(
            reduced.constraints.ask_first,
            vec!["Schema changes".to_string()]
        );
        assert_eq!(reduced.sacred.len(), 1);
        assert_eq!(reduced.deliverable.as_deref(), Some("Working system"));
    }

    #[test]
    fn compact_drops_reference_prose() {
        let reduced = compact(&full_brief());
        assert!(reduced.frontmatter.stack.is_empty());
        assert!(reduced.frontmatter.context.is_empty());
        assert!(reduced.assumptions.is_empty());
        assert!(reduced.unknown_sections.is_empty());
        assert!(reduced.identity.is_none());
    }

    #[test]
    fn compact_is_smaller_when_rendered() {
        use crate::emit::emit_prompt;
        let brief = full_brief();
        let full = emit_prompt(&brief);
        let compacted = emit_prompt(&compact(&brief));
        assert!(
            estimate_tokens(&compacted) < estimate_tokens(&full),
            "compact emit should be smaller: {} vs {}",
            estimate_tokens(&compacted),
            estimate_tokens(&full)
        );
    }
}
