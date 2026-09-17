use std::path::Path;

use crate::emit::markers::{inject_section, wrap_with_markers};
use crate::framing::with_scope;
use crate::model::Brief;

/// The conventions file Aider auto-loads via its config `read:` key.
const CONVENTIONS_FILE: &str = "CONVENTIONS.md";

/// Emit an Aider `CONVENTIONS.md` document from a Brief.
///
/// Aider's idiomatic register is conversational — bulleted natural-language
/// preferences, not imperative directives. Hard constraints render as plain
/// statements, soft constraints as "Prefer ..." preferences, and ask-first
/// items as "Ask before ..." lines. Sacred regions render as a "Files not to
/// modify" list. See docs/design/backends/aider/README.md.
///
/// `CONVENTIONS.md` alone is a half-integration: Aider only loads it when the
/// session config (`.aider.conf.yml`) carries `read: CONVENTIONS.md`. The full
/// integration is wired up by [`install_aider`], which also touches that config.
pub fn emit_aider(brief: &Brief) -> String {
    let mut out = String::new();

    out.push_str(&format!("# {}\n\n", brief.goal));

    if !brief.frontmatter.stack.is_empty() {
        out.push_str(&format!(
            "Built with {}.\n\n",
            brief.frontmatter.stack.join(", ")
        ));
    }

    if !brief.frontmatter.context.is_empty() {
        out.push_str("## Reference\n\nKeep these files in mind:\n\n");
        for ctx in &brief.frontmatter.context {
            out.push_str(&format!("- `{ctx}`\n"));
        }
        out.push('\n');
    }

    if !brief.constraints.hard.is_empty() {
        out.push_str("## Guidelines\n\n");
        for c in &brief.constraints.hard {
            out.push_str(&format!("- {}\n", with_scope(c, c)));
        }
        out.push('\n');
    }

    if !brief.constraints.soft.is_empty() {
        out.push_str("## Preferences\n\n");
        for c in &brief.constraints.soft {
            out.push_str(&format!("- {}\n", with_scope(&format!("Prefer: {c}"), c)));
        }
        out.push('\n');
    }

    if !brief.constraints.ask_first.is_empty() {
        out.push_str("## Ask first\n\n");
        for c in &brief.constraints.ask_first {
            out.push_str(&format!(
                "- {}\n",
                with_scope(&format!("Ask before: {c}"), c)
            ));
        }
        out.push('\n');
    }

    if !brief.sacred.is_empty() {
        out.push_str("## Files not to modify\n\n");
        for entry in &brief.sacred {
            out.push_str(&format!("- `{}` — {}\n", entry.path, entry.reason));
        }
        out.push('\n');
    }

    let unvalidated: Vec<_> = brief.assumptions.iter().filter(|a| !a.validated).collect();
    if !unvalidated.is_empty() {
        out.push_str("## Open questions\n\n");
        for a in &unvalidated {
            out.push_str(&format!("- {}\n", a.text));
        }
        out.push('\n');
    }

    if let Some(ref deliverable) = brief.deliverable {
        out.push_str("## Done when\n\n");
        out.push_str(deliverable.trim_end_matches('\n'));
        out.push('\n');
    }

    for section in &brief.unknown_sections {
        out.push_str(&format!(
            "\n## {}\n\n{}\n",
            section.heading, section.content
        ));
    }

    out
}

/// Idempotently merge the Aider session config so it auto-loads `CONVENTIONS.md`.
///
/// Returns the new `.aider.conf.yml` text. Rules:
/// - `read:` is ensured to include `CONVENTIONS.md`. An existing scalar `read`
///   for a different file is promoted to a list containing both; an existing
///   list gains the entry only if absent.
/// - `model:` is written only when the brief declares one *and* the config does
///   not already set it — a user's chosen model is never overwritten.
///
/// Re-running on its own output is a no-op (both keys already satisfied).
///
/// The file belongs to the user, so this is a **line splice**, not a
/// deserialize-and-re-emit: every byte outside the `read` entry and an added
/// `model` line comes back unchanged, comments and key order included. The
/// round-trip this replaced silently deleted both (`docs/bugs.md`). A shape the
/// splicer cannot edit safely is an error, never a fallback to re-emitting.
pub fn merge_aider_conf(existing: Option<&str>, model: Option<&str>) -> anyhow::Result<String> {
    let source = existing.unwrap_or("");

    // Nothing to preserve: write the keys in the order brief adds them.
    if source.trim().is_empty() {
        let mut out = format!("read: {CONVENTIONS_FILE}\n");
        if let Some(m) = model {
            out.push_str(&format!("model: {m}\n"));
        }
        return Ok(out);
    }

    let mut doc = YamlLines::parse(source)?;
    doc.ensure_read_includes_conventions()?;
    if let Some(m) = model
        && !doc.has_top_level_key("model")
    {
        doc.append_line(&format!("model: {m}"));
    }
    Ok(doc.render())
}

