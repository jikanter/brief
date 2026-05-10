use std::path::Path;

use crate::emit::markers::{inject_section, wrap_with_markers};
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

    // Context
    if !brief.frontmatter.context.is_empty() {
        out.push_str("## Reference Context\n\nRead these files for background before starting work:\n");
        for ctx in &brief.frontmatter.context {
            let clean = ctx.strip_prefix("./").unwrap_or(ctx);
            out.push_str(&format!("- @{clean}\n"));
        }
        out.push('\n');
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
                out.push_str(&format!("- **IMPORTANT:** {c}\n"));
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

    // Unknown sections (passthrough)
    for section in &brief.unknown_sections {
        out.push_str(&format!("\n## {}\n\n{}\n", section.heading, section.content));
    }

    out
}

/// Inject a briefing section into an existing CLAUDE.md, or create one.
///
/// If the file contains `<brief:generated>` / `</brief:generated>` markers
/// (or the legacy `<!-- brief:start -->` / `<!-- brief:end -->` pair), the
/// content between them is replaced with a freshly-emitted, new-format
/// section. Otherwise the marked section is appended to the end of the file.
pub fn install_claude(brief: &Brief, claude_md_path: &Path) -> Result<String, std::io::Error> {
    let section = emit_claude(brief);
    let wrapped = wrap_with_markers(&section);

    let output = if claude_md_path.exists() {
        let existing = std::fs::read_to_string(claude_md_path)?;
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

    std::fs::write(claude_md_path, &output)?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::emit::markers::MARKER_END;
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
        assert!(output.contains("**IMPORTANT:** No breaking changes"));
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

    #[test]
    fn emit_contains_context() {
        let brief = Brief {
            frontmatter: Frontmatter {
                stack: vec!["Rust".into()],
                context: vec![
                    "./docs/architecture.md".into(),
                    "./performance-baseline.csv".into(),
                ],
                ..Default::default()
            },
            goal: "Optimize queries".into(),
            constraints: Constraints::default(),
            sacred: vec![],
            assumptions: vec![],
            deliverable: None,
            unknown_sections: vec![],
        };
        let output = emit_claude(&brief);
        assert!(output.contains("## Reference Context"));
        assert!(output.contains("@docs/architecture.md"));
        assert!(output.contains("@performance-baseline.csv"));
    }

    #[test]
    fn emit_contains_unknown_sections() {
        let brief = Brief {
            frontmatter: Frontmatter::default(),
            goal: "Goal".into(),
            constraints: Constraints::default(),
            sacred: vec![],
            assumptions: vec![],
            deliverable: None,
            unknown_sections: vec![
                UnknownSection {
                    heading: "Commands".into(),
                    content: "- Build: `cargo build`\n- Test: `cargo test`".into(),
                },
                UnknownSection {
                    heading: "Code Style".into(),
                    content: "- Use thiserror for errors".into(),
                },
            ],
        };
        let output = emit_claude(&brief);
        assert!(output.contains("## Commands"));
        assert!(output.contains("- Build: `cargo build`"));
        assert!(output.contains("## Code Style"));
        assert!(output.contains("- Use thiserror for errors"));
    }

    #[test]
    fn emit_omits_context_when_empty() {
        let brief = Brief {
            frontmatter: Frontmatter {
                stack: vec!["Rust".into()],
                context: vec![],
                ..Default::default()
            },
            goal: "Do stuff".into(),
            constraints: Constraints::default(),
            sacred: vec![],
            assumptions: vec![],
            deliverable: None,
            unknown_sections: vec![],
        };
        let output = emit_claude(&brief);
        assert!(!output.contains("Reference Context"));
    }

    // Anchor test: confirms install_claude still uses the shared marker tags.
    #[test]
    fn install_claude_writes_marker_wrapped_content() {
        let dir = tempfile::tempdir().unwrap();
        let claude_md = dir.path().join("CLAUDE.md");
        let brief = Brief {
            frontmatter: Frontmatter::default(),
            goal: "Fix it".into(),
            constraints: Constraints::default(),
            sacred: vec![],
            assumptions: vec![],
            deliverable: None,
            unknown_sections: vec![],
        };
        install_claude(&brief, &claude_md).unwrap();
        let result = std::fs::read_to_string(&claude_md).unwrap();
        assert!(result.contains("<brief:generated>"));
        assert!(result.contains(MARKER_END));
        assert!(result.contains("# Briefing: Fix it"));
    }
}
