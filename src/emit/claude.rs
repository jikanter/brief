use std::path::Path;

use crate::emit::markers::{inject_section, wrap_with_markers};
use crate::framing::{frame_ask_first, frame_hard, frame_soft};
use crate::model::Brief;

/// Emit a CLAUDE.md-compatible section from a Brief.
///
/// The Markdown skeleton (`# Briefing`, `## Constraints`, `## Sacred Regions`)
/// is preserved so the file still reads cleanly to humans. Each section's
/// body is wrapped in an Anthropic-canonical XML tag (`<context>`, `<rules>`,
/// `<protected_files>`, `<assumptions>`, `<deliverable>`) so Claude — which
/// has strong priors on those tag names from Anthropic's own prompting
/// guidance — can latch onto the structure when CLAUDE.md is passed through
/// to the model.
pub fn emit_claude(brief: &Brief) -> String {
    let mut out = String::new();

    out.push_str(&format!("# Briefing: {}\n\n", brief.goal));

    // Stack — a single line, kept near the top for the human reader.
    if !brief.frontmatter.stack.is_empty() {
        out.push_str(&format!(
            "**Stack:** {}\n\n",
            brief.frontmatter.stack.join(", ")
        ));
    }

    // Constraints come right after the goal (the P0 "compromise order"): CLAUDE.md
    // is read by humans too, so goal leads, but constraints sit ahead of the
    // reference material rather than after it. Each tier is framed by polarity /
    // register (NEVER/MUST/PREFER/STOP) and wrapped in <rules priority="...">.
    let has_constraints = !brief.constraints.hard.is_empty()
        || !brief.constraints.soft.is_empty()
        || !brief.constraints.ask_first.is_empty();

    if has_constraints {
        out.push_str("## Constraints\n\n");

        if !brief.constraints.hard.is_empty() {
            out.push_str("### Hard (Non-negotiable)\n\n");
            out.push_str("<rules priority=\"required\">\n");
            for c in &brief.constraints.hard {
                out.push_str(&format!("- {}\n", frame_hard(c)));
            }
            out.push_str("</rules>\n\n");
        }

        if !brief.constraints.soft.is_empty() {
            out.push_str("### Soft (Preferred)\n\n");
            out.push_str("<rules priority=\"preferred\">\n");
            for c in &brief.constraints.soft {
                out.push_str(&format!("- {}\n", frame_soft(c)));
            }
            out.push_str("</rules>\n\n");
        }

        if !brief.constraints.ask_first.is_empty() {
            out.push_str("### Ask First (Requires approval)\n\n");
            out.push_str("<rules priority=\"ask-first\">\n");
            for c in &brief.constraints.ask_first {
                out.push_str(&format!("- {}\n", frame_ask_first(c)));
            }
            out.push_str("</rules>\n\n");
        }
    }

    // Sacred — wrapped in <protected_files>, with a preamble that removes
    // ambiguity ("under any circumstances") and gives the model an action to
    // take ("STOP and report") instead of pure suppression.
    if !brief.sacred.is_empty() {
        out.push_str("## Sacred Regions (Do Not Modify)\n\n");
        out.push_str(
            "These files must not be modified under any circumstances. If a task requires changing one, STOP and report the conflict instead of proceeding.\n\n",
        );
        out.push_str("<protected_files>\n");
        for entry in &brief.sacred {
            out.push_str(&format!("- `{}` — {}\n", entry.path, entry.reason));
        }
        out.push_str("</protected_files>\n\n");
    }

    // Reference Context — wrapped in <context> per Anthropic's prompting guide.
    if !brief.frontmatter.context.is_empty() {
        out.push_str(
            "## Reference Context\n\nRead these files for background before starting work:\n\n",
        );
        out.push_str("<context>\n");
        for ctx in &brief.frontmatter.context {
            let clean = ctx.strip_prefix("./").unwrap_or(ctx);
            out.push_str(&format!("- @{clean}\n"));
        }
        out.push_str("</context>\n\n");
    }

    // Identity — H2 heading with optional plain text beneath it.
    if let Some(ref identity) = brief.identity {
        out.push_str(&format!("## {}\n\n", identity.heading));
        if !identity.content.is_empty() {
            out.push_str(&format!("{}\n\n", identity.content));
        }
    }

    // Assumptions — wrapped in <assumptions>.
    if !brief.assumptions.is_empty() {
        out.push_str("## Assumptions\n\n");
        out.push_str("<assumptions>\n");
        for a in &brief.assumptions {
            let marker = if a.validated { "[x]" } else { "[ ]" };
            out.push_str(&format!("- {marker} {}\n", a.text));
        }
        out.push_str("</assumptions>\n\n");
    }

    // Deliverable — wrapped in <deliverable>.
    if let Some(ref deliverable) = brief.deliverable {
        out.push_str("## Deliverable\n\n");
        out.push_str("<deliverable>\n");
        out.push_str(deliverable.trim_end_matches('\n'));
        out.push_str("\n</deliverable>\n");
    }

    // Unknown sections (passthrough — no XML wrapping, format-extensibility
    // surface stays untouched).
    for section in &brief.unknown_sections {
        out.push_str(&format!(
            "\n## {}\n\n{}\n",
            section.heading, section.content
        ));
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
            identity: Some(Identity {
                heading: "Identity".into(),
                content: "A Minimal Project".into(),
            }),
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
            identity: None,
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
        // P0 framing: prohibition → NEVER (negation folded), soft → PREFER,
        // ask-first → STOP. No more **IMPORTANT:** prefix.
        assert!(output.contains("NEVER: breaking changes"));
        assert!(!output.contains("**IMPORTANT:**"));
        assert!(output.contains("PREFER: async"));
        assert!(output.contains("STOP and confirm with the user before: Schema changes"));
    }

    #[test]
    fn emit_contains_sacred() {
        let brief = Brief {
            frontmatter: Frontmatter::default(),
            identity: None,
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
            identity: None,
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
            identity: None,
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
            identity: None,
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

    #[test]
    fn emit_wraps_constraints_in_rules_tags() {
        let brief = Brief {
            frontmatter: Frontmatter::default(),
            identity: None,
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
        assert!(output.contains("<rules priority=\"required\">"));
        assert!(output.contains("<rules priority=\"preferred\">"));
        assert!(output.contains("<rules priority=\"ask-first\">"));
        assert_eq!(output.matches("</rules>").count(), 3);
        // Markdown headings still present alongside the tags.
        assert!(output.contains("### Hard (Non-negotiable)"));
        assert!(output.contains("### Soft (Preferred)"));
        assert!(output.contains("### Ask First (Requires approval)"));
    }

    #[test]
    fn emit_wraps_sacred_in_protected_files_tag() {
        let brief = Brief {
            frontmatter: Frontmatter::default(),
            identity: None,
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
        let output = emit_claude(&brief);
        assert!(output.contains("<protected_files>"));
        assert!(output.contains("- `src/auth/**` — Auth"));
        assert!(output.contains("</protected_files>"));
    }

    #[test]
    fn emit_wraps_context_in_context_tag() {
        let brief = Brief {
            frontmatter: Frontmatter {
                stack: vec!["Rust".into()],
                context: vec!["./docs/arch.md".into()],
                ..Default::default()
            },
            identity: None,
            goal: "Goal".into(),
            constraints: Constraints::default(),
            sacred: vec![],
            assumptions: vec![],
            deliverable: None,
            unknown_sections: vec![],
        };
        let output = emit_claude(&brief);
        assert!(output.contains("<context>"));
        assert!(output.contains("- @docs/arch.md"));
        assert!(output.contains("</context>"));
    }

    #[test]
    fn emit_wraps_assumptions_and_deliverable() {
        let brief = Brief {
            frontmatter: Frontmatter::default(),
            identity: None,
            goal: "Goal".into(),
            constraints: Constraints::default(),
            sacred: vec![],
            assumptions: vec![Assumption {
                text: "Cache hits 80%".into(),
                validated: false,
                has_checkbox: true,
            }],
            deliverable: Some("Working pipeline".into()),
            unknown_sections: vec![],
        };
        let output = emit_claude(&brief);
        assert!(output.contains("<assumptions>"));
        assert!(output.contains("- [ ] Cache hits 80%"));
        assert!(output.contains("</assumptions>"));
        assert!(output.contains("<deliverable>"));
        assert!(output.contains("Working pipeline"));
        assert!(output.contains("</deliverable>"));
    }

    #[test]
    fn emit_omits_xml_tags_for_empty_sections() {
        let brief = Brief {
            frontmatter: Frontmatter::default(),
            identity: None,
            goal: "Goal".into(),
            constraints: Constraints::default(),
            sacred: vec![],
            assumptions: vec![],
            deliverable: None,
            unknown_sections: vec![],
        };
        let output = emit_claude(&brief);
        assert!(!output.contains("<rules"));
        assert!(!output.contains("<protected_files>"));
        assert!(!output.contains("<context>"));
        assert!(!output.contains("<assumptions>"));
        assert!(!output.contains("<deliverable>"));
    }

    // Anchor test: confirms install_claude still uses the shared marker tags.
    #[test]
    fn install_claude_writes_marker_wrapped_content() {
        let dir = tempfile::tempdir().unwrap();
        let claude_md = dir.path().join("CLAUDE.md");
        let brief = Brief {
            frontmatter: Frontmatter::default(),
            identity: None,
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
