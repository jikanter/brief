//! Cassette bindings: which astrophage eval-replay sets a brief's role replays.
//!
//! A binding is authored in either of two places, and both may be used at once:
//!
//! ```markdown
//! ---
//! cassettes: [rust-reviewer-v1]
//! ---
//!
//! ## Fixtures
//! - cassette: rust-reviewer-v1 — pinned eval-replay set for the rust-reviewer role
//! ```
//!
//! `## Fixtures` stays an unknown section, so every emitter that passed it
//! through before still does. This module reads the bindings out of it; the
//! only consumer is `brief emit promptfoo`. Brief never records or replays a
//! cassette: astrophage does, when the eval harness starts it.

use crate::model::Brief;

/// The H2 heading whose list items are cassette bindings.
pub const FIXTURES_HEADING: &str = "Fixtures";

const CASSETTE_PREFIX: &str = "cassette:";

/// One cassette set the brief's eval replays.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CassetteBinding {
    /// The set name: the directory under the consumer's `cassettes/`.
    pub name: String,
    /// Why the set is bound, from a `## Fixtures` item. `None` when the set is
    /// named only in frontmatter, or the item gives no reason.
    pub reason: Option<String>,
}

/// Every cassette binding in the brief: frontmatter `cassettes:` first, in
/// order, then `## Fixtures` items not already named. A set named in both
/// places is one binding, carrying the section's reason.
pub fn cassette_bindings(brief: &Brief) -> Vec<CassetteBinding> {
    let mut bindings: Vec<CassetteBinding> = brief
        .frontmatter
        .cassettes
        .iter()
        .map(|name| CassetteBinding {
            name: name.trim().to_string(),
            reason: None,
        })
        .collect();

    for item in fixture_items(brief) {
        let Some(parsed) = parse_item(&item) else {
            continue;
        };
        match bindings.iter_mut().find(|b| b.name == parsed.name) {
            Some(existing) => {
                if existing.reason.is_none() {
                    existing.reason = parsed.reason;
                }
            }
            None => bindings.push(parsed),
        }
    }

    bindings
}

/// `## Fixtures` items that do not read as `cassette: <set> — <reason>`, as
/// authored. `brief validate` warns on each.
pub fn malformed_fixture_items(brief: &Brief) -> Vec<String> {
    fixture_items(brief)
        .into_iter()
        .filter(|item| parse_item(item).is_none())
        .collect()
}

/// A set name is usable when it is a plain directory name: it starts with a
/// letter or digit and holds only letters, digits, `.`, `_` and `-`. That rules
/// out a path (`a/b`, `../x`), a hidden directory, and whitespace, any of which
/// would point astrophage somewhere other than `cassettes/<set>/`.
pub fn is_valid_set_name(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(first) if first.is_ascii_alphanumeric() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

/// The diagnostic for a set name [`is_valid_set_name`] rejects.
pub fn invalid_set_name_message(name: &str) -> String {
    format!(
        "Invalid cassette set name `{name}`: use letters, digits, `.`, `_` and `-`, \
         starting with a letter or digit"
    )
}

/// The text of each top-level list item under every `## Fixtures` heading.
fn fixture_items(brief: &Brief) -> Vec<String> {
    brief
        .unknown_sections
        .iter()
        .filter(|section| section.heading == FIXTURES_HEADING)
        .flat_map(|section| section.content.lines())
        .filter_map(|line| {
            ["- ", "* ", "+ "]
                .iter()
                .find_map(|marker| line.strip_prefix(marker))
        })
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .collect()
}

/// Parse `cassette: <set> — <reason>`. The set may be backtick-wrapped; the
/// separator is an em dash or `--`, as in a sacred entry; the reason is
/// optional. `None` when the prefix or the set name is missing.
fn parse_item(item: &str) -> Option<CassetteBinding> {
    let rest = item.strip_prefix(CASSETTE_PREFIX)?.trim_start();

    let (name, rest) = match rest.strip_prefix('`') {
        Some(quoted) => {
            let end = quoted.find('`')?;
            (&quoted[..end], &quoted[end + 1..])
        }
        None => match rest.find(char::is_whitespace) {
            Some(end) => (&rest[..end], &rest[end..]),
            None => (rest, ""),
        },
    };
    let name = name.trim();
    if name.is_empty() {
        return None;
    }

    let rest = rest.trim();
    let reason = rest
        .strip_prefix('—')
        .or_else(|| rest.strip_prefix("--"))
        .unwrap_or(rest)
        .trim();

    Some(CassetteBinding {
        name: name.to_string(),
        reason: (!reason.is_empty()).then(|| reason.to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_names() {
        for good in ["rust-reviewer-v1", "a", "set_2.1", "V1"] {
            assert!(is_valid_set_name(good), "{good}");
        }
        for bad in ["", ".hidden", "../x", "a/b", "a b", "-flag", "a\\b"] {
            assert!(!is_valid_set_name(bad), "{bad}");
        }
    }

    #[test]
    fn item_without_a_closing_backtick_is_malformed() {
        assert_eq!(parse_item("cassette: `open"), None);
    }

    #[test]
    fn item_with_an_empty_name_is_malformed() {
        assert_eq!(parse_item("cassette:"), None);
        assert_eq!(parse_item("cassette: `` — reason"), None);
    }
}