/// The user's config as a line buffer, with just enough YAML awareness to find
/// the top-level `read` key and refuse the shapes a line splice cannot handle.
struct YamlLines {
    /// Lines without their `\n`. A trailing `\r` stays in the content so a CRLF
    /// file round-trips unchanged.
    lines: Vec<String>,
    /// True when the source ended with a newline; an appended line must not
    /// invent one.
    trailing_newline: bool,
    /// The line ending for lines this code adds.
    eol_suffix: &'static str,
}

impl YamlLines {
    fn parse(source: &str) -> anyhow::Result<Self> {
        let trailing_newline = source.ends_with('\n');
        let mut lines: Vec<String> = source.split('\n').map(String::from).collect();
        if trailing_newline {
            lines.pop();
        }
        let doc = YamlLines {
            eol_suffix: if source.contains("\r\n") { "\r" } else { "" },
            lines,
            trailing_newline,
        };
        doc.reject_unsupported()?;
        Ok(doc)
    }

    fn render(&self) -> String {
        let mut out = self.lines.join("\n");
        if self.trailing_newline {
            out.push('\n');
        }
        out
    }

    /// The line's content with any CR stripped, so matching is the same on both
    /// line-ending conventions.
    fn content(&self, i: usize) -> &str {
        self.lines[i].strip_suffix('\r').unwrap_or(&self.lines[i])
    }

    fn push(&mut self, at: usize, text: &str) {
        self.lines.insert(at, format!("{text}{}", self.eol_suffix));
    }

    fn append_line(&mut self, text: &str) {
        let at = self.lines.len();
        self.push(at, text);
    }

    /// True when `key` appears at column zero. A `model` inside a comment or
    /// nested under another key is not the top-level key.
    fn has_top_level_key(&self, key: &str) -> bool {
        (0..self.lines.len()).any(|i| top_level_key(self.content(i)) == Some(key))
    }

    /// Shapes the splicer will not touch. Each one is a case where editing
    /// lines could change what the file means, so brief refuses rather than
    /// guessing — and refusing means the caller writes nothing.
    fn reject_unsupported(&self) -> anyhow::Result<()> {
        for i in 0..self.lines.len() {
            let line = self.content(i);
            if line
                .chars()
                .take_while(|c| c.is_whitespace())
                .any(|c| c == '\t')
            {
                anyhow::bail!(
                    "line {} indents with a tab; brief edits this file line by line and will not \
                     re-indent it",
                    i + 1
                );
            }
        }

        let markers: Vec<usize> = (0..self.lines.len())
            .filter(|&i| {
                let line = self.content(i);
                line.trim_end() == "---" || line.starts_with("--- ") || line.trim_end() == "..."
            })
            .collect();
        if markers.len() > 1 || markers.first().is_some_and(|&i| i != 0) {
            anyhow::bail!(
                "contains more than one YAML document; brief will not choose between them"
            );
        }

        if self.find_top_level_read().is_none()
            && (0..self.lines.len()).any(|i| {
                let line = self.content(i);
                line.starts_with(char::is_whitespace)
                    && top_level_key(line.trim_start()) == Some("read")
            })
        {
            anyhow::bail!("has a `read` key that is not at the top level; brief will not edit it");
        }

        Ok(())
    }

    fn find_top_level_read(&self) -> Option<usize> {
        (0..self.lines.len()).find(|&i| top_level_key(self.content(i)) == Some("read"))
    }

    fn ensure_read_includes_conventions(&mut self) -> anyhow::Result<()> {
        let Some(idx) = self.find_top_level_read() else {
            self.append_line(&format!("read: {CONVENTIONS_FILE}"));
            return Ok(());
        };

        let line = self.content(idx).to_string();
        let colon = line.find(':').expect("a key line has a colon");
        let key = line[..=colon].to_string();
        let (value, comment) = split_off_comment(&line[colon + 1..]);
        let value = value.trim();

        reject_anchor_or_alias(value)?;

        if value.is_empty() {
            return self.extend_block_list(idx);
        }
        if value.starts_with('[') {
            return self.extend_flow_list(idx, colon + 1);
        }

        // A scalar: satisfied, or promoted to a block list holding both. An
        // inline comment belongs to the value the user wrote, so it travels
        // with that item.
        if unquote(value) == CONVENTIONS_FILE {
            return Ok(());
        }
        self.lines.remove(idx);
        self.push(idx, &key);
        self.push(idx + 1, &format!("  - {value}{comment}"));
        self.push(idx + 2, &format!("  - {CONVENTIONS_FILE}"));
        Ok(())
    }

