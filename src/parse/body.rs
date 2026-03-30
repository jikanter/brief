use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};

use crate::model::*;

#[derive(Debug)]
enum Section {
    None,
    Constraints,
    Sacred,
    Assumptions,
    Deliverable,
    Unknown(String),
}

#[derive(Debug)]
enum ConstraintType {
    Hard,
    Soft,
    AskFirst,
}

/// Intermediate result from body parsing (before combining with frontmatter).
pub struct ParsedBody {
    pub goal: Option<String>,
    pub constraints: Constraints,
    pub sacred: Vec<SacredEntry>,
    pub assumptions: Vec<Assumption>,
    pub deliverable: Option<String>,
    pub unknown_sections: Vec<UnknownSection>,
}

/// Parse the markdown body (after frontmatter extraction) into structured sections.
pub fn parse_body(input: &str) -> ParsedBody {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TASKLISTS);
    let parser = Parser::new_ext(input, options).into_offset_iter();

    let mut goal: Option<String> = None;
    let mut unknown_sections: Vec<UnknownSection> = Vec::new();
    let mut state = ParseState {
        constraints: Constraints::default(),
        sacred: Vec::new(),
        assumptions: Vec::new(),
        deliverable_parts: Vec::new(),
    };

    let mut current_section = Section::None;
    let mut current_constraint_type: Option<ConstraintType> = None;

    // Heading tracking
    let mut in_heading = false;
    let mut heading_text = String::new();

    // List item tracking
    let mut in_item = false;
    let mut item_texts: Vec<ItemSegment> = Vec::new();
    let mut task_marker: Option<bool> = None;

    // Content collection for deliverable paragraphs
    let mut in_paragraph = false;

    // Raw Markdown capture for unknown sections: byte offset where content starts
    let mut unknown_section_start: Option<usize> = None;

    for (event, range) in parser {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                in_heading = true;
                heading_text.clear();

                // Finalize unknown section on new H1/H2
                if level <= HeadingLevel::H2 {
                    if let (Section::Unknown(ref name), Some(start)) =
                        (&current_section, unknown_section_start)
                    {
                        let raw = input[start..range.start].trim();
                        if !raw.is_empty() {
                            unknown_sections.push(UnknownSection {
                                heading: name.clone(),
                                content: raw.to_string(),
                            });
                        }
                        unknown_section_start = None;
                    }
                }
            }

            Event::End(TagEnd::Heading(level)) => {
                in_heading = false;
                let text = heading_text.trim().to_string();

                match level {
                    HeadingLevel::H1 => {
                        if goal.is_none() {
                            goal = Some(text);
                        }
                    }
                    HeadingLevel::H2 => {
                        match text.as_str() {
                            "Constraints" => current_section = Section::Constraints,
                            "Sacred" => current_section = Section::Sacred,
                            "Assumptions" => current_section = Section::Assumptions,
                            "Deliverable" => {
                                current_section = Section::Deliverable;
                                state.deliverable_parts.clear();
                            }
                            _ => {
                                current_section = Section::Unknown(text);
                                unknown_section_start = Some(range.end);
                            }
                        }
                        current_constraint_type = None;
                    }
                    HeadingLevel::H3 => {
                        if matches!(current_section, Section::Constraints) {
                            current_constraint_type = match text.as_str() {
                                "Hard" => Some(ConstraintType::Hard),
                                "Soft" => Some(ConstraintType::Soft),
                                "Ask First" => Some(ConstraintType::AskFirst),
                                _ => None,
                            };
                        }
                    }
                    _ => {}
                }
            }

            Event::Text(text) if in_heading => {
                heading_text.push_str(&text);
            }

            Event::Start(Tag::Item) if !matches!(current_section, Section::Unknown(_)) => {
                in_item = true;
                item_texts.clear();
                task_marker = None;
            }

            Event::TaskListMarker(checked) if in_item => {
                task_marker = Some(checked);
            }

            Event::Code(code) if in_item => {
                item_texts.push(ItemSegment::Code(code.to_string()));
            }

            Event::Text(text) if in_item => {
                item_texts.push(ItemSegment::Text(text.to_string()));
            }

            Event::SoftBreak if in_item => {
                item_texts.push(ItemSegment::Text(" ".to_string()));
            }

            Event::End(TagEnd::Item) if in_item => {
                in_item = false;
                process_item(
                    &current_section,
                    &current_constraint_type,
                    &item_texts,
                    task_marker,
                    &mut state,
                );
            }

            // Content outside list items (e.g., deliverable paragraphs)
            Event::Start(Tag::Paragraph)
                if !in_item
                    && !in_heading
                    && !matches!(current_section, Section::Unknown(_)) =>
            {
                in_paragraph = true;
            }

            Event::End(TagEnd::Paragraph) if in_paragraph => {
                in_paragraph = false;
            }

            Event::Text(text) if in_paragraph => {
                if matches!(current_section, Section::Deliverable) {
                    state.deliverable_parts.push(text.to_string());
                }
            }

            Event::Code(code) if in_paragraph => {
                if matches!(current_section, Section::Deliverable) {
                    state.deliverable_parts.push(format!("`{code}`"));
                }
            }

            Event::SoftBreak if in_paragraph => {
                if matches!(current_section, Section::Deliverable) {
                    state.deliverable_parts.push("\n".to_string());
                }
            }

            _ => {}
        }
    }

    // Finalize any trailing unknown section
    if let (Section::Unknown(ref name), Some(start)) = (&current_section, unknown_section_start) {
        let raw = input[start..].trim();
        if !raw.is_empty() {
            unknown_sections.push(UnknownSection {
                heading: name.clone(),
                content: raw.to_string(),
            });
        }
    }

    let deliverable = if state.deliverable_parts.is_empty() {
        None
    } else {
        let text = state.deliverable_parts.join("").trim().to_string();
        if text.is_empty() {
            None
        } else {
            Some(text)
        }
    };

    ParsedBody {
        goal,
        constraints: state.constraints,
        sacred: state.sacred,
        assumptions: state.assumptions,
        deliverable,
        unknown_sections,
    }
}

