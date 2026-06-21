use crate::framing::{SACRED_PREAMBLE, frame_ask_first, frame_hard, frame_soft, with_scope};
use crate::model::Brief;

/// Emit a raw system prompt suitable for direct API use.
///
/// The `prompt` target occupies the highest-privilege position (a system
/// prompt) and is aggressively optimized for behavioral compliance rather than
/// human reading flow:
///
/// - **Framing:** constraints are rendered with imperative verbs (NEVER / MUST
///   / PREFER / STOP) via [`crate::framing`], which LLMs have far stronger
///   priors on than brief's Hard/Soft/Ask-First taxonomy.
/// - **Ordering:** hard constraints and sacred regions take the primacy
///   position (first ~15% of context, highest attention); the deliverable takes
///   the recency position (last, an action directive). Goal, stack, context,
///   soft/ask-first constraints, and assumptions sit in the middle.
///
/// See `docs/analysis/phase2-synthesis.md` §P0.
pub fn emit_prompt(brief: &Brief) -> String {
    let mut out = String::new();

    // --- Primacy: hard constraints + sacred regions ---
    if !brief.constraints.hard.is_empty() {
        out.push_str("HARD CONSTRAINTS:\n");
        for c in &brief.constraints.hard {
            out.push_str(&format!("- {}\n", with_scope(&frame_hard(c), c)));
        }
        out.push('\n');
    }

    if !brief.sacred.is_empty() {
        out.push_str(&format!("SACRED REGIONS:\n{SACRED_PREAMBLE}\n"));
        for entry in &brief.sacred {
            out.push_str(&format!("- {}: {}\n", entry.path, entry.reason));
        }
        out.push('\n');
    }

    // --- Task frame ---
    out.push_str(&format!("GOAL: {}\n\n", brief.goal));

    if !brief.frontmatter.stack.is_empty() {
        out.push_str(&format!(
            "STACK: {}\n\n",
            brief.frontmatter.stack.join(", ")
        ));
    }

    if !brief.frontmatter.context.is_empty() {
        out.push_str("REFERENCE CONTEXT:\n");
        for ctx in &brief.frontmatter.context {
            out.push_str(&format!("- {ctx}\n"));
        }
        out.push('\n');
    }

    // --- Middle: preferences are fine away from the attention extremes ---
    if !brief.constraints.soft.is_empty() {
        out.push_str("SOFT CONSTRAINTS:\n");
        for c in &brief.constraints.soft {
            out.push_str(&format!("- {}\n", with_scope(&frame_soft(c), c)));
        }
        out.push('\n');
    }

    if !brief.constraints.ask_first.is_empty() {
        out.push_str("ASK BEFORE PROCEEDING:\n");
        for c in &brief.constraints.ask_first {
            out.push_str(&format!("- {}\n", with_scope(&frame_ask_first(c), c)));
        }
        out.push('\n');
    }

    let unvalidated: Vec<_> = brief.assumptions.iter().filter(|a| !a.validated).collect();
    let validated: Vec<_> = brief.assumptions.iter().filter(|a| a.validated).collect();

    if !unvalidated.is_empty() {
        out.push_str("ASSUMPTIONS (UNVALIDATED):\n");
        for a in &unvalidated {
            out.push_str(&format!("- {}\n", a.text));
        }
        out.push('\n');
    }

    if !validated.is_empty() {
        out.push_str("ASSUMPTIONS (VALIDATED):\n");
        for a in &validated {
            out.push_str(&format!("- {}\n", a.text));
        }
        out.push('\n');
    }

    // Unknown sections (passthrough)
    for section in &brief.unknown_sections {
        out.push_str(&format!(
            "{}:\n{}\n\n",
            section.heading.to_uppercase(),
            section.content
        ));
    }

    // --- Recency: deliverable is the closing action directive ---
    if let Some(ref deliverable) = brief.deliverable {
        out.push_str("DELIVERABLE:\n");
        out.push_str(deliverable.trim_end());
        out.push('\n');
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;

    #[test]
    fn prompt_has_goal_and_stack() {
        let brief = Brief {
            frontmatter: Frontmatter {
                stack: vec!["Python".into(), "PostgreSQL".into()],
                ..Default::default()
            },
            goal: "Redesign pipeline".into(),
            identity: None,
            constraints: Constraints::default(),
            sacred: vec![],
            assumptions: vec![],
            deliverable: None,
            unknown_sections: vec![],
        };
        let output = emit_prompt(&brief);
        assert!(output.starts_with("GOAL: Redesign pipeline"));
        assert!(output.contains("STACK: Python, PostgreSQL"));
    }

    #[test]
    fn prompt_emits_unknown_sections() {
        let brief = Brief {
            frontmatter: Frontmatter::default(),
            goal: "Goal".into(),
            identity: None,
            constraints: Constraints::default(),
            sacred: vec![],
            assumptions: vec![],
            deliverable: None,
            unknown_sections: vec![UnknownSection {
                heading: "Commands".into(),
                content: "- Build: `cargo build`".into(),
            }],
        };
        let output = emit_prompt(&brief);
        assert!(output.contains("COMMANDS:\n- Build: `cargo build`"));
    }

    #[test]
    fn prompt_frames_hard_constraints_with_imperatives() {
        let brief = Brief {
            frontmatter: Frontmatter::default(),
            goal: "Goal".into(),
            identity: None,
            constraints: Constraints {
                hard: vec![
                    "No lodash".into(),
                    "All public functions must return Result".into(),
                    "Do not push to main".into(),
                ],
                ..Default::default()
            },
            sacred: vec![],
            assumptions: vec![],
            deliverable: None,
            unknown_sections: vec![],
        };
        let output = emit_prompt(&brief);
        // Prohibition → NEVER (negation lead folded in), requirement → MUST.
        assert!(output.contains("NEVER: lodash"));
        assert!(output.contains("MUST: All public functions must return Result"));
        assert!(output.contains("NEVER: push to main"));
        assert!(!output.contains("NEVER: Do not push to main"));
    }

    #[test]
    fn prompt_frames_soft_and_ask_first() {
        let brief = Brief {
            frontmatter: Frontmatter::default(),
            goal: "Goal".into(),
            identity: None,
            constraints: Constraints {
                hard: vec![],
                soft: vec!["async over threads".into()],
                ask_first: vec!["Schema changes".into()],
            },
            sacred: vec![],
            assumptions: vec![],
            deliverable: None,
            unknown_sections: vec![],
        };
        let output = emit_prompt(&brief);
        assert!(output.contains("PREFER: async over threads"));
        assert!(output.contains("STOP and confirm with the user before: Schema changes"));
    }

    #[test]
    fn prompt_sacred_has_preamble() {
        let brief = Brief {
            frontmatter: Frontmatter::default(),
            goal: "Goal".into(),
            identity: None,
            constraints: Constraints::default(),
            sacred: vec![SacredEntry {
                path: "src/auth/**".into(),
                reason: "Auth".into(),
                well_formed: true,
            }],
            assumptions: vec![],
            deliverable: None,
            unknown_sections: vec![],
        };
        let output = emit_prompt(&brief);
        assert!(output.contains("under any circumstances"));
        assert!(output.contains("STOP and report"));
    }

    #[test]
    fn prompt_orders_constraints_and_sacred_before_goal() {
        let brief = Brief {
            frontmatter: Frontmatter::default(),
            goal: "Build the thing".into(),
            identity: None,
            constraints: Constraints {
                hard: vec!["No unsafe code".into()],
                ..Default::default()
            },
            sacred: vec![SacredEntry {
                path: "db/migrations/**".into(),
                reason: "Immutable".into(),
                well_formed: true,
            }],
            assumptions: vec![],
            deliverable: Some("A working binary".into()),
            unknown_sections: vec![],
        };
        let output = emit_prompt(&brief);
        let hard_pos = output.find("NEVER: unsafe code").unwrap();
        let sacred_pos = output.find("db/migrations/**").unwrap();
        let goal_pos = output.find("Build the thing").unwrap();
        let deliverable_pos = output.find("A working binary").unwrap();
        // Hard constraints + sacred get primacy position, before the goal.
        assert!(hard_pos < goal_pos, "hard constraints should precede goal");
        assert!(sacred_pos < goal_pos, "sacred should precede goal");
        // Deliverable gets recency position — the final section.
        assert!(
            deliverable_pos > goal_pos,
            "deliverable should come after goal"
        );
        assert_eq!(
            output.trim_end().rfind("A working binary"),
            Some(deliverable_pos),
            "deliverable should be the final section"
        );
    }

    #[test]
    fn prompt_separates_validated_assumptions() {
        let brief = Brief {
            frontmatter: Frontmatter::default(),
            goal: "Goal".into(),
            identity: None,
            constraints: Constraints::default(),
            sacred: vec![],
            assumptions: vec![
                Assumption {
                    text: "Unvalidated one".into(),
                    validated: false,
                    has_checkbox: true,
                },
                Assumption {
                    text: "Validated one".into(),
                    validated: true,
                    has_checkbox: true,
                },
            ],
            deliverable: None,
            unknown_sections: vec![],
        };
        let output = emit_prompt(&brief);
        assert!(output.contains("ASSUMPTIONS (UNVALIDATED):\n- Unvalidated one"));
        assert!(output.contains("ASSUMPTIONS (VALIDATED):\n- Validated one"));
    }
}