    /// `read:` with the entries on following lines, or with no value at all.
    fn extend_block_list(&mut self, idx: usize) -> anyhow::Result<()> {
        let mut last_item: Option<(usize, String)> = None;
        for i in idx + 1..self.lines.len() {
            let line = self.content(i);
            if line.trim().is_empty() || line.trim_start().starts_with('#') {
                continue;
            }
            let indent: String = line.chars().take_while(|c| c.is_whitespace()).collect();
            if indent.is_empty() {
                break;
            }
            let Some(item) = line.trim_start().strip_prefix('-') else {
                break;
            };
            let item = item.trim();
            reject_anchor_or_alias(item)?;
            if unquote(item) == CONVENTIONS_FILE {
                return Ok(());
            }
            last_item = Some((i, indent));
        }

        match last_item {
            Some((i, indent)) => self.push(i + 1, &format!("{indent}- {CONVENTIONS_FILE}")),
            // `read:` with nothing under it is null; give it a list.
            None => self.push(idx + 1, &format!("  - {CONVENTIONS_FILE}")),
        }
        Ok(())
    }

    /// `read: [a, b]`, possibly spanning lines. The entry goes in before the
    /// closing bracket so the rest of the line is untouched.
    fn extend_flow_list(&mut self, idx: usize, value_start: usize) -> anyhow::Result<()> {
        let mut inner = String::new();
        let mut close: Option<(usize, usize)> = None;
        for i in idx..self.lines.len() {
            let line = self.content(i);
            let from = if i == idx {
                line[value_start..].find('[').map(|p| value_start + p + 1)
            } else {
                Some(0)
            };
            let Some(from) = from else { continue };
            if let Some(rel) = line[from..].find(']') {
                inner.push_str(&line[from..from + rel]);
                close = Some((i, from + rel));
                break;
            }
            inner.push_str(&line[from..]);
        }
        let Some((close_line, close_col)) = close else {
            anyhow::bail!("has an unterminated `read: [` list; brief will not edit it");
        };

        for item in inner.split(',') {
            let item = item.trim();
            if item.is_empty() {
                continue;
            }
            reject_anchor_or_alias(item)?;
            if unquote(item) == CONVENTIONS_FILE {
                return Ok(());
            }
        }

        let separator = if inner.trim().is_empty() { "" } else { ", " };
        let line = &self.lines[close_line];
        self.lines[close_line] = format!(
            "{}{separator}{CONVENTIONS_FILE}{}",
            &line[..close_col],
            &line[close_col..]
        );
        Ok(())
    }
}

/// The key of a `key: value` line written at column zero, if that is what this
/// line is. A comment, an indented line, or a list item is not.
fn top_level_key(line: &str) -> Option<&str> {
    if line.starts_with(char::is_whitespace) || line.starts_with('#') || line.starts_with('-') {
        return None;
    }
    let colon = line.find(':')?;
    let key = line[..colon].trim_end();
    (!key.is_empty() && !key.contains(['#', '"', '\''])).then_some(key)
}

/// An anchor defines a node the rest of the file may alias, and an alias stands
/// in for one. Either way the text on this line is not the whole story, so a
/// line splice cannot reason about it.
fn reject_anchor_or_alias(value: &str) -> anyhow::Result<()> {
    if value.starts_with('&') {
        anyhow::bail!("defines a YAML anchor on `read`; brief will not edit it");
    }
    if value.starts_with('*') {
        anyhow::bail!("uses a YAML alias for `read`; brief will not edit it");
    }
    Ok(())
}

/// Split a value from a trailing `# comment`, leaving a `#` inside quotes
/// alone. The comment is returned with its leading whitespace so it can be
/// re-attached byte for byte.
fn split_off_comment(rest: &str) -> (&str, &str) {
    let mut quote: Option<char> = None;
    for (i, c) in rest.char_indices() {
        match (quote, c) {
            (None, '"') | (None, '\'') => quote = Some(c),
            (Some(q), c) if c == q => quote = None,
            (None, '#') => {
                let cut = rest[..i].trim_end().len();
                return (&rest[..cut], &rest[cut..]);
            }
            _ => {}
        }
    }
    (rest, "")
}

fn unquote(s: &str) -> &str {
    for q in ['"', '\''] {
        if let Some(inner) = s.strip_prefix(q).and_then(|r| r.strip_suffix(q)) {
            return inner;
        }
    }
    s
}

