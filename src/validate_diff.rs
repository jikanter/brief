//! Sacred-region gate over a set of changed files.
//!
//! This is the testable core of `brief validate-diff`: given a parsed brief and
//! a list of changed file paths, report which paths fall inside a sacred region.
//! The git plumbing and output formatting live in the CLI layer (`main.rs`); this
//! module is pure so it can be unit-tested without a repo.

use std::path::Path;

use serde::Serialize;

use crate::check::check_path;
use crate::model::Brief;

/// A single changed file that landed inside a sacred region.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Violation {
    /// The changed file path, as reported by the diff.
    pub file: String,
    /// The sacred pattern it matched.
    pub pattern: String,
    /// The human reason attached to that sacred region.
    pub reason: String,
}

/// Outcome of checking a batch of changed files against the brief's sacred regions.
#[derive(Debug, Clone, Serialize)]
pub struct DiffReport {
    /// How many files were checked.
    pub checked: usize,
    /// Files that violated a sacred region.
    pub violations: Vec<Violation>,
}

impl DiffReport {
    /// True when no changed file touched a sacred region.
    pub fn is_clean(&self) -> bool {
        self.violations.is_empty()
    }
}

/// Check every changed file against the brief's sacred regions.
///
/// `base_dir` is the directory the brief lives in; file paths are interpreted
/// relative to it (the same convention `brief check` uses).
pub fn check_changed_files(brief: &Brief, files: &[String], base_dir: &Path) -> DiffReport {
    let mut violations = Vec::new();

    for file in files {
        let result = check_path(brief, file, base_dir);
        if result.is_sacred {
            violations.push(Violation {
                file: file.clone(),
                pattern: result.matching_pattern.unwrap_or_else(|| "unknown".into()),
                reason: result.reason.unwrap_or_default(),
            });
        }
    }

    DiffReport {
        checked: files.len(),
        violations,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;

    fn sample_brief() -> Brief {
        Brief {
            frontmatter: Frontmatter {
                stack: vec!["Rust".into()],
                ..Default::default()
            },
            goal: "Test".into(),
            identity: None,
            constraints: Constraints::default(),
            sacred: vec![
                SacredEntry {
                    path: "src/auth/**".into(),
                    reason: "Auth logic".into(),
                    well_formed: true,
                },
                SacredEntry {
                    path: "migrations/".into(),
                    reason: "Historical migrations".into(),
                    well_formed: true,
                },
            ],
            assumptions: vec![],
            deliverable: None,
            unknown_sections: vec![],
        }
    }

    fn base() -> &'static Path {
        // Sacred matching is prefix-based; the base dir need not exist on disk.
        Path::new("/nonexistent/base")
    }

    #[test]
    fn clean_diff_has_no_violations() {
        let brief = sample_brief();
        let files = vec!["src/api/routes.rs".to_string(), "README.md".to_string()];
        let report = check_changed_files(&brief, &files, base());
        assert_eq!(report.checked, 2);
        assert!(report.is_clean());
    }

    #[test]
    fn sacred_file_is_flagged() {
        let brief = sample_brief();
        let files = vec!["src/auth/handler.rs".to_string()];
        let report = check_changed_files(&brief, &files, base());
        assert!(!report.is_clean());
        assert_eq!(report.violations.len(), 1);
        assert_eq!(report.violations[0].file, "src/auth/handler.rs");
        assert_eq!(report.violations[0].pattern, "src/auth/**");
        assert_eq!(report.violations[0].reason, "Auth logic");
    }

    #[test]
    fn mixed_diff_flags_only_sacred_files() {
        let brief = sample_brief();
        let files = vec![
            "src/api/routes.rs".to_string(),
            "migrations/001_init.sql".to_string(),
            "src/auth/token.rs".to_string(),
        ];
        let report = check_changed_files(&brief, &files, base());
        assert_eq!(report.checked, 3);
        assert_eq!(report.violations.len(), 2);
        let flagged: Vec<&str> = report.violations.iter().map(|v| v.file.as_str()).collect();
        assert!(flagged.contains(&"migrations/001_init.sql"));
        assert!(flagged.contains(&"src/auth/token.rs"));
        assert!(!flagged.contains(&"src/api/routes.rs"));
    }

    #[test]
    fn empty_diff_is_clean() {
        let brief = sample_brief();
        let report = check_changed_files(&brief, &[], base());
        assert_eq!(report.checked, 0);
        assert!(report.is_clean());
    }

    #[test]
    fn report_serializes_to_json() {
        let brief = sample_brief();
        let files = vec!["src/auth/handler.rs".to_string()];
        let report = check_changed_files(&brief, &files, base());
        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("\"checked\":1"));
        assert!(json.contains("\"file\":\"src/auth/handler.rs\""));
        assert!(json.contains("\"pattern\":\"src/auth/**\""));
        assert!(json.contains("\"reason\":\"Auth logic\""));
    }
}