/// Segments within a list item, preserving code vs text distinction.
#[derive(Debug)]
enum ItemSegment {
    Text(String),
    Code(String),
}

/// Mutable state accumulated during body parsing.
struct ParseState {
    constraints: Constraints,
    sacred: Vec<SacredEntry>,
    assumptions: Vec<Assumption>,
    deliverable_parts: Vec<String>,
}

fn process_item(
    section: &Section,
    constraint_type: &Option<ConstraintType>,
    segments: &[ItemSegment],
    task_marker: Option<bool>,
    state: &mut ParseState,
) {
    match section {
        Section::Constraints => {
            let text = segments_to_plain_text(segments);
            if !text.is_empty() {
                match constraint_type {
                    Some(ConstraintType::Hard) => state.constraints.hard.push(text),
                    Some(ConstraintType::Soft) => state.constraints.soft.push(text),
                    Some(ConstraintType::AskFirst) => state.constraints.ask_first.push(text),
                    None => {} // no recognized H3 — validator will catch
                }
            }
        }
        Section::Sacred => {
            parse_sacred_item(segments, &mut state.sacred);
        }
        Section::Assumptions => {
            let text = segments_to_plain_text(segments);
            if !text.is_empty() {
                state.assumptions.push(Assumption {
                    text,
                    validated: task_marker.unwrap_or(false),
                    has_checkbox: task_marker.is_some(),
                });
            }
        }
        Section::Deliverable => {
            state.deliverable_parts.push(segments_to_plain_text(segments));
        }
        // Unknown sections use raw Markdown capture, not event-based accumulation
        Section::Unknown(_) | Section::None => {}
    }
}

