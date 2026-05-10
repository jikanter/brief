use std::path::Path;

use crate::model::Brief;

/// Emit a Cursor `.mdc` rule from a Brief.
///
/// Cursor reads rules from `.cursor/rules/*.mdc`. Each file has its own YAML
/// frontmatter (`description`, `globs`, `alwaysApply`) which differs from
/// brief's frontmatter and must be constructed from scratch.
///
/// This is the trivial-base-case mapping: a single bundled rule with
/// `alwaysApply: true`. Brief has no native concept of glob-scoped constraints,
/// so we cannot meaningfully populate Cursor's `globs` field today. When the
/// format grows scoped constraints, this emitter is the first place that
/// expressivity unlocks (see docs/design/backends/cursor/README.md).
pub fn emit_cursor(brief: &Brief) -> String {
    let mut out = String::new();

    // Frontmatter — Cursor's schema, not brief's.
    out.push_str("---\n");
    out.push_str(&format!("description: {}\n", yaml_scalar(&brief.goal)));
    out.push_str("alwaysApply: true\n");
    out.push_str("---\n\n");

    // Goal as H1
    out.push_str(&format!("# {}\n\n", brief.goal));

    if !brief.frontmatter.stack.is_empty() {
        out.push_str(&format!(
            "**Stack:** {}\n\n",
            brief.frontmatter.stack.join(", ")
        ));
    }

    if !brief.frontmatter.context.is_empty() {
        out.push_str("## Context\n\n");
        for ctx in &brief.frontmatter.context {
            out.push_str(&format!("- `{ctx}`\n"));
        }
        out.push('\n');
    }

    // Hard — descriptive register, no IMPORTANT prefix
    if !brief.constraints.hard.is_empty() {
        out.push_str("## Required\n\n");
        for c in &brief.constraints.hard {
            out.push_str(&format!("- {c}\n"));
        }
        out.push('\n');
    }

    if !brief.constraints.soft.is_empty() {
        out.push_str("## Preferred\n\n");
        for c in &brief.constraints.soft {
            out.push_str(&format!("- {c}\n"));
        }
        out.push('\n');
    }

    if !brief.constraints.ask_first.is_empty() {
        out.push_str("## Ask First\n\n");
        for c in &brief.constraints.ask_first {
            out.push_str(&format!("- {c}\n"));
        }
        out.push('\n');
    }

    if !brief.sacred.is_empty() {
        out.push_str("## Protected Files\n\n");
        for entry in &brief.sacred {
            out.push_str(&format!("- `{}` — {}\n", entry.path, entry.reason));
        }
        out.push('\n');
    }

    let unvalidated: Vec<_> = brief.assumptions.iter().filter(|a| !a.validated).collect();
    if !unvalidated.is_empty() {
        out.push_str("## Verify\n\n");
        for a in &unvalidated {
            out.push_str(&format!("- {}\n", a.text));
        }
        out.push('\n');
    }

    if let Some(ref deliverable) = brief.deliverable {
        out.push_str("## Deliverable\n\n");
        out.push_str(deliverable);
        if !deliverable.ends_with('\n') {
            out.push('\n');
        }
    }

    for section in &brief.unknown_sections {
        out.push_str(&format!(
            "\n## {}\n\n{}\n",
            section.heading, section.content
        ));
    }

    out
}

/// Quote a YAML scalar if it contains characters that would confuse the parser.
fn yaml_scalar(s: &str) -> String {
    let needs_quoting = s.chars().any(|c| {
        matches!(
            c,
            ':' | '#'
                | '['
                | ']'
                | '{'
                | '}'
                | ','
                | '&'
                | '*'
                | '!'
                | '|'
                | '>'
                | '\''
                | '"'
                | '%'
                | '@'
                | '`'
                | '\n'
        )
    });
    if needs_quoting {
        // Double-quoted form: escape backslashes and double quotes.
        let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
        format!("\"{escaped}\"")
    } else {
        s.to_string()
    }
}

