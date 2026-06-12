use crate::framing::{frame_ask_first, frame_hard, frame_soft};
use crate::model::Brief;

/// Emit a raw system prompt suitable for direct API use.
///
/// The `prompt` target occupies the highest-privilege position — it lands
/// directly in an API system prompt — so it is aggressively optimized for
/// compliance on two axes (phase2-synthesis P0):
///
/// - **Register.** Hard constraints are framed by polarity (`NEVER:`/`MUST:`/
///   plain convention), soft ones as `PREFER:`, ask-first ones as explicit
///   `STOP and confirm` interruptions (see [`crate::framing`]).
/// - **Order.** Sections are placed for attention dynamics, not human reading
///   flow: hard constraints and sacred regions take the **primacy** position
///   (first ~15% of context gets the highest attention), goal/stack/context and
///   the softer constraints sit in the middle, and the deliverable takes the
///   **recency** position (last ~15%) as the closing action directive.
pub fn emit_prompt(brief: &Brief) -> String {
    let mut out = String::new();

    // -- Primacy: the rules that must not be violated --

    if !brief.constraints.hard.is_empty() {
        out.push_str("HARD CONSTRAINTS:\n");
        for c in &brief.constraints.hard {
            out.push_str(&format!("- {}\n", frame_hard(c)));
        }
        out.push('\n');
    }

    if !brief.sacred.is_empty() {
        out.push_str(
            "DO NOT MODIFY (these files must not be changed under any circumstances; if a task requires it, STOP and report the conflict):\n",
        );
        for entry in &brief.sacred {
            out.push_str(&format!("- {}: {}\n", entry.path, entry.reason));
        }
        out.push('\n');
    }

    // -- Middle: task frame and reference context --

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

    if !brief.constraints.soft.is_empty() {
        out.push_str("SOFT CONSTRAINTS:\n");
        for c in &brief.constraints.soft {
            out.push_str(&format!("- {}\n", frame_soft(c)));
        }
        out.push('\n');
    }

    if !brief.constraints.ask_first.is_empty() {
        out.push_str("ASK BEFORE PROCEEDING:\n");
        for c in &brief.constraints.ask_first {
            out.push_str(&format!("- {}\n", frame_ask_first(c)));
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

    // Unknown sections (passthrough) — reference material, before the closer.
    for section in &brief.unknown_sections {
        out.push_str(&format!(
            "{}:\n{}\n\n",
            section.heading.to_uppercase(),
            section.content
        ));
    }

    // -- Recency: the closing action directive --

    if let Some(ref deliverable) = brief.deliverable {
        out.push_str("DELIVERABLE:\n");
        out.push_str(deliverable);
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
    fn prompt_frames_constraints_by_polarity() {
        let brief = Brief {
            frontmatter: Frontmatter::default(),
            goal: "Goal".into(),
            identity: None,
            constraints: Constraints {
                hard: vec![
                    "Do not break the public API".into(),
                    "All handlers return Result".into(),
                    "Use thiserror for errors".into(),
                ],
                soft: vec![],
                ask_first: vec!["Changing the schema".into()],
            },
            sacred: vec![],
            assumptions: vec![],
            deliverable: None,
            unknown_sections: vec![],
        };
        let output = emit_prompt(&brief);
        assert!(output.contains("- NEVER: break the public API"));
        assert!(output.contains("- MUST: All handlers return Result"));
        // Convention stays plain — not dressed as adversarial.
        assert!(output.contains("- Use thiserror for errors"));
        assert!(output.contains("STOP and confirm with the user before: Changing the schema"));
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
