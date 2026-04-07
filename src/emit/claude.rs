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

/// Find all brief marker pairs in the text.
/// Returns tuples of (pair_start, end_marker_start, pair_end) where:
/// - pair_start: byte offset of the start of MARKER_START
/// - end_marker_start: byte offset of the start of MARKER_END
/// - pair_end: byte offset past MARKER_END (including any trailing newline)
fn find_marker_pairs(text: &str) -> Vec<(usize, usize, usize)> {
    let mut pairs = Vec::new();
    let mut search_from = 0;

    while search_from < text.len() {
        let start = match text[search_from..].find(MARKER_START) {
            Some(offset) => search_from + offset,
            None => break,
        };
        let after_start = start + MARKER_START.len();
        let end_marker_start = match text[after_start..].find(MARKER_END) {
            Some(offset) => after_start + offset,
            None => break,
        };
        let mut pair_end = end_marker_start + MARKER_END.len();
        if pair_end < text.len() && text.as_bytes()[pair_end] == b'\n' {
            pair_end += 1;
        }
        pairs.push((start, end_marker_start, pair_end));
        search_from = pair_end;
    }

    pairs
}

/// Replace content between markers, or append if no markers found.
/// Returns the resulting content and the number of marker pairs found.
fn inject_section(existing: &str, wrapped_section: &str) -> (String, usize) {
    let pairs = find_marker_pairs(existing);

    if pairs.is_empty() {
        let mut out = existing.to_string();
        if !out.is_empty() && !out.ends_with('\n') {
            out.push('\n');
        }
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(wrapped_section);
        return (out, 0);
    }

    let total_pairs = pairs.len();
    let mut out = String::with_capacity(existing.len());
    let mut cursor = 0;

    for (i, &(pair_start, end_marker_start, pair_end)) in pairs.iter().enumerate() {
        out.push_str(&existing[cursor..pair_start]);

        if i == 0 {
            // First pair: replace with new content
            out.push_str(wrapped_section);
        } else {
            // Subsequent pairs: strip if empty, preserve if not
            let content_start = pair_start + MARKER_START.len();
            let between = &existing[content_start..end_marker_start];
            if between.trim().is_empty() {
                // Empty pair: skip entirely
            } else {
                // Non-empty pair: preserve as-is
                out.push_str(&existing[pair_start..pair_end]);
            }
        }

        cursor = pair_end;
    }

    out.push_str(&existing[cursor..]);

    (out, total_pairs)
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
        let (result, pairs) = inject_section(existing, section);
        assert_eq!(pairs, 1);
        assert!(result.contains("# My Project"));
        assert!(result.contains("new content"));
        assert!(!result.contains("old content"));
        assert!(result.contains("## Other Stuff"));
    }

    #[test]
    fn inject_appends_when_no_markers() {
        let existing = "# My Project\n\nSome intro.\n";
        let section = "<!-- brief:start -->\nbriefing here\n<!-- brief:end -->\n";
        let (result, pairs) = inject_section(existing, section);
        assert_eq!(pairs, 0);
        assert!(result.starts_with("# My Project"));
        assert!(result.contains("<!-- brief:start -->"));
        assert!(result.contains("briefing here"));
        assert!(result.ends_with("<!-- brief:end -->\n"));
    }

    #[test]
    fn inject_appends_to_empty() {
        let section = "<!-- brief:start -->\ncontent\n<!-- brief:end -->\n";
        let (result, pairs) = inject_section("", section);
        assert_eq!(pairs, 0);
        assert_eq!(result, section);
    }

    #[test]
    fn inject_preserves_content_around_markers() {
        let existing = "before\n<!-- brief:start -->\nold\n<!-- brief:end -->\nafter\n";
        let section = "<!-- brief:start -->\nnew\n<!-- brief:end -->\n";
        let (result, pairs) = inject_section(existing, section);
        assert_eq!(pairs, 1);
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

    #[test]
    fn inject_multiple_pairs_uses_first_and_strips_empty() {
        let existing = "\
header\n\
<!-- brief:start -->\n\
old content\n\
<!-- brief:end -->\n\
middle\n\
<!-- brief:start -->\n\
<!-- brief:end -->\n\
footer\n";
        let section = "<!-- brief:start -->\nnew content\n<!-- brief:end -->\n";
        let (result, pairs) = inject_section(existing, section);
        assert_eq!(pairs, 2);
        // First pair replaced
        assert!(result.contains("new content"));
        assert!(!result.contains("old content"));
        // Empty second pair stripped
        assert_eq!(result.matches("<!-- brief:start -->").count(), 1);
        assert_eq!(result.matches("<!-- brief:end -->").count(), 1);
        // Surrounding content preserved
        assert!(result.contains("header"));
        assert!(result.contains("middle"));
        assert!(result.contains("footer"));
    }

    #[test]
    fn inject_multiple_pairs_preserves_nonempty_extra() {
        let existing = "\
header\n\
<!-- brief:start -->\n\
old content\n\
<!-- brief:end -->\n\
middle\n\
<!-- brief:start -->\n\
manual notes\n\
<!-- brief:end -->\n\
footer\n";
        let section = "<!-- brief:start -->\nnew content\n<!-- brief:end -->\n";
        let (result, pairs) = inject_section(existing, section);
        assert_eq!(pairs, 2);
        // First pair replaced
        assert!(result.contains("new content"));
        assert!(!result.contains("old content"));
        // Non-empty second pair preserved
        assert!(result.contains("manual notes"));
        assert_eq!(result.matches("<!-- brief:start -->").count(), 2);
    }

    #[test]
    fn inject_three_pairs_strips_all_empty_extras() {
        let existing = "\
<!-- brief:start -->\n\
old\n\
<!-- brief:end -->\n\
between1\n\
<!-- brief:start -->\n\
<!-- brief:end -->\n\
between2\n\
<!-- brief:start -->\n\
<!-- brief:end -->\n\
end\n";
        let section = "<!-- brief:start -->\nnew\n<!-- brief:end -->\n";
        let (result, pairs) = inject_section(existing, section);
        assert_eq!(pairs, 3);
        assert!(result.contains("new"));
        assert!(!result.contains("old"));
        // Both empty extras stripped
        assert_eq!(result.matches("<!-- brief:start -->").count(), 1);
        assert!(result.contains("between1"));
        assert!(result.contains("between2"));
        assert!(result.contains("end"));
    }

    #[test]
    fn find_marker_pairs_finds_all() {
        let text = "a\n<!-- brief:start -->\nb\n<!-- brief:end -->\nc\n<!-- brief:start -->\n<!-- brief:end -->\nd\n";
        let pairs = find_marker_pairs(text);
        assert_eq!(pairs.len(), 2);
    }

    #[test]
    fn find_marker_pairs_handles_no_markers() {
        let pairs = find_marker_pairs("just some text\n");
        assert!(pairs.is_empty());
    }

    #[test]
    fn find_marker_pairs_handles_unmatched_start() {
        let text = "<!-- brief:start -->\nno end marker\n";
        let pairs = find_marker_pairs(text);
        assert!(pairs.is_empty());
    }
}
