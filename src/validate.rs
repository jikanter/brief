use std::path::Path;

use crate::model::{Brief, Diagnostic, Severity};

/// Validate a parsed `Brief` against the format spec and the filesystem.
///
/// Returns a list of diagnostics. Exit 0 if no errors, exit 1 if any errors.
pub fn validate(brief: &Brief, base_dir: &Path) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // 1. Stack must be non-empty
    if brief.frontmatter.stack.is_empty() {
        diagnostics.push(Diagnostic {
            severity: Severity::Error,
            message: "Missing required `stack` field in frontmatter".to_string(),
        });
    }

    // 2. Must have a goal (H1)
    if brief.goal.is_empty() {
        diagnostics.push(Diagnostic {
            severity: Severity::Error,
            message: "Missing H1 goal statement".to_string(),
        });
    }

    // 3. Check context files exist
    for ctx_path in &brief.frontmatter.context {
        let resolved = base_dir.join(ctx_path);
        if !resolved.exists() {
            diagnostics.push(Diagnostic {
                severity: Severity::Warning,
                message: format!("Context file not found: {ctx_path}"),
            });
        }
    }

    // 4. Check sacred entries are well-formed
    for entry in &brief.sacred {
        if !entry.well_formed {
            diagnostics.push(Diagnostic {
                severity: Severity::Error,
                message: format!(
                    "Malformed sacred entry: path `{}` should be wrapped in backticks",
                    entry.path
                ),
            });
        }
    }

    // 5. Check sacred globs match at least one file
    for entry in &brief.sacred {
        let pattern = base_dir.join(&entry.path).to_string_lossy().to_string();
        match glob::glob(&pattern) {
            Ok(paths) => {
                if paths.count() == 0 {
                    diagnostics.push(Diagnostic {
                        severity: Severity::Warning,
                        message: format!("Sacred path `{}` matches no files", entry.path),
                    });
                }
            }
            Err(e) => {
                diagnostics.push(Diagnostic {
                    severity: Severity::Warning,
                    message: format!("Invalid glob pattern in sacred entry `{}`: {e}", entry.path),
                });
            }
        }
    }

    // 6. Check assumptions have checkboxes
    for assumption in &brief.assumptions {
        if !assumption.has_checkbox {
            diagnostics.push(Diagnostic {
                severity: Severity::Error,
                message: format!(
                    "Assumption missing checkbox syntax: \"{}\"",
                    assumption.text
                ),
            });
        }
    }

    diagnostics
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;
    use tempfile::TempDir;

    fn make_valid_brief() -> Brief {
        Brief {
            frontmatter: Frontmatter {
                stack: vec!["Rust".to_string()],
                context: vec![],
                model: None,
                version: "1".to_string(),
                skill_name: None,
                skill_description: None,
            },
            goal: "Fix the bug".to_string(),
            constraints: Constraints {
                hard: vec!["Don't break tests".to_string()],
                soft: vec![],
                ask_first: vec![],
            },
            sacred: vec![SacredEntry {
                path: "src/auth.rs".to_string(),
                reason: "Auth logic".to_string(),
                well_formed: true,
            }],
            assumptions: vec![Assumption {
                text: "It works".to_string(),
                validated: false,
                has_checkbox: true,
            }],
            deliverable: Some("Working code".to_string()),
            unknown_sections: vec![],
        }
    }

    #[test]
    fn valid_brief_has_no_errors() {
        let tmp = TempDir::new().unwrap();
        // Create the sacred file so the glob matches
        std::fs::create_dir_all(tmp.path().join("src")).unwrap();
        std::fs::write(tmp.path().join("src/auth.rs"), "fn main() {}").unwrap();

        let brief = make_valid_brief();
        let diags = validate(&brief, tmp.path());
        let errors: Vec<_> = diags
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .collect();
        assert!(errors.is_empty(), "Expected no errors, got: {errors:?}");
    }

    #[test]
    fn missing_stack_is_error() {
        let tmp = TempDir::new().unwrap();
        let mut brief = make_valid_brief();
        brief.frontmatter.stack.clear();
        let diags = validate(&brief, tmp.path());
        assert!(diags.iter().any(|d| d.message.contains("stack")));
    }

    #[test]
    fn missing_goal_is_error() {
        let tmp = TempDir::new().unwrap();
        let mut brief = make_valid_brief();
        brief.goal.clear();
        let diags = validate(&brief, tmp.path());
        assert!(diags.iter().any(|d| d.message.contains("H1 goal")));
    }

    #[test]
    fn missing_context_file_is_warning() {
        let tmp = TempDir::new().unwrap();
        let mut brief = make_valid_brief();
        brief.frontmatter.context = vec!["./nonexistent.md".to_string()];
        let diags = validate(&brief, tmp.path());
        let warnings: Vec<_> = diags
            .iter()
            .filter(|d| d.severity == Severity::Warning)
            .collect();
        assert!(warnings.iter().any(|d| d.message.contains("nonexistent")));
    }

    #[test]
    fn malformed_sacred_is_error() {
        let tmp = TempDir::new().unwrap();
        let mut brief = make_valid_brief();
        brief.sacred = vec![SacredEntry {
            path: "no-backticks".to_string(),
            reason: "missing".to_string(),
            well_formed: false,
        }];
        let diags = validate(&brief, tmp.path());
        assert!(diags
            .iter()
            .any(|d| d.severity == Severity::Error && d.message.contains("backticks")));
    }

    #[test]
    fn assumption_without_checkbox_is_error() {
        let tmp = TempDir::new().unwrap();
        let mut brief = make_valid_brief();
        brief.assumptions = vec![Assumption {
            text: "No checkbox".to_string(),
            validated: false,
            has_checkbox: false,
        }];
        let diags = validate(&brief, tmp.path());
        assert!(diags
            .iter()
            .any(|d| d.severity == Severity::Error && d.message.contains("checkbox")));
    }
}
