use crate::model::Brief;

/// Derive a skill name slug from the goal text.
fn slugify(goal: &str) -> String {
    goal.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// Emit a Claude Code SKILL.md file from a Brief.
pub fn emit_skill(brief: &Brief) -> String {
    let mut out = String::new();

    let name = brief
        .frontmatter
        .skill_name
        .as_deref()
        .map(String::from)
        .unwrap_or_else(|| slugify(&brief.goal));

    let description = brief
        .frontmatter
        .skill_description
        .as_deref()
        .unwrap_or(&brief.goal);

    // YAML frontmatter
    out.push_str("---\n");
    out.push_str(&format!("name: {name}\n"));
    out.push_str(&format!("description: {description}\n"));
    out.push_str("---\n\n");

    // Opening instruction from goal
    out.push_str(description);
    out.push('.');

    // Stack context
    if !brief.frontmatter.stack.is_empty() {
        out.push_str(&format!(
            " This project uses {}.",
            brief.frontmatter.stack.join(", ")
        ));
    }
    out.push('\n');

    // Context files
    if !brief.frontmatter.context.is_empty() {
        out.push_str("\nBefore starting, read these files for context:\n");
        for ctx in &brief.frontmatter.context {
            out.push_str(&format!("- `{ctx}`\n"));
        }
    }

    // Rules — hard constraints
    if !brief.constraints.hard.is_empty() {
        out.push_str("\n## Rules\n\nYou MUST follow these rules:\n");
        for c in &brief.constraints.hard {
            out.push_str(&format!("- {c}\n"));
        }
    }

    // Preferences — soft constraints
    if !brief.constraints.soft.is_empty() {
        out.push_str("\nPrefer these approaches when possible:\n");
        for c in &brief.constraints.soft {
            out.push_str(&format!("- {c}\n"));
        }
    }

    // Ask first
    if !brief.constraints.ask_first.is_empty() {
        out.push_str("\nAsk the user before proceeding with:\n");
        for c in &brief.constraints.ask_first {
            out.push_str(&format!("- {c}\n"));
        }
    }

    // Sacred / protected regions
    if !brief.sacred.is_empty() {
        out.push_str("\n## Protected regions\n\nDo NOT modify or suggest changes to these files:\n");
        for entry in &brief.sacred {
            out.push_str(&format!("- `{}` — {}\n", entry.path, entry.reason));
        }
    }

    // Assumptions as verification steps
    let unvalidated: Vec<_> = brief.assumptions.iter().filter(|a| !a.validated).collect();
    if !unvalidated.is_empty() {
        out.push_str("\n## Verify before proceeding\n\n");
        out.push_str("Confirm these assumptions still hold before acting on them:\n");
        for a in &unvalidated {
            out.push_str(&format!("- {}\n", a.text));
        }
    }

    // Deliverable
    if let Some(ref deliverable) = brief.deliverable {
        out.push_str("\n## Expected output\n\n");
        out.push_str(deliverable);
        if !deliverable.ends_with('\n') {
            out.push('\n');
        }
    }

    out
}

/// Return the skill name that would be used for this brief.
pub fn skill_name(brief: &Brief) -> String {
    brief
        .frontmatter
        .skill_name
        .as_deref()
        .map(String::from)
        .unwrap_or_else(|| slugify(&brief.goal))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;

    #[test]
    fn slugify_basic() {
        assert_eq!(slugify("Fix the login bug"), "fix-the-login-bug");
        assert_eq!(slugify("Review Code!"), "review-code");
        assert_eq!(
            slugify("  Spaces  and---dashes  "),
            "spaces-and-dashes"
        );
    }

    #[test]
    fn emit_uses_explicit_skill_name() {
        let brief = Brief {
            frontmatter: Frontmatter {
                stack: vec!["Rust".into()],
                skill_name: Some("review".into()),
                skill_description: Some("Review code following team standards".into()),
                ..Default::default()
            },
            goal: "Perform thorough code review".into(),
            constraints: Constraints::default(),
            sacred: vec![],
            assumptions: vec![],
            deliverable: None,
            unknown_sections: vec![],
        };
        let output = emit_skill(&brief);
        assert!(output.contains("name: review\n"));
        assert!(output.contains("description: Review code following team standards\n"));
    }

    #[test]
    fn emit_derives_name_from_goal() {
        let brief = Brief {
            frontmatter: Frontmatter {
                stack: vec!["Python".into()],
                ..Default::default()
            },
            goal: "Deploy the service".into(),
            constraints: Constraints::default(),
            sacred: vec![],
            assumptions: vec![],
            deliverable: None,
            unknown_sections: vec![],
        };
        let output = emit_skill(&brief);
        assert!(output.contains("name: deploy-the-service\n"));
        assert!(output.contains("description: Deploy the service\n"));
    }

    #[test]
    fn emit_includes_rules_and_preferences() {
        let brief = Brief {
            frontmatter: Frontmatter::default(),
            goal: "Task".into(),
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
        let output = emit_skill(&brief);
        assert!(output.contains("## Rules"));
        assert!(output.contains("You MUST follow these rules:"));
        assert!(output.contains("- No breaking changes"));
        assert!(output.contains("Prefer these approaches"));
        assert!(output.contains("- Prefer async"));
        assert!(output.contains("Ask the user before"));
        assert!(output.contains("- Schema changes"));
    }

    #[test]
    fn emit_includes_protected_regions() {
        let brief = Brief {
            frontmatter: Frontmatter::default(),
            goal: "Task".into(),
            constraints: Constraints::default(),
            sacred: vec![SacredEntry {
                path: "src/auth/**".into(),
                reason: "SOC2 audited".into(),
                well_formed: true,
            }],
            assumptions: vec![],
            deliverable: None,
            unknown_sections: vec![],
        };
        let output = emit_skill(&brief);
        assert!(output.contains("## Protected regions"));
        assert!(output.contains("`src/auth/**` — SOC2 audited"));
    }

    #[test]
    fn emit_converts_unvalidated_assumptions_to_verification() {
        let brief = Brief {
            frontmatter: Frontmatter::default(),
            goal: "Task".into(),
            constraints: Constraints::default(),
            sacred: vec![],
            assumptions: vec![
                Assumption {
                    text: "Redis is available".into(),
                    validated: false,
                    has_checkbox: true,
                },
                Assumption {
                    text: "Tests pass".into(),
                    validated: true,
                    has_checkbox: true,
                },
            ],
            deliverable: None,
            unknown_sections: vec![],
        };
        let output = emit_skill(&brief);
        assert!(output.contains("## Verify before proceeding"));
        assert!(output.contains("- Redis is available"));
        // Validated assumptions should NOT appear in verification section
        assert!(!output.contains("- Tests pass"));
    }

    #[test]
    fn emit_includes_context_files() {
        let brief = Brief {
            frontmatter: Frontmatter {
                context: vec!["./docs/arch.md".into(), "./README.md".into()],
                ..Default::default()
            },
            goal: "Task".into(),
            constraints: Constraints::default(),
            sacred: vec![],
            assumptions: vec![],
            deliverable: None,
            unknown_sections: vec![],
        };
        let output = emit_skill(&brief);
        assert!(output.contains("read these files for context"));
        assert!(output.contains("`./docs/arch.md`"));
        assert!(output.contains("`./README.md`"));
    }

    #[test]
    fn emit_includes_deliverable() {
        let brief = Brief {
            frontmatter: Frontmatter::default(),
            goal: "Task".into(),
            constraints: Constraints::default(),
            sacred: vec![],
            assumptions: vec![],
            deliverable: Some("Working code with tests.\n".into()),
            unknown_sections: vec![],
        };
        let output = emit_skill(&brief);
        assert!(output.contains("## Expected output"));
        assert!(output.contains("Working code with tests."));
    }

    #[test]
    fn emit_includes_stack_in_opening() {
        let brief = Brief {
            frontmatter: Frontmatter {
                stack: vec!["Python 3.12".into(), "PostgreSQL 16".into()],
                ..Default::default()
            },
            goal: "Review code".into(),
            constraints: Constraints::default(),
            sacred: vec![],
            assumptions: vec![],
            deliverable: None,
            unknown_sections: vec![],
        };
        let output = emit_skill(&brief);
        assert!(output.contains("This project uses Python 3.12, PostgreSQL 16."));
    }
}
