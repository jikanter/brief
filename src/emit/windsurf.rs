use std::path::Path;

use crate::model::Brief;

/// Emit a Windsurf workspace rule (`.windsurf/rules/brief.md`) from a Brief.
///
/// Windsurf reads per-project rules from `.windsurf/rules/*.md`, each carrying
/// YAML frontmatter with a `trigger` activation mode. Brief has no glob-scoping
/// concept, so the only honest mapping is a single bundled rule with
/// `trigger: always_on`. The register is descriptive (Cursor/Copilot-style),
/// not Claude's imperative NEVER/MUST.
///
/// Windsurf caps a workspace rule file at 12,000 characters. This emitter never
/// truncates; the caller's budget check (see [`crate::budget`]) warns on
/// overrun. See docs/design/backends/windsurf/README.md — several specifics
/// there are flagged unverified against the March 2026 audit.
pub fn emit_windsurf(brief: &Brief) -> String {
    let mut out = String::new();

    // Windsurf's frontmatter schema, not brief's.
    out.push_str("---\n");
    out.push_str("trigger: always_on\n");
    out.push_str("---\n\n");

    out.push_str(&format!("# {}\n\n", brief.goal));

    if !brief.frontmatter.stack.is_empty() {
        out.push_str(&format!(
            "**Stack:** {}\n\n",
            brief.frontmatter.stack.join(", ")
        ));
    }

    if !brief.frontmatter.context.is_empty() {
        out.push_str("## Context\n\n");
        for ctx in &brief.frontmatter.context {
            out.push_str(&format!("- `{ctx}`\n"));
        }
        out.push('\n');
    }

    if !brief.constraints.hard.is_empty() {
        out.push_str("## Required\n\n");
        for c in &brief.constraints.hard {
            out.push_str(&format!("- {c}\n"));
        }
        out.push('\n');
    }

    if !brief.constraints.soft.is_empty() {
        out.push_str("## Preferred\n\n");
        for c in &brief.constraints.soft {
            out.push_str(&format!("- {c}\n"));
        }
        out.push('\n');
    }

    if !brief.constraints.ask_first.is_empty() {
        out.push_str("## Ask First\n\n");
        for c in &brief.constraints.ask_first {
            out.push_str(&format!("- {c}\n"));
        }
        out.push('\n');
    }

    if !brief.sacred.is_empty() {
        out.push_str("## Protected Files\n\n");
        for entry in &brief.sacred {
            out.push_str(&format!("- `{}` — {}\n", entry.path, entry.reason));
        }
        out.push('\n');
    }

    let unvalidated: Vec<_> = brief.assumptions.iter().filter(|a| !a.validated).collect();
    if !unvalidated.is_empty() {
        out.push_str("## Verify\n\n");
        for a in &unvalidated {
            out.push_str(&format!("- {}\n", a.text));
        }
        out.push('\n');
    }

    if let Some(ref deliverable) = brief.deliverable {
        out.push_str("## Deliverable\n\n");
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

/// Install a Windsurf rule into `<base>/.windsurf/rules/brief.md`.
///
/// Brief owns this file end-to-end (its name is brief-specific), so each
/// install overwrites it outright — no `<brief:generated>` markers, mirroring
/// the Cursor `.mdc` install. Other rule files in the directory are untouched.
/// The directory is created if it does not already exist.
pub fn install_windsurf(
    brief: &Brief,
    base_dir: &Path,
) -> Result<std::path::PathBuf, std::io::Error> {
    let rules_dir = base_dir.join(".windsurf").join("rules");
    std::fs::create_dir_all(&rules_dir)?;
    let target = rules_dir.join("brief.md");
    std::fs::write(&target, emit_windsurf(brief))?;
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;

    fn full_brief() -> Brief {
        Brief {
            frontmatter: Frontmatter {
                stack: vec!["Rust".into()],
                context: vec!["./docs/arch.md".into()],
                ..Default::default()
            },
            goal: "Add notifications".into(),
            identity: None,
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
            assumptions: vec![
                Assumption {
                    text: "Gateway scales".into(),
                    validated: false,
                    has_checkbox: true,
                },
                Assumption {
                    text: "Already validated".into(),
                    validated: true,
                    has_checkbox: true,
                },
            ],
            deliverable: Some("Working system".into()),
            unknown_sections: vec![UnknownSection {
                heading: "Commands".into(),
                content: "- Build: `cargo build`".into(),
            }],
        }
    }

    #[test]
    fn emit_opens_with_always_on_frontmatter() {
        let output = emit_windsurf(&full_brief());
        assert!(output.starts_with("---\n"));
        assert!(output.contains("trigger: always_on\n"));
        // Frontmatter closes before the body.
        let after_open = &output[4..];
        assert!(after_open.contains("\n---\n"));
    }

    #[test]
    fn emit_renders_goal_and_sections_descriptively() {
        let output = emit_windsurf(&full_brief());
        assert!(output.contains("# Add notifications"));
        assert!(output.contains("## Required"));
        assert!(output.contains("- Must pass CI"));
        assert!(output.contains("## Preferred"));
        assert!(output.contains("## Ask First"));
        assert!(!output.contains("**IMPORTANT:**"));
    }

    #[test]
    fn emit_renders_protected_and_verify() {
        let output = emit_windsurf(&full_brief());
        assert!(output.contains("## Protected Files"));
        assert!(output.contains("`src/auth/**` — Audited"));
        assert!(output.contains("## Verify"));
        assert!(output.contains("Gateway scales"));
        assert!(!output.contains("Already validated"));
    }

    #[test]
    fn install_writes_to_windsurf_rules_brief_md() {
        let dir = tempfile::tempdir().unwrap();
        let written = install_windsurf(&full_brief(), dir.path()).unwrap();
        let expected = dir.path().join(".windsurf").join("rules").join("brief.md");
        assert_eq!(written, expected);
        assert!(expected.exists());
    }

    #[test]
    fn install_overwrites_and_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let rules_dir = dir.path().join(".windsurf").join("rules");
        std::fs::create_dir_all(&rules_dir).unwrap();
        std::fs::write(rules_dir.join("brief.md"), "stale").unwrap();

        install_windsurf(&full_brief(), dir.path()).unwrap();
        let first = std::fs::read_to_string(rules_dir.join("brief.md")).unwrap();
        assert!(!first.contains("stale"));
        assert!(first.contains("# Add notifications"));

        install_windsurf(&full_brief(), dir.path()).unwrap();
        let second = std::fs::read_to_string(rules_dir.join("brief.md")).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn install_leaves_other_rule_files_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let rules_dir = dir.path().join(".windsurf").join("rules");
        std::fs::create_dir_all(&rules_dir).unwrap();
        std::fs::write(rules_dir.join("hand-written.md"), "user rule\n").unwrap();

        install_windsurf(&full_brief(), dir.path()).unwrap();

        assert_eq!(
            std::fs::read_to_string(rules_dir.join("hand-written.md")).unwrap(),
            "user rule\n"
        );
    }
}
