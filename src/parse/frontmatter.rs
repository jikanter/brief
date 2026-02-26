use crate::model::Frontmatter;
use anyhow::{Context, Result};

/// Extract YAML frontmatter and return (frontmatter, remaining markdown body).
///
/// Frontmatter is delimited by `---` on its own line at the start of the file.
pub fn extract_frontmatter(input: &str) -> Result<(Frontmatter, &str)> {
    let trimmed = input.trim_start();
    if !trimmed.starts_with("---") {
        return Ok((Frontmatter::default(), input));
    }

    // Skip the opening ---
    let after_opening = &trimmed[3..];
    let after_opening = after_opening.strip_prefix('\n').unwrap_or(after_opening);

    // Find closing ---
    let end_pos = after_opening
        .find("\n---")
        .context("Unclosed frontmatter: missing closing `---`")?;

    let yaml_str = &after_opening[..end_pos];
    let rest = &after_opening[end_pos + 4..]; // skip \n---
    let rest = rest.strip_prefix('\n').unwrap_or(rest);

    let frontmatter: Frontmatter =
        serde_yaml::from_str(yaml_str).context("Failed to parse YAML frontmatter")?;

    Ok((frontmatter, rest))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_frontmatter() {
        let input = "---\nstack: [Rust]\n---\n\n# Goal\n";
        let (fm, rest) = extract_frontmatter(input).unwrap();
        assert_eq!(fm.stack, vec!["Rust"]);
        assert!(fm.context.is_empty());
        assert_eq!(fm.model, None);
        assert_eq!(fm.version, "1");
        assert!(rest.contains("# Goal"));
    }

    #[test]
    fn parse_full_frontmatter() {
        let input = "---\nstack: [TypeScript 5.4, React 18]\ncontext: [./docs/arch.md]\nmodel: claude-sonnet-4-20250514\nversion: \"1\"\n---\n\n# Goal\n";
        let (fm, rest) = extract_frontmatter(input).unwrap();
        assert_eq!(fm.stack, vec!["TypeScript 5.4", "React 18"]);
        assert_eq!(fm.context, vec!["./docs/arch.md"]);
        assert_eq!(fm.model, Some("claude-sonnet-4-20250514".to_string()));
        assert_eq!(fm.version, "1");
        assert!(rest.contains("# Goal"));
    }

    #[test]
    fn missing_frontmatter_returns_defaults() {
        let input = "# Just a heading\n\nSome text.";
        let (fm, rest) = extract_frontmatter(input).unwrap();
        assert!(fm.stack.is_empty());
        assert_eq!(rest, input);
    }

    #[test]
    fn missing_stack_defaults_to_empty() {
        let input = "---\ncontext: [./file.md]\n---\n\nBody";
        let (fm, _) = extract_frontmatter(input).unwrap();
        assert!(fm.stack.is_empty());
        assert_eq!(fm.context, vec!["./file.md"]);
    }

    #[test]
    fn unclosed_frontmatter_is_error() {
        let input = "---\nstack: [Rust]\n\n# No closing delimiter";
        let result = extract_frontmatter(input);
        assert!(result.is_err());
    }
}
