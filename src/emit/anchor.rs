use crate::framing::{frame_ask_first_short, frame_hard};
use crate::model::Brief;

/// The maximum number of content lines in an anchor block. Attention decay in
/// long sessions is the problem the anchor addresses, so it must stay short
/// enough to re-inject cheaply — it carries only the highest-priority rules.
const MAX_LINES: usize = 5;

/// Emit a compact "attention anchor" — a 3–5 line summary of the highest-
/// priority constraints, suitable for periodic re-injection by an agent
/// framework to counter attention decay over a long multi-turn session (P6).
///
/// Unlike the full emit targets, the anchor is deliberately lossy: it keeps the
/// hard constraints (framed by polarity via [`crate::framing`]), then ask-first
/// items, then a one-line sacred summary, capped at [`MAX_LINES`]. Lower-priority
/// material (soft constraints, assumptions, prose) is intentionally dropped —
/// the anchor is a reminder, not a replacement for the full briefing.
///
/// Output is wrapped in `<brief:anchor>` tags so a framework can locate and
/// replace it between turns.
pub fn emit_anchor(brief: &Brief) -> String {
    let mut lines: Vec<String> = Vec::new();

    // Reserve the last slot for a sacred summary if there are sacred regions.
    let sacred_line = sacred_summary(brief);
    let rule_budget = MAX_LINES - usize::from(sacred_line.is_some());

    let rules = brief.constraints.hard.iter().map(|c| frame_hard(c)).chain(
        brief
            .constraints
            .ask_first
            .iter()
            .map(|c| frame_ask_first_short(c)),
    );
    for rule in rules.take(rule_budget) {
        lines.push(rule);
    }

    if let Some(s) = sacred_line {
        lines.push(s);
    }

    // Never emit an empty anchor: fall back to the goal as the single reminder.
    if lines.is_empty() {
        lines.push(format!("Goal: {}", brief.goal.trim()));
    }

    format!("<brief:anchor>\n{}\n</brief:anchor>\n", lines.join("\n"))
}

/// A single-line summary of sacred regions: up to three paths, then an ellipsis.
fn sacred_summary(brief: &Brief) -> Option<String> {
    if brief.sacred.is_empty() {
        return None;
    }
    let shown: Vec<&str> = brief
        .sacred
        .iter()
        .take(3)
        .map(|s| s.path.as_str())
        .collect();
    let mut summary = format!("Sacred (do not modify): {}", shown.join(", "));
    if brief.sacred.len() > 3 {
        summary.push_str(", …");
    }
    Some(summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;

    fn brief(hard: Vec<&str>, ask: Vec<&str>, sacred: Vec<&str>) -> Brief {
        Brief {
            frontmatter: Frontmatter::default(),
            goal: "Ship the thing".into(),
            identity: None,
            constraints: Constraints {
                hard: hard.into_iter().map(String::from).collect(),
                soft: vec!["Prefer small commits".into()],
                ask_first: ask.into_iter().map(String::from).collect(),
            },
            sacred: sacred
                .into_iter()
                .map(|p| SacredEntry {
                    path: p.into(),
                    reason: "r".into(),
                    well_formed: true,
                })
                .collect(),
            assumptions: vec![],
            deliverable: None,
            unknown_sections: vec![],
        }
    }

    #[test]
    fn wraps_in_anchor_tags() {
        let out = emit_anchor(&brief(vec!["Must pass CI"], vec![], vec![]));
        assert!(out.starts_with("<brief:anchor>\n"));
        assert!(out.trim_end().ends_with("</brief:anchor>"));
    }

    #[test]
    fn frames_hard_by_polarity() {
        let out = emit_anchor(&brief(
            vec!["Do not break the API", "Use thiserror"],
            vec![],
            vec![],
        ));
        assert!(out.contains("NEVER: break the API"));
        assert!(out.contains("Use thiserror")); // convention stays plain
        assert!(!out.contains("NEVER: Use thiserror"));
    }

    #[test]
    fn includes_ask_first_and_sacred() {
        let out = emit_anchor(&brief(
            vec!["Must pass CI"],
            vec!["Changing the schema"],
            vec!["src/auth/**"],
        ));
        assert!(out.contains("MUST: pass CI"));
        assert!(out.contains("STOP before: Changing the schema"));
        assert!(out.contains("Sacred (do not modify): src/auth/**"));
    }

    #[test]
    fn omits_soft_constraints_and_assumptions() {
        let out = emit_anchor(&brief(vec!["Must pass CI"], vec![], vec![]));
        assert!(!out.contains("Prefer small commits"));
    }

    #[test]
    fn caps_at_five_lines() {
        let out = emit_anchor(&brief(
            vec!["Must a", "Must b", "Must c", "Must d", "Must e", "Must f"],
            vec!["ask one"],
            vec!["src/x"],
        ));
        let content_lines = out
            .lines()
            .filter(|l| !l.starts_with("<brief:anchor>") && !l.starts_with("</brief:anchor>"))
            .filter(|l| !l.is_empty())
            .count();
        assert!(
            content_lines <= MAX_LINES,
            "got {content_lines} lines:\n{out}"
        );
        // The sacred summary always claims the reserved slot.
        assert!(out.contains("Sacred (do not modify): src/x"));
    }

    #[test]
    fn falls_back_to_goal_when_no_rules() {
        let out = emit_anchor(&brief(vec![], vec![], vec![]));
        assert!(out.contains("Goal: Ship the thing"));
    }

    #[test]
    fn truncates_many_sacred_paths() {
        let out = emit_anchor(&brief(vec![], vec![], vec!["a", "b", "c", "d", "e"]));
        assert!(out.contains("Sacred (do not modify): a, b, c, …"));
    }
}
