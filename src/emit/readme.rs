use std::path::Path;

use crate::emit::markers::{inject_section, wrap_with_markers};
use crate::framing::with_scope;
use crate::model::Brief;

/// Emit a README section from a Brief.
///
/// A README is read by people (contributors and reviewers), not by an agent
/// runtime, so this target has its own register rather than the AGENTS.md one:
/// purpose before rules, the three constraint tiers named in plain English
/// with no agent markers, sacred paths with their reasons, and context as
/// Markdown links a forge renders as clickable.
pub fn emit_readme(brief: &Brief) -> String {
    let mut out = format!("# {}\n\n", brief.goal);

    if !brief.frontmatter.stack.is_empty() {
        out.push_str(&format!(
            "Built with {}.\n\n",
            brief.frontmatter.stack.join(", ")
        ));
    }

    if let Some(deliverable) = &brief.deliverable {
        out.push_str(&format!(
            "## What done looks like\n\n{}\n\n",
            deliverable.trim_end()
        ));
    }

    let tiers = [
        ("Must", &brief.constraints.hard),
        ("Should", &brief.constraints.soft),
        ("Ask a maintainer first", &brief.constraints.ask_first),
    ];
    if tiers.iter().any(|(_, cs)| !cs.is_empty()) {
        out.push_str("## Ground rules\n\n");
        for (heading, constraints) in tiers {
            if constraints.is_empty() {
                continue;
            }
            out.push_str(&format!("### {heading}\n\n"));
            for c in constraints {
                out.push_str(&format!("- {}\n", with_scope(c.as_ref(), c)));
            }
            out.push('\n');
        }
    }

    if !brief.sacred.is_empty() {
        out.push_str("## Protected paths\n\nThese paths must not be modified:\n\n");
        for entry in &brief.sacred {
            out.push_str(&format!("- `{}` — {}\n", entry.path, entry.reason));
        }
        out.push('\n');
    }

    if !brief.assumptions.is_empty() {
        out.push_str("## Assumptions\n\n");
        for assumption in &brief.assumptions {
            let marker = if assumption.validated { "[x]" } else { "[ ]" };
            out.push_str(&format!("- {marker} {}\n", assumption.text));
        }
        out.push('\n');
    }

    if !brief.frontmatter.context.is_empty() {
        out.push_str("## Further reading\n\n");
        for ctx in &brief.frontmatter.context {
            let label = ctx.strip_prefix("./").unwrap_or(ctx);
            out.push_str(&format!("- [{label}]({ctx})\n"));
        }
        out.push('\n');
    }

    for section in &brief.unknown_sections {
        out.push_str(&format!(
            "## {}\n\n{}\n\n",
            section.heading,
            section.content.trim_end()
        ));
    }

    out.truncate(out.trim_end().len());
    out.push('\n');
    out
}

