//! Shared marker helpers for installing brief-generated content into host
//! Markdown files (CLAUDE.md, AGENTS.md, etc.).
//!
//! All install targets that wrap a generated section in `<brief:generated>`
//! tags share this implementation: locating marker pairs, replacing the first
//! pair in place, stripping empty extras, and migrating the legacy
//! `<!-- brief:start --> / <!-- brief:end -->` HTML-comment flavor (which is
//! invisible to Claude Code because Claude strips HTML comments before the
//! model sees the file).

pub const MARKER_START: &str = "<brief:generated>";
pub const MARKER_END: &str = "</brief:generated>";

const LEGACY_MARKER_START: &str = "<!-- brief:start -->";
const LEGACY_MARKER_END: &str = "<!-- brief:end -->";

/// A brief marker pair located inside a host document.
#[derive(Debug, Clone, Copy)]
pub struct MarkerPair {
    pub pair_start: usize,
    pub content_start: usize,
    pub content_end: usize,
    pub pair_end: usize,
}

/// Wrap emitted content in brief markers.
pub fn wrap_with_markers(content: &str) -> String {
    format!("{MARKER_START}\n{content}{MARKER_END}\n")
}

/// Find all brief marker pairs in the text. Recognizes both the current
/// `<brief:generated>` flavor and the legacy `<!-- brief:start -->` pair so
/// legacy installs can be migrated in place.
pub fn find_marker_pairs(text: &str) -> Vec<MarkerPair> {
    let flavors: [(&str, &str); 2] = [
        (MARKER_START, MARKER_END),
        (LEGACY_MARKER_START, LEGACY_MARKER_END),
    ];
    let mut pairs = Vec::new();
    let mut search_from = 0;

    while search_from < text.len() {
        let next = flavors
            .iter()
            .filter_map(|(start_tok, end_tok)| {
                text[search_from..]
                    .find(start_tok)
                    .map(|off| (search_from + off, *start_tok, *end_tok))
            })
            .min_by_key(|&(pos, _, _)| pos);

        let (pair_start, start_tok, end_tok) = match next {
            Some(v) => v,
            None => break,
        };

        let content_start = pair_start + start_tok.len();
        let content_end = match text[content_start..].find(end_tok) {
            Some(offset) => content_start + offset,
            None => break,
        };
        let mut pair_end = content_end + end_tok.len();
        if pair_end < text.len() && text.as_bytes()[pair_end] == b'\n' {
            pair_end += 1;
        }
        pairs.push(MarkerPair {
            pair_start,
            content_start,
            content_end,
            pair_end,
        });
        search_from = pair_end;
    }

    pairs
}

/// Where to place the brief section on a *first* install (when the host file
/// has no existing markers). On a re-install the section is always replaced in
/// place, so position is ignored once markers exist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Position {
    /// Append to the end of the file (the default, non-destructive).
    Bottom,
    /// Prepend to the very top — primacy positioning for attention dynamics.
    Top,
    /// Insert immediately after the named `## <heading>` section.
    After(String),
}

/// Like [`inject_section`], but on a first install (no existing markers) the
/// section is placed according to `position`. When markers already exist the
/// section is replaced in place and `position` is ignored (idempotent).
pub fn inject_section_at(
    existing: &str,
    wrapped_section: &str,
    position: &Position,
) -> (String, usize) {
    if !find_marker_pairs(existing).is_empty() {
        return inject_section(existing, wrapped_section);
    }
    match position {
        Position::Bottom => inject_section(existing, wrapped_section),
        Position::Top => {
            if existing.is_empty() {
                return (wrapped_section.to_string(), 0);
            }
            let mut out = String::with_capacity(existing.len() + wrapped_section.len());
            out.push_str(wrapped_section);
            if !out.ends_with('\n') {
                out.push('\n');
            }
            out.push('\n');
            out.push_str(existing);
            (out, 0)
        }
        Position::After(heading) => {
            match insert_after_heading(existing, wrapped_section, heading) {
                Some(out) => (out, 0),
                // Heading not found — fall back to a safe append.
                None => inject_section(existing, wrapped_section),
            }
        }
    }
}