fn segments_to_plain_text(segments: &[ItemSegment]) -> String {
    let mut out = String::new();
    for seg in segments {
        match seg {
            ItemSegment::Text(t) => out.push_str(t),
            ItemSegment::Code(c) => {
                out.push('`');
                out.push_str(c);
                out.push('`');
            }
        }
    }
    out.trim().to_string()
}

fn parse_sacred_item(segments: &[ItemSegment], sacred: &mut Vec<SacredEntry>) {
    // Well-formed: first segment is Code (the path), followed by Text with separator
    let first_is_code = matches!(segments.first(), Some(ItemSegment::Code(_)));

    if first_is_code {
        let path = match &segments[0] {
            ItemSegment::Code(c) => c.clone(),
            _ => unreachable!(),
        };

        // Remaining segments form the reason (strip leading separator)
        let rest: String = segments[1..]
            .iter()
            .map(|s| match s {
                ItemSegment::Text(t) => t.as_str(),
                ItemSegment::Code(c) => c.as_str(),
            })
            .collect::<Vec<_>>()
            .join("");
        let rest = rest.trim();
        let reason = strip_separator(rest);

        sacred.push(SacredEntry {
            path,
            reason,
            well_formed: true,
        });
    } else {
        // Malformed: no backtick-wrapped path. Try to split on separator.
        let full = segments_to_plain_text(segments);
        let (path, reason) = split_on_separator(&full);
        sacred.push(SacredEntry {
            path,
            reason,
            well_formed: false,
        });
    }
}

fn strip_separator(s: &str) -> String {
    s.strip_prefix('—')
        .or_else(|| s.strip_prefix("--"))
        .unwrap_or(s)
        .trim()
        .to_string()
}

