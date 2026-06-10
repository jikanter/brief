use std::path::Path;

use crate::emit::markers::{inject_section, wrap_with_markers};
use crate::model::Brief;

/// Emit an AGENTS.md-compatible section from a Brief.
///
/// AGENTS.md is the cross-vendor convention promoted at agents.md and adopted
/// by OpenAI Codex CLI, Cursor, Amp, Google Jules, and others. Codex CLI in
/// particular reads `AGENTS.md` (and the override variant `AGENTS.override.md`)
/// from the project root downward, plus a global `~/.codex/AGENTS.md`. It
/// passes the file's raw bytes through to the model wrapped in literal
/// `<INSTRUCTIONS>...</INSTRUCTIONS>` tags with no Markdown rendering and no
/// HTML-comment stripping, so brief's `<brief:generated>` markers survive
/// intact — the same encapsulation guarantee as Claude Code's CLAUDE.md.
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
        out.push_str("## Context\n\nRefer to these files for background:\n\n");
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
        out.push_str("## Instructions\n\n");
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
        out.push_str("## Protected Files\n\n");
        for entry in &brief.sacred {
            out.push_str(&format!("- `{}`: {}\n", entry.path, entry.reason));
        }
        out.push('\n');
    }

    // Assumptions
    if !brief.assumptions.is_empty() {
        out.push_str("## Assumptions\n\n");
        for a in &brief.assumptions {
            let marker = if a.validated { "[x]" } else { "[ ]" };
            out.push_str(&format!("- {marker} {}\n", a.text));
        }
        out.push('\n');
    }

    // Deliverable
    if let Some(ref deliverable) = brief.deliverable {
        out.push_str("## Deliverable\n\n");
        out.push_str(deliverable);
        out.push('\n');
    }

    // Unknown sections (passthrough)
    for section in &brief.unknown_sections {
        out.push_str(&format!(
            "\n## {}\n\n{}\n",
            section.heading, section.content
        ));
    }

    out
}

/// Inject a briefing section into an existing AGENTS.md, or create one.
///
/// Behaves identically to `install_claude` for CLAUDE.md: wraps the emitted
/// section in `<brief:generated>` markers, replaces the first existing pair
/// in place (migrating the legacy `<!-- brief:start -->` flavor), strips
/// empty extra pairs, and appends if no markers exist. Codex CLI consumes
/// AGENTS.md without sanitizing the wrapped markers, so the encapsulation
/// round-trip works the same way.
pub fn install_agents_md(brief: &Brief, agents_md_path: &Path) -> Result<String, std::io::Error> {
    let section = emit_agents_md(brief);
    let wrapped = wrap_with_markers(&section);

    let output = if agents_md_path.exists() {
        let existing = std::fs::read_to_string(agents_md_path)?;
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

    std::fs::write(agents_md_path, &output)?;
    Ok(output)
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
        let output = emit_agents_md(&brief);
        assert!(output.contains("Must pass CI **(REQUIRED)**"));
    }

    #[test]
    fn agents_md_emits_unknown_sections() {
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
        let output = emit_agents_md(&brief);
        assert!(output.contains("## Commands"));
        assert!(output.contains("- Build: `cargo build`"));
    }

    #[test]
    fn agents_md_uses_protected_files_heading() {
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
        let output = emit_agents_md(&brief);
        assert!(output.contains("## Protected Files"));
        assert!(output.contains("`src/auth/**`: Auth"));
    }

    fn minimal_brief() -> Brief {
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
    fn install_agents_md_creates_file_when_missing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("AGENTS.md");
        let brief = minimal_brief();

        install_agents_md(&brief, &path).unwrap();

        let result = std::fs::read_to_string(&path).unwrap();
        assert!(result.starts_with("<brief:generated>"));
        assert!(result.contains("# Fix the login bug"));
        assert!(result.contains("Do not break existing tests **(REQUIRED)**"));
        assert!(result.contains("`src/auth.rs`: Tenant resolution"));
        assert!(result.trim_end().ends_with("</brief:generated>"));
    }

    #[test]
    fn install_agents_md_replaces_existing_marker_pair() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("AGENTS.md");
        std::fs::write(
            &path,
            "# My Project\n\nIntro.\n\n<brief:generated>\nstale brief content\n</brief:generated>\n\n## Other\n\nMore.\n",
        )
        .unwrap();
        let brief = minimal_brief();

        install_agents_md(&brief, &path).unwrap();

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
    fn install_agents_md_appends_when_no_markers() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("AGENTS.md");
        std::fs::write(&path, "# My Project\n\nFreeform agent notes.\n").unwrap();
        let brief = minimal_brief();

        install_agents_md(&brief, &path).unwrap();

        let result = std::fs::read_to_string(&path).unwrap();
        assert!(result.starts_with("# My Project"));
        assert!(result.contains("Freeform agent notes."));
        assert!(result.contains("<brief:generated>"));
        assert!(result.contains("# Fix the login bug"));
        assert!(result.trim_end().ends_with("</brief:generated>"));
    }

    #[test]
    fn install_agents_md_migrates_legacy_html_comment_markers() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("AGENTS.md");
        std::fs::write(
            &path,
            "# My Project\n\n<!-- brief:start -->\nold legacy\n<!-- brief:end -->\n\n## Other\n",
        )
        .unwrap();
        let brief = minimal_brief();

        install_agents_md(&brief, &path).unwrap();

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

    // Encapsulation guarantee: Codex CLI wraps AGENTS.md inside
    // `<INSTRUCTIONS>...</INSTRUCTIONS>` before sending to the model. A
    // literal `</INSTRUCTIONS>` substring in our output would collide with
    // that wrapper. Brief never emits one — this test pins that invariant.
    #[test]
    fn install_agents_md_does_not_emit_codex_wrapper_collision() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("AGENTS.md");
        let brief = minimal_brief();

        install_agents_md(&brief, &path).unwrap();

        let result = std::fs::read_to_string(&path).unwrap();
        assert!(!result.contains("</INSTRUCTIONS>"));
        assert!(!result.contains("<INSTRUCTIONS>"));
    }
}