/// Insert `wrapped_section` right after the `## <heading>` section (before the
/// next H2 or end of file). Returns `None` if the heading is not present.
fn insert_after_heading(existing: &str, wrapped_section: &str, heading: &str) -> Option<String> {
    let lines: Vec<&str> = existing.lines().collect();
    let target = format!("## {}", heading.trim());
    let start = lines.iter().position(|l| l.trim() == target)?;
    // Find the next H2 after the heading; the section ends just before it.
    let end = (start + 1..lines.len())
        .find(|&i| lines[i].trim_start().starts_with("## "))
        .unwrap_or(lines.len());

    let before = lines[..end].join("\n");
    let after = lines[end..].join("\n");

    let mut out = String::with_capacity(existing.len() + wrapped_section.len());
    out.push_str(&before);
    out.push('\n');
    out.push('\n');
    out.push_str(wrapped_section);
    if !after.is_empty() {
        if !out.ends_with('\n') {
            out.push('\n');
        }
        out.push('\n');
        out.push_str(&after);
    }
    if existing.ends_with('\n') && !out.ends_with('\n') {
        out.push('\n');
    }
    Some(out)
}

/// Remove every brief marker pair (current and legacy flavors) from the text,
/// leaving the user's surrounding content. Used by `--uninstall`.
pub fn remove_sections(existing: &str) -> String {
    let pairs = find_marker_pairs(existing);
    if pairs.is_empty() {
        return existing.to_string();
    }
    let mut out = String::with_capacity(existing.len());
    let mut cursor = 0;
    for pair in pairs {
        out.push_str(&existing[cursor..pair.pair_start]);
        cursor = pair.pair_end;
    }
    out.push_str(&existing[cursor..]);
    out
}