/// Install the full Aider integration: `CONVENTIONS.md` + `.aider.conf.yml`.
///
/// Returns the files written, in order. `CONVENTIONS.md` is freeform and
/// hand-editable, so brief injects its section between `<brief:generated>`
/// markers (replace-in-place / append, migrating the legacy flavor).
/// `.aider.conf.yml` is merged idempotently via [`merge_aider_conf`] so the
/// conventions auto-load each session.
pub fn install_aider(
    brief: &Brief,
    base_dir: &Path,
) -> Result<Vec<std::path::PathBuf>, std::io::Error> {
    let mut written = Vec::new();

    // Decide the config edit before anything is written: a shape the splicer
    // refuses must leave every file as it found it.
    let conf = base_dir.join(".aider.conf.yml");
    let existing = std::fs::read_to_string(&conf).ok();
    let merged = merge_aider_conf(existing.as_deref(), brief.frontmatter.model.as_deref())
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("{} {e}", conf.display()),
            )
        })?;

    // 1. CONVENTIONS.md — marker injection.
    let conventions = base_dir.join(CONVENTIONS_FILE);
    let wrapped = wrap_with_markers(&emit_aider(brief));
    let output = if conventions.exists() {
        let existing = std::fs::read_to_string(&conventions)?;
        let (result, pairs_found) = inject_section(&existing, &wrapped);
        if pairs_found > 1 {
            eprintln!(
                "warning: found {pairs_found} brief marker pairs; using the first, stripping remaining empty pairs"
            );
        }
        result
    } else {
        wrapped
    };
    std::fs::write(&conventions, &output)?;
    written.push(conventions);

    // 2. .aider.conf.yml — the line splice decided above.
    std::fs::write(&conf, merged)?;
    written.push(conf);

    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;

    fn full_brief() -> Brief {
        Brief {
            frontmatter: Frontmatter {
                stack: vec!["Rust".into()],
                context: vec!["./docs/arch.md".into()],
                model: Some("claude-opus-4-8".into()),
                ..Default::default()
            },
            goal: "Add notifications".into(),
            identity: None,
            constraints: Constraints {
                hard: vec!["Use Result<T, AppError> for error handling".into()],
                soft: vec!["small focused commits".into()],
                ask_first: vec!["Changes to the schema".into()],
            },
            sacred: vec![SacredEntry {
                path: "src/auth/**".into(),
                reason: "Audited".into(),
                well_formed: true,
            }],
            assumptions: vec![
                Assumption {
                    text: "Gateway scales".into(),
                    validated: false,
                    has_checkbox: true,
                },
                Assumption {
                    text: "Already known".into(),
                    validated: true,
                    has_checkbox: true,
                },
            ],
            deliverable: Some("Working system".into()),
            unknown_sections: vec![UnknownSection {
                heading: "Commands".into(),
                content: "- Build: `cargo build`".into(),
            }],
        }
    }

    // -- emit register --

    #[test]
    fn emit_uses_conversational_register() {
        let output = emit_aider(&full_brief());
        assert!(output.starts_with("# Add notifications"));
        // Hard constraints are plain statements, no MUST/NEVER/IMPORTANT.
        assert!(output.contains("## Guidelines"));
        assert!(output.contains("- Use Result<T, AppError> for error handling"));
        assert!(!output.contains("**IMPORTANT:**"));
        assert!(!output.contains("MUST:"));
        assert!(!output.contains("NEVER:"));
    }

    #[test]
    fn emit_renders_prefer_and_ask_before() {
        let output = emit_aider(&full_brief());
        assert!(output.contains("## Preferences"));
        assert!(output.contains("- Prefer: small focused commits"));
        assert!(output.contains("## Ask first"));
        assert!(output.contains("- Ask before: Changes to the schema"));
    }

    #[test]
    fn emit_renders_files_not_to_modify() {
        let output = emit_aider(&full_brief());
        assert!(output.contains("## Files not to modify"));
        assert!(output.contains("`src/auth/**` — Audited"));
    }

    #[test]
    fn emit_renders_only_unvalidated_assumptions() {
        let output = emit_aider(&full_brief());
        assert!(output.contains("## Open questions"));
        assert!(output.contains("Gateway scales"));
        assert!(!output.contains("Already known"));
    }

    // -- conf merge --
    //
    // The splice is pinned byte for byte in tests/aider_conf_splice_tests.rs;
    // these cover the two keys' own rules.

    #[test]
    fn merge_creates_read_and_model_when_empty() {
        assert_eq!(
            merge_aider_conf(None, Some("claude-opus-4-8")).unwrap(),
            "read: CONVENTIONS.md\nmodel: claude-opus-4-8\n"
        );
    }

    #[test]
    fn merge_does_not_overwrite_existing_model() {
        let yaml = merge_aider_conf(Some("model: gpt-4o\n"), Some("claude-opus-4-8")).unwrap();
        assert_eq!(yaml, "model: gpt-4o\nread: CONVENTIONS.md\n");
    }

    #[test]
    fn merge_omits_model_when_brief_has_none() {
        assert_eq!(
            merge_aider_conf(None, None).unwrap(),
            "read: CONVENTIONS.md\n"
        );
    }

    #[test]
    fn merge_promotes_existing_scalar_read_to_list() {
        assert_eq!(
            merge_aider_conf(Some("read: OTHER.md\n"), None).unwrap(),
            "read:\n  - OTHER.md\n  - CONVENTIONS.md\n"
        );
    }

    #[test]
    fn merge_extends_existing_read_list() {
        assert_eq!(
            merge_aider_conf(Some("read:\n  - OTHER.md\n"), None).unwrap(),
            "read:\n  - OTHER.md\n  - CONVENTIONS.md\n"
        );
    }

    #[test]
    fn merge_is_idempotent() {
        let once = merge_aider_conf(None, Some("claude-opus-4-8")).unwrap();
        let twice = merge_aider_conf(Some(&once), Some("claude-opus-4-8")).unwrap();
        assert_eq!(once, twice);
    }

    /// Still a scalar, not promoted to a one-element list.
    #[test]
    fn merge_idempotent_when_read_already_present() {
        assert_eq!(
            merge_aider_conf(Some("read: CONVENTIONS.md\n"), None).unwrap(),
            "read: CONVENTIONS.md\n"
        );
    }

    /// The user's key order survives, which the `serde_yaml` -> `serde-saphyr`
    /// swap had briefly cost: `serde_json::Map` alphabetizes without its
    /// `preserve_order` feature, and that feature also reorders the schema
    /// `schemars` derives. Splicing lines sidesteps the choice entirely.
    #[test]
    fn merge_preserves_the_authors_key_order() {
        let yaml = merge_aider_conf(Some("zeta: 1\nalpha: 2\n"), None).unwrap();
        assert_eq!(yaml, "zeta: 1\nalpha: 2\nread: CONVENTIONS.md\n");
    }

    // -- install --

    #[test]
    fn install_writes_both_files() {
        let dir = tempfile::tempdir().unwrap();
        let written = install_aider(&full_brief(), dir.path()).unwrap();
        assert_eq!(written.len(), 2);
        let conventions = dir.path().join("CONVENTIONS.md");
        let conf = dir.path().join(".aider.conf.yml");
        assert!(conventions.exists());
        assert!(conf.exists());

        let conv_content = std::fs::read_to_string(&conventions).unwrap();
        assert!(conv_content.starts_with("<brief:generated>"));
        assert!(conv_content.contains("# Add notifications"));

        let conf_content = std::fs::read_to_string(&conf).unwrap();
        assert!(conf_content.contains("CONVENTIONS.md"));
        assert!(conf_content.contains("claude-opus-4-8"));
    }

    #[test]
    fn install_preserves_user_conventions_content() {
        let dir = tempfile::tempdir().unwrap();
        let conventions = dir.path().join("CONVENTIONS.md");
        std::fs::write(
            &conventions,
            "# Team conventions\n\nHand-written stuff.\n\n<brief:generated>\nstale\n</brief:generated>\n",
        )
        .unwrap();

        install_aider(&full_brief(), dir.path()).unwrap();

        let content = std::fs::read_to_string(&conventions).unwrap();
        assert!(content.contains("# Team conventions"));
        assert!(content.contains("Hand-written stuff."));
        assert!(content.contains("# Add notifications"));
        assert!(!content.contains("stale"));
        assert_eq!(content.matches("<brief:generated>").count(), 1);
    }

    #[test]
    fn install_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        install_aider(&full_brief(), dir.path()).unwrap();
        let conv1 = std::fs::read_to_string(dir.path().join("CONVENTIONS.md")).unwrap();
        let conf1 = std::fs::read_to_string(dir.path().join(".aider.conf.yml")).unwrap();

        install_aider(&full_brief(), dir.path()).unwrap();
        let conv2 = std::fs::read_to_string(dir.path().join("CONVENTIONS.md")).unwrap();
        let conf2 = std::fs::read_to_string(dir.path().join(".aider.conf.yml")).unwrap();

        assert_eq!(conv1, conv2);
        assert_eq!(conf1, conf2);
    }
}