/// Install a Cursor rule into `<base>/.cursor/rules/brief.mdc`.
///
/// Brief owns this file end-to-end (unlike CLAUDE.md/AGENTS.md which mix with
/// hand-written content), so each install overwrites the file outright — no
/// `<brief:generated>` markers, no marker migration. The directory is created
/// if it does not already exist.
pub fn install_cursor(
    brief: &Brief,
    base_dir: &Path,
) -> Result<std::path::PathBuf, std::io::Error> {
    let rules_dir = base_dir.join(".cursor").join("rules");
    std::fs::create_dir_all(&rules_dir)?;
    let target = rules_dir.join("brief.mdc");
    std::fs::write(&target, emit_cursor(brief))?;
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;

    fn full_brief() -> Brief {
        Brief {
            frontmatter: Frontmatter {
                stack: vec!["TypeScript 5.4".into(), "React 18".into()],
                context: vec!["./docs/architecture.md".into()],
                ..Default::default()
            },
            goal: "Add real-time notifications".into(),
            constraints: Constraints {
                hard: vec![
                    "Must not degrade page load time by more than 100ms".into(),
                    "All notifications delivered within 5 seconds".into(),
                ],
                soft: vec!["Prefer WebSocket over polling".into()],
                ask_first: vec!["Changes to the notification schema".into()],
            },
            sacred: vec![SacredEntry {
                path: "src/auth/**".into(),
                reason: "SOC2 audited".into(),
                well_formed: true,
            }],
            assumptions: vec![
                Assumption {
                    text: "Gateway can handle 5k concurrent connections".into(),
                    validated: false,
                    has_checkbox: true,
                },
                Assumption {
                    text: "REST API supports event subscriptions".into(),
                    validated: true,
                    has_checkbox: true,
                },
            ],
            deliverable: Some(
                "Working notification system with real-time delivery and read tracking.".into(),
            ),
            unknown_sections: vec![UnknownSection {
                heading: "Commands".into(),
                content: "- Build: `npm run build`".into(),
            }],
        }
    }

    // -- Frontmatter shape --

    #[test]
    fn emit_starts_with_yaml_frontmatter() {
        let brief = full_brief();
        let output = emit_cursor(&brief);
        assert!(
            output.starts_with("---\n"),
            "expected YAML frontmatter opening, got:\n{output}"
        );
    }

    #[test]
    fn emit_frontmatter_uses_goal_as_description() {
        let brief = full_brief();
        let output = emit_cursor(&brief);
        // Cursor's `description` is what the model consults to decide whether
        // to load the rule when alwaysApply is false. Goal is the most natural
        // single-line summary.
        assert!(
            output.contains("description: Add real-time notifications\n"),
            "expected `description` derived from goal, got:\n{output}"
        );
    }

    #[test]
    fn emit_frontmatter_sets_always_apply_true() {
        // No scope info in brief today, so the only correct mapping is the
        // always-on activation mode.
        let brief = full_brief();
        let output = emit_cursor(&brief);
        assert!(
            output.contains("alwaysApply: true\n"),
            "expected `alwaysApply: true` in frontmatter, got:\n{output}"
        );
    }

    #[test]
    fn emit_frontmatter_omits_globs() {
        // Brief has no glob-scoping concept, so emitting an empty `globs: []`
        // would be misleading. Per Cursor's activation matrix, alwaysApply:true
        // makes globs irrelevant anyway — leaving it out is the honest move.
        let brief = full_brief();
        let output = emit_cursor(&brief);
        let frontmatter_end = output.find("\n---\n").expect("frontmatter must close");
        let fm = &output[..frontmatter_end];
        assert!(
            !fm.contains("globs:"),
            "expected no `globs:` key while brief has no scoped constraints, got frontmatter:\n{fm}"
        );
    }

    #[test]
    fn emit_frontmatter_closes_before_body() {
        let brief = full_brief();
        let output = emit_cursor(&brief);
        // Frontmatter must close cleanly with a `---` line before any body
        // content. Cursor's parser will silently fail otherwise.
        let after_open = &output[4..]; // skip the leading "---\n"
        assert!(
            after_open.contains("\n---\n"),
            "expected closing `---` after frontmatter, got:\n{output}"
        );
    }

    // -- Body sections --

    #[test]
    fn emit_includes_goal_as_h1() {
        let brief = full_brief();
        let output = emit_cursor(&brief);
        assert!(
            output.contains("# Add real-time notifications"),
            "expected H1 with goal, got:\n{output}"
        );
    }

    #[test]
    fn emit_renders_stack_line() {
        let brief = full_brief();
        let output = emit_cursor(&brief);
        assert!(
            output.contains("**Stack:** TypeScript 5.4, React 18"),
            "expected Stack line, got:\n{output}"
        );
    }

    #[test]
    fn emit_renders_hard_constraints_descriptively() {
        // Cursor's idiomatic register is descriptive, not imperative — so we
        // do NOT use `**IMPORTANT:**` prefixes (those are Claude-flavored).
        let brief = full_brief();
        let output = emit_cursor(&brief);
        assert!(output.contains("## Required"));
        assert!(output.contains("- Must not degrade page load time by more than 100ms"));
        assert!(
            !output.contains("**IMPORTANT:**"),
            "expected descriptive register, not Claude's imperative style"
        );
    }

    #[test]
    fn emit_renders_soft_constraints() {
        let brief = full_brief();
        let output = emit_cursor(&brief);
        assert!(output.contains("## Preferred"));
        assert!(output.contains("- Prefer WebSocket over polling"));
    }

    #[test]
    fn emit_renders_ask_first_constraints() {
        let brief = full_brief();
        let output = emit_cursor(&brief);
        assert!(output.contains("## Ask First"));
        assert!(output.contains("- Changes to the notification schema"));
    }

    #[test]
    fn emit_renders_sacred_as_protected_files() {
        let brief = full_brief();
        let output = emit_cursor(&brief);
        assert!(output.contains("## Protected Files"));
        assert!(output.contains("`src/auth/**`"));
        assert!(output.contains("SOC2 audited"));
    }

    #[test]
    fn emit_renders_only_unvalidated_assumptions() {
        let brief = full_brief();
        let output = emit_cursor(&brief);
        // Validated assumptions don't need verification, so they're noise in
        // the rule — only surface what the agent should still confirm.
        assert!(output.contains("## Verify"));
        assert!(output.contains("Gateway can handle 5k concurrent connections"));
        assert!(
            !output.contains("REST API supports event subscriptions"),
            "validated assumptions should not appear in Verify section"
        );
    }

    #[test]
    fn emit_renders_deliverable() {
        let brief = full_brief();
        let output = emit_cursor(&brief);
        assert!(output.contains("## Deliverable"));
        assert!(output.contains("Working notification system"));
    }

    #[test]
    fn emit_renders_context_files() {
        let brief = full_brief();
        let output = emit_cursor(&brief);
        assert!(output.contains("## Context"));
        assert!(output.contains("`./docs/architecture.md`"));
    }

    #[test]
    fn emit_passes_through_unknown_sections() {
        let brief = full_brief();
        let output = emit_cursor(&brief);
        assert!(output.contains("## Commands"));
        assert!(output.contains("- Build: `npm run build`"));
    }

    // -- Empty / minimal handling --

    #[test]
    fn emit_minimal_brief_omits_empty_sections() {
        let brief = Brief {
            frontmatter: Frontmatter::default(),
            goal: "Fix it".into(),
            constraints: Constraints::default(),
            sacred: vec![],
            assumptions: vec![],
            deliverable: None,
            unknown_sections: vec![],
        };
        let output = emit_cursor(&brief);
        assert!(output.contains("# Fix it"));
        assert!(!output.contains("## Required"));
        assert!(!output.contains("## Preferred"));
        assert!(!output.contains("## Ask First"));
        assert!(!output.contains("## Protected Files"));
        assert!(!output.contains("## Verify"));
        assert!(!output.contains("## Deliverable"));
        assert!(!output.contains("## Context"));
        assert!(!output.contains("**Stack:**"));
    }

    // -- Frontmatter escaping --

    #[test]
    fn emit_quotes_description_with_yaml_special_chars() {
        // Goals containing `:`, `#`, `[`, etc. would break unquoted YAML.
        let brief = Brief {
            frontmatter: Frontmatter::default(),
            goal: "Fix bug: handle [nested] {edge} cases".into(),
            constraints: Constraints::default(),
            sacred: vec![],
            assumptions: vec![],
            deliverable: None,
            unknown_sections: vec![],
        };
        let output = emit_cursor(&brief);
        let frontmatter_end = output.find("\n---\n").expect("frontmatter must close");
        let fm = &output[..frontmatter_end];
        // Whatever escaping we use, YAML must round-trip back to the goal.
        let parsed: serde_yaml::Value = serde_yaml::from_str(fm.trim_start_matches("---\n"))
            .expect("frontmatter must be valid YAML");
        assert_eq!(
            parsed["description"].as_str(),
            Some("Fix bug: handle [nested] {edge} cases")
        );
    }

    // -- install_cursor --

    #[test]
    fn install_cursor_writes_to_dot_cursor_rules_brief_mdc() {
        let dir = tempfile::tempdir().unwrap();
        let brief = full_brief();

        let written = install_cursor(&brief, dir.path()).unwrap();

        let expected = dir.path().join(".cursor").join("rules").join("brief.mdc");
        assert_eq!(written, expected);
        assert!(expected.exists(), "expected file at {expected:?}");
    }

    #[test]
    fn install_cursor_creates_directory_if_missing() {
        let dir = tempfile::tempdir().unwrap();
        let brief = full_brief();

        // .cursor/rules does not exist yet
        assert!(!dir.path().join(".cursor").exists());

        install_cursor(&brief, dir.path()).unwrap();

        assert!(dir.path().join(".cursor").join("rules").is_dir());
    }

    #[test]
    fn install_cursor_overwrites_existing_brief_mdc() {
        let dir = tempfile::tempdir().unwrap();
        let rules_dir = dir.path().join(".cursor").join("rules");
        std::fs::create_dir_all(&rules_dir).unwrap();
        let target = rules_dir.join("brief.mdc");
        std::fs::write(&target, "stale content").unwrap();

        let brief = full_brief();
        install_cursor(&brief, dir.path()).unwrap();

        let result = std::fs::read_to_string(&target).unwrap();
        assert!(!result.contains("stale content"));
        assert!(result.contains("# Add real-time notifications"));
    }

    #[test]
    fn install_cursor_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let brief = full_brief();

        install_cursor(&brief, dir.path()).unwrap();
        let first = std::fs::read_to_string(dir.path().join(".cursor/rules/brief.mdc")).unwrap();

        install_cursor(&brief, dir.path()).unwrap();
        let second = std::fs::read_to_string(dir.path().join(".cursor/rules/brief.mdc")).unwrap();

        assert_eq!(first, second);
    }

    #[test]
    fn install_cursor_does_not_touch_other_mdc_files_in_rules_dir() {
        let dir = tempfile::tempdir().unwrap();
        let rules_dir = dir.path().join(".cursor").join("rules");
        std::fs::create_dir_all(&rules_dir).unwrap();
        let other = rules_dir.join("hand-written.mdc");
        std::fs::write(&other, "user-authored rule\n").unwrap();

        let brief = full_brief();
        install_cursor(&brief, dir.path()).unwrap();

        let other_content = std::fs::read_to_string(&other).unwrap();
        assert_eq!(other_content, "user-authored rule\n");
    }
}
