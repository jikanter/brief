//! XML emit target — Anthropic-style XML tags for system prompts.
//!
//! Anthropic's prompting docs recommend XML tags as the canonical structuring
//! convention for instructions. This emitter renders a `Brief` into an
//! XML-tagged form suitable for piping into a Claude API system prompt. It is
//! intentionally not an XML document (no `<?xml ?>` declaration, no DTD): the
//! output is prompt content for an LLM, not data for an XML parser.

use crate::framing::{frame_ask_first, frame_hard, frame_soft};
use crate::model::{Brief, Constraint};

/// Render a constraint's path scope as an XML attribute (` scope="a b"`), or an
/// empty string when the constraint is project-wide. Globs are space-separated
/// and XML-escaped inside the attribute value.
fn scope_attr(c: &Constraint) -> String {
    if c.scope.is_empty() {
        return String::new();
    }
    format!(" scope=\"{}\"", escape_xml(&c.scope.join(" ")))
}

/// Emit an XML-tagged briefing suitable for direct use in an Anthropic API
/// system prompt.
pub fn emit_xml(brief: &Brief) -> String {
    let mut out = String::new();

    out.push_str("<brief>\n");

    out.push_str(&format!("<goal>{}</goal>\n", escape_xml(&brief.goal)));

    if !brief.frontmatter.stack.is_empty() {
        out.push_str(&format!(
            "<stack>{}</stack>\n",
            escape_xml(&brief.frontmatter.stack.join(", "))
        ));
    }

    if !brief.frontmatter.context.is_empty() {
        out.push_str("\n<context>\n");
        for ctx in &brief.frontmatter.context {
            let clean = ctx.strip_prefix("./").unwrap_or(ctx);
            out.push_str(&format!("<file>{}</file>\n", escape_xml(clean)));
        }
        out.push_str("</context>\n");
    }

    let has_constraints = !brief.constraints.hard.is_empty()
        || !brief.constraints.soft.is_empty()
        || !brief.constraints.ask_first.is_empty();

    if has_constraints {
        out.push_str("\n<constraints>\n");

        if !brief.constraints.hard.is_empty() {
            out.push_str("<hard>\n");
            for c in &brief.constraints.hard {
                out.push_str(&format!(
                    "<rule{}>{}</rule>\n",
                    scope_attr(c),
                    escape_xml(&frame_hard(c))
                ));
            }
            out.push_str("</hard>\n");
        }

        if !brief.constraints.soft.is_empty() {
            out.push_str("<soft>\n");
            for c in &brief.constraints.soft {
                out.push_str(&format!(
                    "<rule{}>{}</rule>\n",
                    scope_attr(c),
                    escape_xml(&frame_soft(c))
                ));
            }
            out.push_str("</soft>\n");
        }

        if !brief.constraints.ask_first.is_empty() {
            out.push_str("<ask-first>\n");
            for c in &brief.constraints.ask_first {
                out.push_str(&format!(
                    "<rule{}>{}</rule>\n",
                    scope_attr(c),
                    escape_xml(&frame_ask_first(c))
                ));
            }
            out.push_str("</ask-first>\n");
        }

        out.push_str("</constraints>\n");
    }

    if !brief.sacred.is_empty() {
        out.push_str("\n<sacred>\n");
        for entry in &brief.sacred {
            out.push_str(&format!(
                "<region path=\"{}\">{}</region>\n",
                escape_xml_attr(&entry.path),
                escape_xml(&entry.reason)
            ));
        }
        out.push_str("</sacred>\n");
    }

    if !brief.assumptions.is_empty() {
        out.push_str("\n<assumptions>\n");
        for a in brief.assumptions.iter().filter(|a| !a.validated) {
            out.push_str(&format!(
                "<unvalidated>{}</unvalidated>\n",
                escape_xml(&a.text)
            ));
        }
        for a in brief.assumptions.iter().filter(|a| a.validated) {
            out.push_str(&format!("<validated>{}</validated>\n", escape_xml(&a.text)));
        }
        out.push_str("</assumptions>\n");
    }

    if let Some(ref deliverable) = brief.deliverable {
        out.push_str(&format!(
            "\n<deliverable>{}</deliverable>\n",
            escape_xml(deliverable.trim_end_matches('\n'))
        ));
    }

    for section in &brief.unknown_sections {
        out.push_str(&format!(
            "\n<section name=\"{}\">{}</section>\n",
            escape_xml_attr(&section.heading),
            escape_xml(&section.content)
        ));
    }

    out.push_str("</brief>\n");

    out
}