/// Inject a briefing section into an existing README.md or create one.
///
/// Behaves identically to `install_claude` for CLAUDE.md: wraps the emitted
/// section in `<brief:generated>` markers, replaces the first existing pair
/// in place (migrating the legacy `<!-- brief:start -->` flavor), strips
/// empty extra pairs, and appends if no markers exist. Hand-written README
/// prose outside the markers is never touched.
pub fn install_readme(brief: &Brief, readme_path: &Path) -> Result<String, std::io::Error> {
    let section = emit_readme(brief);
    let wrapped = wrap_with_markers(&section);

    let output = if readme_path.exists() {
        let existing = std::fs::read_to_string(readme_path)?;
        let (result, pairs_found) = inject_section(&existing, &wrapped);
        if pairs_found > 1 {
            eprintln!(
                "warning: found {} brief marker pairs; using the first, stripping remaining empty pairs",
                pairs_found
            );
        }
        result
    } else {
        wrapped
    };

    std::fs::write(readme_path, &output)?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::emit::emit_agents_md;
    use crate::model::*;

    #[test]
    fn readme_groups_hard_rules_under_must() {
        let brief = Brief {
            frontmatter: Frontmatter::default(),
            goal: "Build feature".into(),
            identity: None,
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
        let output = emit_readme(&brief);
        assert!(output.contains("## Ground rules\n\n### Must\n\n- Must pass CI\n"));
        assert!(!output.contains("(REQUIRED)"));
    }

    #[test]
    fn readme_emits_unknown_sections() {
        let brief = Brief {
            frontmatter: Frontmatter::default(),
            goal: "This super project Goal".into(),
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
        let output = emit_readme(&brief);
        assert!(output.contains("## Commands"));
        assert!(output.contains("- Build: `cargo build`"))
    }

    #[test]
    fn readme_uses_protected_paths_heading() {
        let brief = Brief {
            frontmatter: Frontmatter::default(),
            goal: "Awesomely Project Goal".into(),
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
        let output = emit_readme(&brief);
        assert!(output.contains("## Protected paths"));
        assert!(output.contains("- `src/auth/**` — Auth\n"));
    }

    fn rich_brief() -> Brief {
        Brief {
            frontmatter: Frontmatter {
                stack: vec!["Rust".into(), "SQLite".into()],
                context: vec![
                    "./docs/architecture.md".into(),
                    "https://example.com/spec".into(),
                ],
                ..Frontmatter::default()
            },
            goal: "Ship the sync engine".into(),
            identity: None,
            constraints: Constraints {
                hard: vec![Constraint {
                    text: "No unsafe".into(),
                    scope: vec!["src/core/**".into()],
                }],
                soft: vec!["Prefer small commits".into()],
                ask_first: vec!["New dependencies".into()],
            },
            sacred: vec![],
            assumptions: vec![
                Assumption {
                    text: "SQLite WAL is enough".into(),
                    validated: false,
                    has_checkbox: true,
                },
                Assumption {
                    text: "Clients run Rust 1.80".into(),
                    validated: true,
                    has_checkbox: true,
                },
            ],
            deliverable: Some("Two replicas converge offline.".into()),
            unknown_sections: vec![],
        }
    }

    #[test]
    fn readme_is_not_the_agents_md_section() {
        let brief = rich_brief();
        assert_ne!(emit_readme(&brief), emit_agents_md(&brief));
    }

    #[test]
    fn readme_states_stack_and_purpose_before_rules() {
        let output = emit_readme(&rich_brief());
        assert!(output.starts_with(
            "# Ship the sync engine\n\nBuilt with Rust, SQLite.\n\n## What done looks like\n\nTwo replicas converge offline.\n\n## Ground rules\n"
        ));
    }

    #[test]
    fn readme_names_each_rule_tier_in_plain_english() {
        let output = emit_readme(&rich_brief());
        assert!(output.contains("### Must\n\n- When working in `src/core/**`: No unsafe\n"));
        assert!(output.contains("### Should\n\n- Prefer small commits\n"));
        assert!(output.contains("### Ask a maintainer first\n\n- New dependencies\n"));
    }

    #[test]
    fn readme_omits_empty_rule_tiers() {
        let mut brief = rich_brief();
        brief.constraints.soft.clear();
        assert!(!emit_readme(&brief).contains("### Should"));
    }

    #[test]
    fn readme_links_context_as_markdown() {
        let output = emit_readme(&rich_brief());
        assert!(output.contains(
            "## Further reading\n\n- [docs/architecture.md](./docs/architecture.md)\n- [https://example.com/spec](https://example.com/spec)\n"
        ));
    }

    #[test]
    fn readme_keeps_assumption_checkboxes() {
        let output = emit_readme(&rich_brief());
        assert!(output.contains(
            "## Assumptions\n\n- [ ] SQLite WAL is enough\n- [x] Clients run Rust 1.80\n"
        ));
    }

    #[test]
    fn readme_ends_with_one_newline() {
        let output = emit_readme(&rich_brief());
        assert!(output.ends_with("\n") && !output.ends_with("\n\n"));
    }

    fn minimal_brief_for_readme_tests() -> Brief {
        Brief {
            frontmatter: Frontmatter::default(),
            goal: "Fix the login bug".into(),
            identity: None,
            constraints: Constraints {
                hard: vec!["Do not break existing tests".into()],
                soft: vec![],
                ask_first: vec![],
            },
            sacred: vec![SacredEntry {
                path: "src/auth.rs".into(),
                reason: "Tenant resolution".into(),
                well_formed: true,
            }],
            assumptions: vec![],
            deliverable: None,
            unknown_sections: vec![],
        }
    }

    #[test]
    fn install_readme_creates_file_when_missing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("README.md");
        let brief = crate::emit::readme::tests::minimal_brief_for_readme_tests();

        install_readme(&brief, &path).unwrap();

        let result = std::fs::read_to_string(&path).unwrap();
        assert!(result.starts_with("<brief:generated>"));
        assert!(result.contains("# Fix the login bug"));
        assert!(result.contains("- Do not break existing tests\n"));
        assert!(result.contains("`src/auth.rs` — Tenant resolution"));
        assert!(result.trim_end().ends_with("</brief:generated>"));
    }

    #[test]
    fn install_readme_replaces_existing_marker_pair() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("README.md");
        std::fs::write(
            &path,
            "# My Project\n\nIntro.\n\n<brief:generated>\nstale brief content\n</brief:generated>\n\n## Other\n\nMore.\n",
        )
            .unwrap();
        let brief = crate::emit::readme::tests::minimal_brief_for_readme_tests();

        install_readme(&brief, &path).unwrap();

        let result = std::fs::read_to_string(&path).unwrap();
        assert!(result.contains("# My Project"));
        assert!(result.contains("Intro."));
        assert!(result.contains("## Other"));
        assert!(result.contains("More."));
        assert!(result.contains("# Fix the login bug"));
        assert!(!result.contains("stale brief content"));
        assert_eq!(result.matches("<brief:generated>").count(), 1);
        assert_eq!(result.matches("</brief:generated>").count(), 1);
    }

    #[test]
    fn install_readme_appends_when_no_markers() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("README.md");
        std::fs::write(&path, "# My Project\n\nFreeform project notes.\n").unwrap();
        let brief = crate::emit::readme::tests::minimal_brief_for_readme_tests();

        install_readme(&brief, &path).unwrap();

        let result = std::fs::read_to_string(&path).unwrap();
        assert!(result.starts_with("# My Project"));
        assert!(result.contains("Freeform project notes."));
        assert!(result.contains("<brief:generated>"));
        assert!(result.contains("# Fix the login bug"));
        assert!(result.trim_end().ends_with("</brief:generated>"));
    }

    #[test]
    fn install_readme_migrates_legacy_html_comment_markers() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("README.md");
        std::fs::write(
            &path,
            "# My Project\n\n<!-- brief:start -->\nold legacy\n<!-- brief:end -->\n\n## Other\n",
        )
        .unwrap();
        let brief = crate::emit::readme::tests::minimal_brief_for_readme_tests();

        install_readme(&brief, &path).unwrap();

        let result = std::fs::read_to_string(&path).unwrap();
        assert!(!result.contains("<!-- brief:start -->"));
        assert!(!result.contains("<!-- brief:end -->"));
        assert!(!result.contains("old legacy"));
        assert_eq!(result.matches("<brief:generated>").count(), 1);
        assert_eq!(result.matches("</brief:generated>").count(), 1);
        assert!(result.contains("# My Project"));
        assert!(result.contains("## Other"));
        assert!(result.contains("# Fix the login bug"));
    }
}