/// Replace content between markers, or append if no markers found.
/// Returns the resulting content and the number of marker pairs found.
pub fn inject_section(existing: &str, wrapped_section: &str) -> (String, usize) {
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

    for (i, pair) in pairs.iter().enumerate() {
        out.push_str(&existing[cursor..pair.pair_start]);

        if i == 0 {
            out.push_str(wrapped_section);
        } else {
            let between = &existing[pair.content_start..pair.content_end];
            if between.trim().is_empty() {
                // Empty pair: skip entirely
            } else {
                out.push_str(&existing[pair.pair_start..pair.pair_end]);
            }
        }

        cursor = pair.pair_end;
    }

    out.push_str(&existing[cursor..]);

    (out, total_pairs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inject_replaces_existing_markers() {
        let existing = "# My Project\n\nSome intro.\n\n<brief:generated>\nold content\n</brief:generated>\n\n## Other Stuff\n";
        let section = "<brief:generated>\nnew content\n</brief:generated>\n";
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
        let section = "<brief:generated>\nbriefing here\n</brief:generated>\n";
        let (result, pairs) = inject_section(existing, section);
        assert_eq!(pairs, 0);
        assert!(result.starts_with("# My Project"));
        assert!(result.contains("<brief:generated>"));
        assert!(result.contains("briefing here"));
        assert!(result.ends_with("</brief:generated>\n"));
    }

    #[test]
    fn inject_appends_to_empty() {
        let section = "<brief:generated>\ncontent\n</brief:generated>\n";
        let (result, pairs) = inject_section("", section);
        assert_eq!(pairs, 0);
        assert_eq!(result, section);
    }

    #[test]
    fn inject_preserves_content_around_markers() {
        let existing = "before\n<brief:generated>\nold\n</brief:generated>\nafter\n";
        let section = "<brief:generated>\nnew\n</brief:generated>\n";
        let (result, pairs) = inject_section(existing, section);
        assert_eq!(pairs, 1);
        assert_eq!(
            result,
            "before\n<brief:generated>\nnew\n</brief:generated>\nafter\n"
        );
    }

    #[test]
    fn wrap_with_markers_produces_valid_output() {
        let content = "# Briefing: Test\n\n**Stack:** Rust\n\n";
        let result = wrap_with_markers(content);
        assert!(result.starts_with("<brief:generated>"));
        assert!(result.contains(content));
        assert!(result.ends_with("</brief:generated>\n"));
    }

    #[test]
    fn inject_migrates_legacy_html_comment_markers() {
        let existing = "\
# My Project\n\
\n\
<!-- brief:start -->\n\
# Briefing: Old task\n\
stale content\n\
<!-- brief:end -->\n\
\n\
## Other\n";
        let section =
            "<brief:generated>\n# Briefing: New task\nfresh content\n</brief:generated>\n";
        let (result, pairs) = inject_section(existing, section);
        assert_eq!(pairs, 1);
        assert!(!result.contains("<!-- brief:start -->"));
        assert!(!result.contains("<!-- brief:end -->"));
        assert!(result.contains("<brief:generated>"));
        assert!(result.contains("</brief:generated>"));
        assert!(result.contains("fresh content"));
        assert!(!result.contains("stale content"));
        assert!(result.contains("# My Project"));
        assert!(result.contains("## Other"));
    }

    #[test]
    fn inject_multiple_pairs_uses_first_and_strips_empty() {
        let existing = "\
header\n\
<brief:generated>\n\
old content\n\
</brief:generated>\n\
middle\n\
<brief:generated>\n\
</brief:generated>\n\
footer\n";
        let section = "<brief:generated>\nnew content\n</brief:generated>\n";
        let (result, pairs) = inject_section(existing, section);
        assert_eq!(pairs, 2);
        assert!(result.contains("new content"));
        assert!(!result.contains("old content"));
        assert_eq!(result.matches("<brief:generated>").count(), 1);
        assert_eq!(result.matches("</brief:generated>").count(), 1);
        assert!(result.contains("header"));
        assert!(result.contains("middle"));
        assert!(result.contains("footer"));
    }

    #[test]
    fn inject_multiple_pairs_preserves_nonempty_extra() {
        let existing = "\
header\n\
<brief:generated>\n\
old content\n\
</brief:generated>\n\
middle\n\
<brief:generated>\n\
manual notes\n\
</brief:generated>\n\
footer\n";
        let section = "<brief:generated>\nnew content\n</brief:generated>\n";
        let (result, pairs) = inject_section(existing, section);
        assert_eq!(pairs, 2);
        assert!(result.contains("new content"));
        assert!(!result.contains("old content"));
        assert!(result.contains("manual notes"));
        assert_eq!(result.matches("<brief:generated>").count(), 2);
    }

    #[test]
    fn inject_three_pairs_strips_all_empty_extras() {
        let existing = "\
<brief:generated>\n\
old\n\
</brief:generated>\n\
between1\n\
<brief:generated>\n\
</brief:generated>\n\
between2\n\
<brief:generated>\n\
</brief:generated>\n\
end\n";
        let section = "<brief:generated>\nnew\n</brief:generated>\n";
        let (result, pairs) = inject_section(existing, section);
        assert_eq!(pairs, 3);
        assert!(result.contains("new"));
        assert!(!result.contains("old"));
        assert_eq!(result.matches("<brief:generated>").count(), 1);
        assert!(result.contains("between1"));
        assert!(result.contains("between2"));
        assert!(result.contains("end"));
    }

    #[test]
    fn find_marker_pairs_finds_all() {
        let text = "a\n<brief:generated>\nb\n</brief:generated>\nc\n<brief:generated>\n</brief:generated>\nd\n";
        let pairs = find_marker_pairs(text);
        assert_eq!(pairs.len(), 2);
    }

    #[test]
    fn find_marker_pairs_mixed_legacy_and_new() {
        let text = "\
<!-- brief:start -->\n\
legacy body\n\
<!-- brief:end -->\n\
middle\n\
<brief:generated>\n\
new body\n\
</brief:generated>\n";
        let pairs = find_marker_pairs(text);
        assert_eq!(pairs.len(), 2);
        assert_eq!(
            text[pairs[0].content_start..pairs[0].content_end].trim(),
            "legacy body"
        );
        assert_eq!(
            text[pairs[1].content_start..pairs[1].content_end].trim(),
            "new body"
        );
    }

    #[test]
    fn find_marker_pairs_handles_no_markers() {
        let pairs = find_marker_pairs("just some text\n");
        assert!(pairs.is_empty());
    }

    #[test]
    fn inject_at_top_prepends_above_existing() {
        let existing = "# My Project\n\nIntro.\n";
        let section = "<brief:generated>\nbrief\n</brief:generated>\n";
        let (out, pairs) = inject_section_at(existing, section, &Position::Top);
        assert_eq!(pairs, 0);
        assert!(out.starts_with("<brief:generated>"));
        assert!(out.contains("# My Project"));
        assert!(out.find("brief").unwrap() < out.find("# My Project").unwrap());
    }

    #[test]
    fn inject_at_bottom_appends() {
        let existing = "# My Project\n";
        let section = "<brief:generated>\nbrief\n</brief:generated>\n";
        let (out, _) = inject_section_at(existing, section, &Position::Bottom);
        assert!(out.starts_with("# My Project"));
        assert!(out.trim_end().ends_with("</brief:generated>"));
    }

    #[test]
    fn inject_after_heading_places_within_section() {
        let existing = "# Title\n\n## Setup\n\nsetup body\n\n## Other\n\nother body\n";
        let section = "<brief:generated>\nbrief\n</brief:generated>\n";
        let (out, _) = inject_section_at(existing, section, &Position::After("Setup".into()));
        // brief comes after Setup body but before Other.
        assert!(out.find("setup body").unwrap() < out.find("brief").unwrap());
        assert!(out.find("brief").unwrap() < out.find("## Other").unwrap());
    }

    #[test]
    fn inject_after_missing_heading_falls_back_to_append() {
        let existing = "# Title\n\n## Setup\n\nbody\n";
        let section = "<brief:generated>\nbrief\n</brief:generated>\n";
        let (out, _) = inject_section_at(existing, section, &Position::After("Nope".into()));
        assert!(out.find("body").unwrap() < out.find("brief").unwrap());
    }

    #[test]
    fn inject_at_position_replaces_in_place_when_markers_exist() {
        // Position is ignored once markers exist — re-install stays idempotent.
        let existing = "head\n<brief:generated>\nold\n</brief:generated>\ntail\n";
        let section = "<brief:generated>\nnew\n</brief:generated>\n";
        let (out, pairs) = inject_section_at(existing, section, &Position::Top);
        assert_eq!(pairs, 1);
        assert_eq!(
            out,
            "head\n<brief:generated>\nnew\n</brief:generated>\ntail\n"
        );
    }

    #[test]
    fn remove_sections_strips_brief_region() {
        let existing = "# Project\n\n<brief:generated>\ngen\n</brief:generated>\n\n## Notes\n";
        let out = remove_sections(existing);
        assert!(!out.contains("<brief:generated>"));
        assert!(!out.contains("gen"));
        assert!(out.contains("# Project"));
        assert!(out.contains("## Notes"));
    }

    #[test]
    fn remove_sections_strips_legacy_markers() {
        let existing = "x\n<!-- brief:start -->\nold\n<!-- brief:end -->\ny\n";
        let out = remove_sections(existing);
        assert!(!out.contains("brief:start"));
        assert!(!out.contains("old"));
        assert!(out.contains('x'));
        assert!(out.contains('y'));
    }

    #[test]
    fn remove_sections_noop_without_markers() {
        let existing = "just content\n";
        assert_eq!(remove_sections(existing), existing);
    }

    #[test]
    fn find_marker_pairs_handles_unmatched_start() {
        let text = "<brief:generated>\nno end marker\n";
        let pairs = find_marker_pairs(text);
        assert!(pairs.is_empty());
    }

    #[test]
    fn find_marker_pairs_handles_unmatched_legacy_start() {
        let text = "<!-- brief:start -->\nno end marker\n";
        let pairs = find_marker_pairs(text);
        assert!(pairs.is_empty());
    }
}