/// Escape `&`, `<`, `>` for safe inclusion in XML element text.
fn escape_xml(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(c),
        }
    }
    out
}

/// Escape characters unsafe inside a double-quoted XML attribute value.
fn escape_xml_attr(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;

    fn empty_brief() -> Brief {
        Brief {
            frontmatter: Frontmatter::default(),
            goal: "Goal".into(),
            identity: None,
            constraints: Constraints::default(),
            sacred: vec![],
            assumptions: vec![],
            deliverable: None,
            unknown_sections: vec![],
        }
    }

    #[test]
    fn xml_contains_goal_and_stack() {
        let brief = Brief {
            frontmatter: Frontmatter {
                stack: vec!["Rust".into(), "PostgreSQL".into()],
                ..Default::default()
            },
            goal: "Build a thing".into(),
            identity: None,
            ..empty_brief()
        };
        let output = emit_xml(&brief);
        assert!(output.starts_with("<brief>\n"));
        assert!(output.contains("<goal>Build a thing</goal>"));
        assert!(output.contains("<stack>Rust, PostgreSQL</stack>"));
        assert!(output.trim_end().ends_with("</brief>"));
    }

    #[test]
    fn xml_contains_constraints_with_reframing() {
        let brief = Brief {
            goal: "G".into(),
            identity: None,
            constraints: Constraints {
                hard: vec![
                    "Pass the existing CI suite".into(),
                    "No breaking schema changes".into(),
                ],
                soft: vec![
                    "Use async where reasonable".into(),
                    "Keep modules under 200 lines".into(),
                ],
                ask_first: vec!["Changing the schema".into()],
            },
            ..empty_brief()
        };
        let output = emit_xml(&brief);
        assert!(output.contains("<rule>MUST: Pass the existing CI suite</rule>"));
        assert!(output.contains("<rule>NEVER: breaking schema changes</rule>"));
        assert!(output.contains("<rule>PREFER: Use async where reasonable</rule>"));
        assert!(output.contains("<rule>PREFER: Keep modules under 200 lines</rule>"));
        assert!(
            output.contains(
                "<rule>STOP and confirm with the user before: Changing the schema</rule>"
            )
        );
    }

    #[test]
    fn xml_does_not_double_prefix_imperative_constraints() {
        let brief = Brief {
            goal: "G".into(),
            identity: None,
            constraints: Constraints {
                hard: vec!["MUST: ship it".into(), "NEVER: leak PII".into()],
                soft: vec!["PREFER: small modules".into()],
                ask_first: vec!["STOP: schema changes".into()],
            },
            ..empty_brief()
        };
        let output = emit_xml(&brief);
        assert!(output.contains("<rule>MUST: ship it</rule>"));
        assert!(output.contains("<rule>NEVER: leak PII</rule>"));
        assert!(output.contains("<rule>PREFER: small modules</rule>"));
        assert!(output.contains("<rule>STOP: schema changes</rule>"));
        // No double prefix.
        assert!(!output.contains("MUST: MUST"));
        assert!(!output.contains("NEVER: NEVER"));
        assert!(!output.contains("PREFER: PREFER"));
        assert!(!output.contains("STOP and confirm with the user before: STOP"));
    }

    #[test]
    fn xml_contains_sacred_regions() {
        let brief = Brief {
            goal: "G".into(),
            identity: None,
            sacred: vec![SacredEntry {
                path: "src/auth/**".into(),
                reason: "Audited".into(),
                well_formed: true,
            }],
            ..empty_brief()
        };
        let output = emit_xml(&brief);
        assert!(output.contains("<sacred>"));
        assert!(output.contains("<region path=\"src/auth/**\">Audited</region>"));
        assert!(output.contains("</sacred>"));
    }

    #[test]
    fn xml_separates_assumptions() {
        let brief = Brief {
            goal: "G".into(),
            identity: None,
            assumptions: vec![
                Assumption {
                    text: "Unvalidated A".into(),
                    validated: false,
                    has_checkbox: true,
                },
                Assumption {
                    text: "Validated B".into(),
                    validated: true,
                    has_checkbox: true,
                },
            ],
            ..empty_brief()
        };
        let output = emit_xml(&brief);
        assert!(output.contains("<unvalidated>Unvalidated A</unvalidated>"));
        assert!(output.contains("<validated>Validated B</validated>"));
        // Unvalidated should appear before validated in output.
        let u = output.find("<unvalidated>").unwrap();
        let v = output.find("<validated>").unwrap();
        assert!(u < v);
    }

    #[test]
    fn xml_omits_empty_sections() {
        let brief = empty_brief();
        let output = emit_xml(&brief);
        assert!(output.contains("<goal>Goal</goal>"));
        assert!(!output.contains("<stack>"));
        assert!(!output.contains("<context>"));
        assert!(!output.contains("<constraints>"));
        assert!(!output.contains("<sacred>"));
        assert!(!output.contains("<assumptions>"));
        assert!(!output.contains("<deliverable>"));
    }

    #[test]
    fn xml_contains_unknown_sections() {
        let brief = Brief {
            goal: "G".into(),
            identity: None,
            unknown_sections: vec![UnknownSection {
                heading: "Commands".into(),
                content: "- Build: `cargo build`".into(),
            }],
            ..empty_brief()
        };
        let output = emit_xml(&brief);
        assert!(output.contains("<section name=\"Commands\">"));
        assert!(output.contains("- Build: `cargo build`"));
        assert!(output.contains("</section>"));
    }

    #[test]
    fn xml_escapes_special_characters() {
        let brief = Brief {
            goal: "Fix A & B < C > D".into(),
            identity: None,
            constraints: Constraints {
                hard: vec!["Pipe with | and < or >".into()],
                ..Default::default()
            },
            sacred: vec![SacredEntry {
                path: "src/<weird>/**".into(),
                reason: "R & R".into(),
                well_formed: true,
            }],
            ..empty_brief()
        };
        let output = emit_xml(&brief);
        assert!(output.contains("<goal>Fix A &amp; B &lt; C &gt; D</goal>"));
        assert!(output.contains("&lt; or &gt;"));
        assert!(output.contains("path=\"src/&lt;weird&gt;/**\""));
        assert!(output.contains(">R &amp; R</region>"));
    }

    #[test]
    fn xml_contains_context_files() {
        let brief = Brief {
            frontmatter: Frontmatter {
                stack: vec!["Rust".into()],
                context: vec![
                    "./docs/architecture.md".into(),
                    "performance-baseline.csv".into(),
                ],
                ..Default::default()
            },
            goal: "G".into(),
            identity: None,
            ..empty_brief()
        };
        let output = emit_xml(&brief);
        assert!(output.contains("<context>"));
        assert!(output.contains("<file>docs/architecture.md</file>"));
        assert!(output.contains("<file>performance-baseline.csv</file>"));
        assert!(output.contains("</context>"));
    }

    #[test]
    fn xml_omits_stack_when_empty() {
        let brief = empty_brief();
        let output = emit_xml(&brief);
        assert!(!output.contains("<stack>"));
    }

    #[test]
    fn xml_includes_deliverable_when_present() {
        let brief = Brief {
            goal: "G".into(),
            identity: None,
            deliverable: Some("Ship a green CI".into()),
            ..empty_brief()
        };
        let output = emit_xml(&brief);
        assert!(output.contains("<deliverable>Ship a green CI</deliverable>"));
    }

    #[test]
    fn reframing_leaves_already_framed_text_alone() {
        use crate::framing::frame_hard;
        // Text already opening with a canonical prefix (NEVER:/MUST:/AVOID:) is
        // returned as-is — the emitter does not double-prefix.
        assert_eq!(frame_hard("MUST: run linter"), "MUST: run linter");
        assert_eq!(frame_hard("NEVER: leak PII"), "NEVER: leak PII");

        // Negation word-forms are folded into NEVER (the negation lead is dropped).
        assert_eq!(frame_hard("Do not push to main"), "NEVER: push to main");
        assert_eq!(
            frame_hard("Never block the event loop"),
            "NEVER: block the event loop"
        );
        assert_eq!(frame_hard("Avoid lodash"), "NEVER: lodash");

        // Positive requirement with no imperative gets MUST:.
        assert_eq!(
            frame_hard("All endpoints return JSON"),
            "MUST: All endpoints return JSON"
        );

        // Bare negation gets NEVER: with the "no " lead folded in.
        assert_eq!(frame_hard("No silent failures"), "NEVER: silent failures");
    }
}
