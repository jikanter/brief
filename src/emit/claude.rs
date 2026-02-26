use crate::model::Brief;

/// Emit a CLAUDE.md-compatible section from a Brief.
pub fn emit_claude(brief: &Brief) -> String {
    let mut out = String::new();

    out.push_str(&format!("# Briefing: {}\n\n", brief.goal));

    // Stack
    if !brief.frontmatter.stack.is_empty() {
        out.push_str(&format!(
            "**Stack:** {}\n\n",
            brief.frontmatter.stack.join(", ")
        ));
    }

    // Constraints
    let has_constraints = !brief.constraints.hard.is_empty()
        || !brief.constraints.soft.is_empty()
        || !brief.constraints.ask_first.is_empty();

    if has_constraints {
        out.push_str("## Constraints\n\n");

        if !brief.constraints.hard.is_empty() {
            out.push_str("### Hard (Non-negotiable)\n");
            for c in &brief.constraints.hard {
                out.push_str(&format!("- {c}\n"));
            }
            out.push('\n');
        }

        if !brief.constraints.soft.is_empty() {
            out.push_str("### Soft (Preferred)\n");
            for c in &brief.constraints.soft {
                out.push_str(&format!("- {c}\n"));
            }
            out.push('\n');
        }

        if !brief.constraints.ask_first.is_empty() {
            out.push_str("### Ask First (Requires approval)\n");
            for c in &brief.constraints.ask_first {
                out.push_str(&format!("- {c}\n"));
            }
            out.push('\n');
        }
    }

    // Sacred
    if !brief.sacred.is_empty() {
        out.push_str("## Sacred Regions (Do Not Modify)\n");
        for entry in &brief.sacred {
            out.push_str(&format!("- `{}` — {}\n", entry.path, entry.reason));
        }
        out.push('\n');
    }

    // Assumptions
    if !brief.assumptions.is_empty() {
        out.push_str("## Assumptions\n");
        for a in &brief.assumptions {
            let marker = if a.validated { "[x]" } else { "[ ]" };
            out.push_str(&format!("- {marker} {}\n", a.text));
        }
        out.push('\n');
    }

    // Deliverable
    if let Some(ref deliverable) = brief.deliverable {
        out.push_str("## Deliverable\n");
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
    fn emit_contains_goal() {
        let brief = Brief {
            frontmatter: Frontmatter {
                stack: vec!["Rust".into()],
                ..Default::default()
            },
            goal: "Fix the bug".into(),
            constraints: Constraints::default(),
            sacred: vec![],
            assumptions: vec![],
            deliverable: None,
            unknown_sections: vec![],
        };
        let output = emit_claude(&brief);
        assert!(output.contains("# Briefing: Fix the bug"));
    }

    #[test]
    fn emit_contains_constraints() {
        let brief = Brief {
            frontmatter: Frontmatter::default(),
            goal: "Goal".into(),
            constraints: Constraints {
                hard: vec!["No breaking changes".into()],
                soft: vec!["Prefer async".into()],
                ask_first: vec!["Schema changes".into()],
            },
            sacred: vec![],
            assumptions: vec![],
            deliverable: None,
            unknown_sections: vec![],
        };
        let output = emit_claude(&brief);
        assert!(output.contains("Non-negotiable"));
        assert!(output.contains("No breaking changes"));
        assert!(output.contains("Prefer async"));
        assert!(output.contains("Schema changes"));
    }

    #[test]
    fn emit_contains_sacred() {
        let brief = Brief {
            frontmatter: Frontmatter::default(),
            goal: "Goal".into(),
            constraints: Constraints::default(),
            sacred: vec![SacredEntry {
                path: "src/auth/**".into(),
                reason: "Auth logic".into(),
                well_formed: true,
            }],
            assumptions: vec![],
            deliverable: None,
            unknown_sections: vec![],
        };
        let output = emit_claude(&brief);
        assert!(output.contains("`src/auth/**`"));
        assert!(output.contains("Auth logic"));
    }
}
