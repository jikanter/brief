//! Derive Claude Code `permissions.allow` entries from a brief's `## Commands`.
//!
//! Used by `brief emit claude --install --full`. A `## Commands` section lists
//! the project's known-safe build/test/lint commands as backtick spans; brief
//! turns each into a `Bash(<command>:*)` allow entry so the agent can run the
//! project's own tooling without a permission prompt. This is a starting
//! allowlist the user can prune — it never *denies* anything.

use crate::model::Brief;

/// The `permissions.allow` entries derived from the brief's `## Commands`
/// section, in order, deduplicated. Empty when there is no such section.
pub fn command_permissions(brief: &Brief) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for section in &brief.unknown_sections {
        if !section.heading.eq_ignore_ascii_case("commands") {
            continue;
        }
        for cmd in backtick_spans(&section.content) {
            let entry = format!("Bash({cmd}:*)");
            if !out.contains(&entry) {
                out.push(entry);
            }
        }
    }
    out
}

/// Extract the contents of every `` `...` `` span in `text`, trimmed, dropping
/// empties. Unterminated trailing backticks are ignored.
fn backtick_spans(text: &str) -> Vec<String> {
    let mut spans = Vec::new();
    let mut rest = text;
    while let Some(open) = rest.find('`') {
        let after = &rest[open + 1..];
        let Some(close) = after.find('`') else {
            break;
        };
        let span = after[..close].trim();
        if !span.is_empty() {
            spans.push(span.to_string());
        }
        rest = &after[close + 1..];
    }
    spans
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;

    fn brief_with_commands(content: &str) -> Brief {
        Brief {
            frontmatter: Frontmatter::default(),
            goal: "Goal".into(),
            identity: None,
            constraints: Constraints::default(),
            sacred: vec![],
            assumptions: vec![],
            deliverable: None,
            unknown_sections: vec![UnknownSection {
                heading: "Commands".into(),
                content: content.into(),
            }],
        }
    }

    #[test]
    fn extracts_bash_permissions_from_commands() {
        let brief = brief_with_commands(
            "- Build: `cargo build`\n- Test: `cargo test`\n- Lint: `cargo clippy`",
        );
        let perms = command_permissions(&brief);
        assert_eq!(
            perms,
            vec![
                "Bash(cargo build:*)".to_string(),
                "Bash(cargo test:*)".to_string(),
                "Bash(cargo clippy:*)".to_string(),
            ]
        );
    }

    #[test]
    fn dedupes_repeated_commands() {
        let brief = brief_with_commands("`npm run build` and again `npm run build`");
        assert_eq!(command_permissions(&brief), vec!["Bash(npm run build:*)"]);
    }

    #[test]
    fn no_commands_section_yields_empty() {
        let brief = Brief {
            frontmatter: Frontmatter::default(),
            goal: "Goal".into(),
            identity: None,
            constraints: Constraints::default(),
            sacred: vec![],
            assumptions: vec![],
            deliverable: None,
            unknown_sections: vec![UnknownSection {
                heading: "Code Style".into(),
                content: "use `rustfmt`".into(),
            }],
        };
        assert!(command_permissions(&brief).is_empty());
    }

    #[test]
    fn ignores_unterminated_backtick() {
        let brief = brief_with_commands("`cargo build` then `oops");
        assert_eq!(command_permissions(&brief), vec!["Bash(cargo build:*)"]);
    }
}