fn split_on_separator(s: &str) -> (String, String) {
    if let Some((path, reason)) = s.split_once('—') {
        (path.trim().to_string(), reason.trim().to_string())
    } else if let Some((path, reason)) = s.split_once("--") {
        (path.trim().to_string(), reason.trim().to_string())
    } else {
        (s.trim().to_string(), String::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_h1_goal() {
        let body = parse_body("# Fix the login bug\n");
        assert_eq!(body.goal, Some("Fix the login bug".to_string()));
    }

    #[test]
    fn no_h1_returns_none() {
        let body = parse_body("## Just an H2\n\nSome text.\n");
        assert_eq!(body.goal, None);
    }

    #[test]
    fn parse_hard_constraints() {
        let md = "## Constraints\n\n### Hard\n- Do not break tests\n- Keep backward compat\n";
        let body = parse_body(md);
        assert_eq!(body.constraints.hard.len(), 2);
        assert_eq!(body.constraints.hard[0], "Do not break tests");
        assert_eq!(body.constraints.hard[1], "Keep backward compat");
    }

    #[test]
    fn parse_all_constraint_types() {
        let md = "## Constraints\n\n### Hard\n- H1\n\n### Soft\n- S1\n\n### Ask First\n- A1\n";
        let body = parse_body(md);
        assert_eq!(body.constraints.hard, vec!["H1"]);
        assert_eq!(body.constraints.soft, vec!["S1"]);
        assert_eq!(body.constraints.ask_first, vec!["A1"]);
    }

    #[test]
    fn parse_well_formed_sacred() {
        let md = "## Sacred\n- `src/auth/**` — Authentication logic\n";
        let body = parse_body(md);
        assert_eq!(body.sacred.len(), 1);
        assert_eq!(body.sacred[0].path, "src/auth/**");
        assert_eq!(body.sacred[0].reason, "Authentication logic");
        assert!(body.sacred[0].well_formed);
    }

    #[test]
    fn parse_sacred_with_double_hyphen() {
        let md = "## Sacred\n- `migrations/` -- Never alter\n";
        let body = parse_body(md);
        assert_eq!(body.sacred[0].path, "migrations/");
        assert_eq!(body.sacred[0].reason, "Never alter");
        assert!(body.sacred[0].well_formed);
    }

    #[test]
    fn parse_malformed_sacred_no_backticks() {
        let md = "## Sacred\n- not-in-backticks — missing backtick wrapping\n";
        let body = parse_body(md);
        assert_eq!(body.sacred.len(), 1);
        assert!(!body.sacred[0].well_formed);
        assert_eq!(body.sacred[0].path, "not-in-backticks");
        assert_eq!(body.sacred[0].reason, "missing backtick wrapping");
    }

    #[test]
    fn parse_validated_assumption() {
        let md = "## Assumptions\n- [x] This is validated\n";
        let body = parse_body(md);
        assert_eq!(body.assumptions.len(), 1);
        assert!(body.assumptions[0].validated);
        assert!(body.assumptions[0].has_checkbox);
        assert_eq!(body.assumptions[0].text, "This is validated");
    }

    #[test]
    fn parse_unvalidated_assumption() {
        let md = "## Assumptions\n- [ ] Not yet validated\n";
        let body = parse_body(md);
        assert_eq!(body.assumptions.len(), 1);
        assert!(!body.assumptions[0].validated);
        assert!(body.assumptions[0].has_checkbox);
    }

    #[test]
    fn parse_assumption_without_checkbox() {
        let md = "## Assumptions\n- Missing checkbox syntax\n";
        let body = parse_body(md);
        assert_eq!(body.assumptions.len(), 1);
        assert!(!body.assumptions[0].has_checkbox);
        assert!(!body.assumptions[0].validated);
    }

    #[test]
    fn parse_deliverable() {
        let md = "## Deliverable\nWorking code with tests.\n";
        let body = parse_body(md);
        assert!(body.deliverable.is_some());
        assert!(body
            .deliverable
            .unwrap()
            .contains("Working code with tests"));
    }

    #[test]
    fn unknown_sections_preserved() {
        let md = "## Custom Section\nSome custom content.\n";
        let body = parse_body(md);
        assert_eq!(body.unknown_sections.len(), 1);
        assert_eq!(body.unknown_sections[0].heading, "Custom Section");
        assert!(body.unknown_sections[0]
            .content
            .contains("Some custom content"));
    }

    #[test]
    fn unknown_sections_preserve_raw_markdown() {
        let md = "## Commands\n\n- Build: `cargo build`\n- Test: `cargo test`\n\n## Style\n\n- Use **strict** mode\n- Prefer `thiserror`\n\n### Sub-heading\n\nMore content with [links](http://example.com).\n";
        let body = parse_body(md);
        assert_eq!(body.unknown_sections.len(), 2);
        // Commands section preserves list with code spans
        assert_eq!(body.unknown_sections[0].heading, "Commands");
        assert!(body.unknown_sections[0].content.contains("- Build: `cargo build`"));
        assert!(body.unknown_sections[0].content.contains("- Test: `cargo test`"));
        // Style section preserves emphasis, code, sub-headings, and links
        assert_eq!(body.unknown_sections[1].heading, "Style");
        assert!(body.unknown_sections[1].content.contains("**strict**"));
        assert!(body.unknown_sections[1].content.contains("`thiserror`"));
        assert!(body.unknown_sections[1].content.contains("### Sub-heading"));
        assert!(body.unknown_sections[1].content.contains("[links](http://example.com)"));
    }

    #[test]
    fn multiple_unknown_sections_between_known() {
        let md = "# Goal\n\n## Custom One\n\nContent one.\n\n## Constraints\n\n### Hard\n- Rule\n\n## Custom Two\n\nContent two.\n";
        let body = parse_body(md);
        assert_eq!(body.goal, Some("Goal".to_string()));
        assert_eq!(body.constraints.hard, vec!["Rule"]);
        assert_eq!(body.unknown_sections.len(), 2);
        assert_eq!(body.unknown_sections[0].heading, "Custom One");
        assert!(body.unknown_sections[0].content.contains("Content one"));
        assert_eq!(body.unknown_sections[1].heading, "Custom Two");
        assert!(body.unknown_sections[1].content.contains("Content two"));
    }
}
