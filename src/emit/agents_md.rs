use crate::model::Brief;

/// Emit an AGENTS.md-compatible section from a Brief.
pub fn emit_agents_md(brief: &Brief) -> String {
    let mut out = String::new();

    out.push_str(&format!("# {}\n\n", brief.goal));

    if !brief.frontmatter.stack.is_empty() {
        out.push_str(&format!(
            "**Stack:** {}\n\n",
            brief.frontmatter.stack.join(", ")
        ));
    }

    // Context
    if !brief.frontmatter.context.is_empty() {
        out.push_str("## Context\n\nRefer to these files for background:\n");
        for ctx in &brief.frontmatter.context {
            out.push_str(&format!("- `{ctx}`\n"));
        }
        out.push('\n');
    }

    // Instructions section — merge all constraints
    let has_constraints = !brief.constraints.hard.is_empty()
        || !brief.constraints.soft.is_empty()
        || !brief.constraints.ask_first.is_empty();

    if has_constraints {
        out.push_str("## Instructions\n");
        for c in &brief.constraints.hard {
            out.push_str(&format!("- {c} **(REQUIRED)**\n"));
        }
        for c in &brief.constraints.soft {
            out.push_str(&format!("- {c} *(preferred)*\n"));
        }
        for c in &brief.constraints.ask_first {
            out.push_str(&format!("- {c} **(ASK FIRST)**\n"));
        }
        out.push('\n');
    }

    // Protected files
    if !brief.sacred.is_empty() {
        out.push_str("## Protected Files\n");
        for entry in &brief.sacred {
            out.push_str(&format!("- `{}`: {}\n", entry.path, entry.reason));
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
    fn agents_md_marks_hard_as_required() {
        let brief = Brief {
            frontmatter: Frontmatter::default(),
            goal: "Build feature".into(),
            constraints: Constraints {
                hard: vec!["Must pass CI".into()],
                soft: vec![],
                ask_first: vec![],
            },
            sacred: vec![],
            assumptions: vec![],
            deliverable: None,
            unknown_sections: vec![],
        };
        let output = emit_agents_md(&brief);
        assert!(output.contains("Must pass CI **(REQUIRED)**"));
    }

    #[test]
    fn agents_md_uses_protected_files_heading() {
        let brief = Brief {
            frontmatter: Frontmatter::default(),
            goal: "Goal".into(),
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
        let output = emit_agents_md(&brief);
        assert!(output.contains("## Protected Files"));
        assert!(output.contains("`src/auth/**`: Auth"));
    }
}
