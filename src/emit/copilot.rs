use std::path::Path;

use crate::emit::markers::{inject_section, wrap_with_markers};
use crate::framing::with_scope;
use crate::model::Brief;

/// Emit a GitHub Copilot `.github/copilot-instructions.md` section from a Brief.
///
/// Copilot reads `.github/copilot-instructions.md` as repository-wide custom
/// instructions — plain Markdown, no frontmatter. Its idiomatic register is
/// descriptive rather than imperative, so unlike the `claude` target this
/// emitter does not reach for NEVER/MUST framing: hard constraints render as a
/// "Requirements" list and soft ones as "Preferences". Path-scoped
/// `.github/instructions/*.instructions.md` output is deferred until brief
/// grows a scoped-constraints concept — see
/// docs/design/backends/copilot/README.md.
pub fn emit_copilot(brief: &Brief) -> String {
    let mut out = String::new();

    out.push_str(&format!("# {}\n\n", brief.goal));

    if !brief.frontmatter.stack.is_empty() {
        out.push_str(&format!(
            "**Stack:** {}\n\n",
            brief.frontmatter.stack.join(", ")
        ));
    }

    if !brief.frontmatter.context.is_empty() {
        out.push_str("## Reference\n\nFor background, see:\n\n");
        for ctx in &brief.frontmatter.context {
            out.push_str(&format!("- `{ctx}`\n"));
        }
        out.push('\n');
    }

    if !brief.constraints.hard.is_empty() {
        out.push_str("## Requirements\n\n");
        for c in &brief.constraints.hard {
            out.push_str(&format!("- {}\n", with_scope(c, c)));
        }
        out.push('\n');
    }

    if !brief.constraints.soft.is_empty() {
        out.push_str("## Preferences\n\n");
        for c in &brief.constraints.soft {
            out.push_str(&format!("- {}\n", with_scope(c, c)));
        }
        out.push('\n');
    }

    if !brief.constraints.ask_first.is_empty() {
        out.push_str("## Check before proceeding\n\n");
        for c in &brief.constraints.ask_first {
            out.push_str(&format!("- {}\n", with_scope(c, c)));
        }
        out.push('\n');
    }

    if !brief.sacred.is_empty() {
        out.push_str("## Protected files\n\nDo not modify these files:\n\n");
        for entry in &brief.sacred {
            out.push_str(&format!("- `{}` — {}\n", entry.path, entry.reason));
        }
        out.push('\n');
    }

    let unvalidated: Vec<_> = brief.assumptions.iter().filter(|a| !a.validated).collect();
    if !unvalidated.is_empty() {
        out.push_str("## Assumptions to verify\n\n");
        for a in &unvalidated {
            out.push_str(&format!("- {}\n", a.text));
        }
        out.push('\n');
    }

    if let Some(ref deliverable) = brief.deliverable {
        out.push_str("## Definition of done\n\n");
        out.push_str(deliverable.trim_end_matches('\n'));
        out.push('\n');
    }

    for section in &brief.unknown_sections {
        out.push_str(&format!(
            "\n## {}\n\n{}\n",
            section.heading, section.content
        ));
    }

    out
}

