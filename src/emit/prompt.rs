use crate::model::Brief;

/// Emit a raw system prompt suitable for direct API use.
pub fn emit_prompt(brief: &Brief) -> String {
    let mut out = String::new();

    out.push_str(&format!("GOAL: {}\n\n", brief.goal));

    if !brief.frontmatter.stack.is_empty() {
        out.push_str(&format!(
            "STACK: {}\n\n",
            brief.frontmatter.stack.join(", ")
        ));
    }

    if !brief.constraints.hard.is_empty() {
        out.push_str("HARD CONSTRAINTS:\n");
        for c in &brief.constraints.hard {
            out.push_str(&format!("- {c}\n"));
        }
        out.push('\n');
    }

    if !brief.constraints.soft.is_empty() {
        out.push_str("SOFT CONSTRAINTS:\n");
        for c in &brief.constraints.soft {
            out.push_str(&format!("- {c}\n"));
        }
        out.push('\n');
    }

    if !brief.constraints.ask_first.is_empty() {
        out.push_str("ASK BEFORE PROCEEDING:\n");
        for c in &brief.constraints.ask_first {
            out.push_str(&format!("- {c}\n"));
        }
        out.push('\n');
    }

    if !brief.sacred.is_empty() {
        out.push_str("DO NOT MODIFY:\n");
        for entry in &brief.sacred {
            out.push_str(&format!("- {}: {}\n", entry.path, entry.reason));
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
    fn prompt_separates_validated_assumptions() {
        let brief = Brief {
            frontmatter: Frontmatter::default(),
            goal: "Goal".into(),
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
