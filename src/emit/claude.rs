use std::path::Path;

use crate::model::Brief;

const MARKER_START: &str = "<!-- brief:start -->";
const MARKER_END: &str = "<!-- brief:end -->";

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

/// Wrap emitted content in brief markers.
fn wrap_with_markers(content: &str) -> String {
    format!("{MARKER_START}\n{content}{MARKER_END}\n")
}

/// Inject a briefing section into an existing CLAUDE.md, or create one.
///
/// If the file contains `<!-- brief:start -->` / `<!-- brief:end -->` markers,
/// the content between them is replaced. Otherwise the marked section is
/// appended to the end of the file.
pub fn install_claude(brief: &Brief, claude_md_path: &Path) -> Result<String, std::io::Error> {
    let section = emit_claude(brief);
    let wrapped = wrap_with_markers(&section);

    let output = if claude_md_path.exists() {
        let existing = std::fs::read_to_string(claude_md_path)?;
        inject_section(&existing, &wrapped)
    } else {
        wrapped
    };

    std::fs::write(claude_md_path, &output)?;
    Ok(output)
}

/// Replace content between markers, or append if no markers found.
fn inject_section(existing: &str, wrapped_section: &str) -> String {
    if let (Some(start_idx), Some(end_idx)) = (existing.find(MARKER_START), existing.find(MARKER_END)) {
        let end_idx = end_idx + MARKER_END.len();
        // Consume a trailing newline after the end marker if present
        let end_idx = if existing[end_idx..].starts_with('\n') {
            end_idx + 1
        } else {
            end_idx
        };
        let mut out = String::with_capacity(existing.len());
        out.push_str(&existing[..start_idx]);
        out.push_str(wrapped_section);
        out.push_str(&existing[end_idx..]);
        out
    } else {
        let mut out = existing.to_string();
        if !out.is_empty() && !out.ends_with('\n') {
            out.push('\n');
        }
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(wrapped_section);
        out
    }
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

    #[test]
    fn inject_replaces_existing_markers() {
        let existing = "# My Project\n\nSome intro.\n\n<!-- brief:start -->\nold content\n<!-- brief:end -->\n\n## Other Stuff\n";
        let section = "<!-- brief:start -->\nnew content\n<!-- brief:end -->\n";
        let result = inject_section(existing, section);
        assert!(result.contains("# My Project"));
        assert!(result.contains("new content"));
        assert!(!result.contains("old content"));
        assert!(result.contains("## Other Stuff"));
    }

    #[test]
    fn inject_appends_when_no_markers() {
        let existing = "# My Project\n\nSome intro.\n";
        let section = "<!-- brief:start -->\nbriefing here\n<!-- brief:end -->\n";
        let result = inject_section(existing, section);
        assert!(result.starts_with("# My Project"));
        assert!(result.contains("<!-- brief:start -->"));
        assert!(result.contains("briefing here"));
        assert!(result.ends_with("<!-- brief:end -->\n"));
    }

    #[test]
    fn inject_appends_to_empty() {
        let section = "<!-- brief:start -->\ncontent\n<!-- brief:end -->\n";
        let result = inject_section("", section);
        assert_eq!(result, section);
    }

    #[test]
    fn inject_preserves_content_around_markers() {
        let existing = "before\n<!-- brief:start -->\nold\n<!-- brief:end -->\nafter\n";
        let section = "<!-- brief:start -->\nnew\n<!-- brief:end -->\n";
        let result = inject_section(existing, section);
        assert_eq!(result, "before\n<!-- brief:start -->\nnew\n<!-- brief:end -->\nafter\n");
    }

    #[test]
    fn wrap_with_markers_produces_valid_output() {
        let content = "# Briefing: Test\n\n**Stack:** Rust\n\n";
        let result = wrap_with_markers(content);
        assert!(result.starts_with("<!-- brief:start -->"));
        assert!(result.contains(content));
        assert!(result.ends_with("<!-- brief:end -->\n"));
    }
}
