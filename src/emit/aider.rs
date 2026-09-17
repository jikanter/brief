use std::path::Path;

use serde_json::{Map, Value};

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
pub fn merge_aider_conf(existing: Option<&str>, model: Option<&str>) -> anyhow::Result<String> {
    let mut map: Map<String, Value> = match existing {
        Some(s) if !s.trim().is_empty() => serde_saphyr::from_str(s)?,
        _ => Map::new(),
    };

    ensure_read_includes_conventions(&mut map);

    if let Some(m) = model
        && !map.contains_key("model")
    {
        map.insert("model".to_string(), Value::from(m));
    }

    Ok(serde_saphyr::to_string(&Value::Object(map))?)
}

fn ensure_read_includes_conventions(map: &mut Map<String, Value>) {
    let conv = Value::from(CONVENTIONS_FILE);

    // Compute the replacement (if any) under an immutable borrow, then insert.
    let new_val = match map.get("read") {
        None => Some(conv),
        Some(Value::String(s)) if s == CONVENTIONS_FILE => None,
        Some(Value::String(s)) => Some(Value::Array(vec![Value::String(s.clone()), conv])),
        Some(Value::Array(seq)) => {
            if seq.iter().any(|v| v.as_str() == Some(CONVENTIONS_FILE)) {
                None
            } else {
                let mut next = seq.clone();
                next.push(conv);
                Some(Value::Array(next))
            }
        }
        // Any other (unexpected) type: replace with the scalar conventions path.
        Some(_) => Some(conv),
    };

    if let Some(v) = new_val {
        map.insert("read".to_string(), v);
    }
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

    // 2. .aider.conf.yml — idempotent YAML merge so CONVENTIONS.md auto-loads.
    let conf = base_dir.join(".aider.conf.yml");
    let existing = std::fs::read_to_string(&conf).ok();
    let merged = merge_aider_conf(existing.as_deref(), brief.frontmatter.model.as_deref())
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
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

    #[test]
    fn merge_creates_read_and_model_when_empty() {
        let yaml = merge_aider_conf(None, Some("claude-opus-4-8")).unwrap();
        let v: Value = serde_saphyr::from_str(&yaml).unwrap();
        assert_eq!(v["read"].as_str(), Some("CONVENTIONS.md"));
        assert_eq!(v["model"].as_str(), Some("claude-opus-4-8"));
    }

    /// Known regression from the `serde_yaml` -> `serde-saphyr` swap, pinned
    /// so it cannot drift further unnoticed.
    ///
    /// `serde_yaml`'s mapping preserved the author's key order. `serde_json`'s
    /// does not unless its `preserve_order` feature is on, and that feature
    /// also reorders the schema `schemars` derives, which would break the
    /// byte-identical guarantee on `docs/schema/brief-frontmatter-v1.schema.json`.
    /// The schema guarantee wins, so a merged `.aider.conf.yml` comes back
    /// alphabetized.
    ///
    /// This only matters because the merge deserializes and re-emits a file
    /// brief does not own -- the same reason it already drops comments (see
    /// `docs/bugs.md`). Replacing the round-trip with a line splice fixes the
    /// ordering and the comments together; until then, this test is the record.
    #[test]
    fn merge_alphabetizes_keys_pending_the_line_splice_fix() {
        let existing = "zeta: 1\nalpha: 2\n";
        let yaml = merge_aider_conf(Some(existing), None).unwrap();
        let keys: Vec<&str> = yaml
            .lines()
            .filter(|l| !l.starts_with(['-', ' ']) && l.contains(':'))
            .map(|l| l.split(':').next().unwrap())
            .collect();
        assert_eq!(keys, vec!["alpha", "read", "zeta"]);
    }

    #[test]
    fn merge_does_not_overwrite_existing_model() {
        let existing = "model: gpt-4o\n";
        let yaml = merge_aider_conf(Some(existing), Some("claude-opus-4-8")).unwrap();
        let v: Value = serde_saphyr::from_str(&yaml).unwrap();
        assert_eq!(v["model"].as_str(), Some("gpt-4o"));
        assert_eq!(v["read"].as_str(), Some("CONVENTIONS.md"));
    }

    #[test]
    fn merge_omits_model_when_brief_has_none() {
        let yaml = merge_aider_conf(None, None).unwrap();
        let v: Value = serde_saphyr::from_str(&yaml).unwrap();
        assert!(v.get("model").is_none());
        assert_eq!(v["read"].as_str(), Some("CONVENTIONS.md"));
    }

    #[test]
    fn merge_promotes_existing_scalar_read_to_list() {
        let existing = "read: OTHER.md\n";
        let yaml = merge_aider_conf(Some(existing), None).unwrap();
        let v: Value = serde_saphyr::from_str(&yaml).unwrap();
        let list = v["read"].as_array().expect("read should be a list");
        let items: Vec<&str> = list.iter().filter_map(|x| x.as_str()).collect();
        assert!(items.contains(&"OTHER.md"));
        assert!(items.contains(&"CONVENTIONS.md"));
    }

    #[test]
    fn merge_extends_existing_read_list() {
        let existing = "read:\n  - OTHER.md\n";
        let yaml = merge_aider_conf(Some(existing), None).unwrap();
        let v: Value = serde_saphyr::from_str(&yaml).unwrap();
        let items: Vec<&str> = v["read"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|x| x.as_str())
            .collect();
        assert!(items.contains(&"OTHER.md"));
        assert!(items.contains(&"CONVENTIONS.md"));
    }

    #[test]
    fn merge_is_idempotent() {
        let once = merge_aider_conf(None, Some("claude-opus-4-8")).unwrap();
        let twice = merge_aider_conf(Some(&once), Some("claude-opus-4-8")).unwrap();
        assert_eq!(once, twice);
    }

    #[test]
    fn merge_idempotent_when_read_already_present() {
        let existing = "read: CONVENTIONS.md\n";
        let yaml = merge_aider_conf(Some(existing), None).unwrap();
        let v: Value = serde_saphyr::from_str(&yaml).unwrap();
        // Still a scalar, not promoted to a one-element list.
        assert_eq!(v["read"].as_str(), Some("CONVENTIONS.md"));
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