/// Install a Copilot instructions section into `.github/copilot-instructions.md`.
///
/// Like CLAUDE.md/AGENTS.md, this file mixes hand-written guidance with the
/// brief-generated section, so the same `<brief:generated>` marker injection
/// applies: replace the first marker pair in place (migrating the legacy
/// flavor), append if none exist. The `.github` directory is created if absent.
pub fn install_copilot(
    brief: &Brief,
    base_dir: &Path,
) -> Result<std::path::PathBuf, std::io::Error> {
    let github_dir = base_dir.join(".github");
    std::fs::create_dir_all(&github_dir)?;
    let target = github_dir.join("copilot-instructions.md");

    let section = emit_copilot(brief);
    let wrapped = wrap_with_markers(&section);

    let output = if target.exists() {
        let existing = std::fs::read_to_string(&target)?;
        let (result, pairs_found) = inject_section(&existing, &wrapped);
        if pairs_found > 1 {
            eprintln!(
                "warning: found {pairs_found} brief marker pairs; using the first, stripping remaining empty pairs"
            );
        }
        result
    } else {
        wrapped
    };

    std::fs::write(&target, &output)?;
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;

    fn full_brief() -> Brief {
        Brief {
            frontmatter: Frontmatter {
                stack: vec!["TypeScript 5.4".into(), "React 18".into()],
                context: vec!["./docs/architecture.md".into()],
                ..Default::default()
            },
            goal: "Add real-time notifications".into(),
            identity: None,
            constraints: Constraints {
                hard: vec!["All notifications delivered within 5 seconds".into()],
                soft: vec!["Prefer WebSocket over polling".into()],
                ask_first: vec!["Changes to the notification schema".into()],
            },
            sacred: vec![SacredEntry {
                path: "src/auth/**".into(),
                reason: "SOC2 audited".into(),
                well_formed: true,
            }],
            assumptions: vec![
                Assumption {
                    text: "Gateway can handle 5k connections".into(),
                    validated: false,
                    has_checkbox: true,
                },
                Assumption {
                    text: "REST API supports subscriptions".into(),
                    validated: true,
                    has_checkbox: true,
                },
            ],
            deliverable: Some("Working notification system.".into()),
            unknown_sections: vec![UnknownSection {
                heading: "Commands".into(),
                content: "- Build: `npm run build`".into(),
            }],
        }
    }

    #[test]
    fn emit_uses_goal_as_h1() {
        let output = emit_copilot(&full_brief());
        assert!(output.starts_with("# Add real-time notifications"));
    }

    #[test]
    fn emit_uses_descriptive_register_not_imperative() {
        // Copilot's idiomatic register is descriptive — no NEVER/MUST/IMPORTANT.
        let output = emit_copilot(&full_brief());
        assert!(output.contains("## Requirements"));
        assert!(output.contains("- All notifications delivered within 5 seconds"));
        assert!(!output.contains("**IMPORTANT:**"));
        assert!(!output.contains("NEVER:"));
        assert!(!output.contains("MUST:"));
    }

    #[test]
    fn emit_renders_preferences_and_ask_first() {
        let output = emit_copilot(&full_brief());
        assert!(output.contains("## Preferences"));
        assert!(output.contains("- Prefer WebSocket over polling"));
        assert!(output.contains("## Check before proceeding"));
        assert!(output.contains("- Changes to the notification schema"));
    }

    #[test]
    fn emit_renders_protected_files() {
        let output = emit_copilot(&full_brief());
        assert!(output.contains("## Protected files"));
        assert!(output.contains("`src/auth/**` — SOC2 audited"));
    }

    #[test]
    fn emit_renders_only_unvalidated_assumptions() {
        let output = emit_copilot(&full_brief());
        assert!(output.contains("Gateway can handle 5k connections"));
        assert!(!output.contains("REST API supports subscriptions"));
    }

    #[test]
    fn emit_passes_through_unknown_sections() {
        let output = emit_copilot(&full_brief());
        assert!(output.contains("## Commands"));
        assert!(output.contains("- Build: `npm run build`"));
    }

    #[test]
    fn emit_minimal_omits_empty_sections() {
        let brief = Brief {
            frontmatter: Frontmatter::default(),
            goal: "Fix it".into(),
            identity: None,
            constraints: Constraints::default(),
            sacred: vec![],
            assumptions: vec![],
            deliverable: None,
            unknown_sections: vec![],
        };
        let output = emit_copilot(&brief);
        assert!(output.contains("# Fix it"));
        assert!(!output.contains("## Requirements"));
        assert!(!output.contains("## Protected files"));
        assert!(!output.contains("**Stack:**"));
    }

    #[test]
    fn install_creates_file_under_github_dir() {
        let dir = tempfile::tempdir().unwrap();
        let written = install_copilot(&full_brief(), dir.path()).unwrap();
        let expected = dir.path().join(".github").join("copilot-instructions.md");
        assert_eq!(written, expected);
        let content = std::fs::read_to_string(&expected).unwrap();
        assert!(content.starts_with("<brief:generated>"));
        assert!(content.contains("# Add real-time notifications"));
        assert!(content.trim_end().ends_with("</brief:generated>"));
    }

    #[test]
    fn install_replaces_existing_marker_pair() {
        let dir = tempfile::tempdir().unwrap();
        let github_dir = dir.path().join(".github");
        std::fs::create_dir_all(&github_dir).unwrap();
        let target = github_dir.join("copilot-instructions.md");
        std::fs::write(
            &target,
            "# Hand-written intro\n\n<brief:generated>\nstale\n</brief:generated>\n\n## Notes\n",
        )
        .unwrap();

        install_copilot(&full_brief(), dir.path()).unwrap();

        let content = std::fs::read_to_string(&target).unwrap();
        assert!(content.contains("# Hand-written intro"));
        assert!(content.contains("## Notes"));
        assert!(content.contains("# Add real-time notifications"));
        assert!(!content.contains("stale"));
        assert_eq!(content.matches("<brief:generated>").count(), 1);
    }

    #[test]
    fn install_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        install_copilot(&full_brief(), dir.path()).unwrap();
        let first =
            std::fs::read_to_string(dir.path().join(".github/copilot-instructions.md")).unwrap();
        install_copilot(&full_brief(), dir.path()).unwrap();
        let second =
            std::fs::read_to_string(dir.path().join(".github/copilot-instructions.md")).unwrap();
        assert_eq!(first, second);
    }
}
