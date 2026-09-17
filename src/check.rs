use std::path::Path;

use crate::model::Brief;
use crate::pathmatch::path_matches_pattern;

/// Result of checking a path against sacred regions.
pub struct CheckResult {
    pub is_sacred: bool,
    pub reason: Option<String>,
    pub matching_pattern: Option<String>,
}

/// Check if a file path falls within any sacred region defined in the brief.
///
/// Matching goes through [`path_matches_pattern`], which compares path
/// components and never consults the filesystem: the `--hook` entry point calls
/// this on files that are about to be created, and the verdict must not change
/// once they exist.
///
/// `base_dir` is accepted for signature stability with the callers in
/// `src/main.rs` and `src/validate_diff.rs`; the answer does not depend on it.
pub fn check_path(brief: &Brief, file_path: &str, _base_dir: &Path) -> CheckResult {
    for entry in &brief.sacred {
        if path_matches_pattern(&entry.path, file_path) {
            return CheckResult {
                is_sacred: true,
                reason: Some(entry.reason.clone()),
                matching_pattern: Some(entry.path.clone()),
            };
        }
    }

    CheckResult {
        is_sacred: false,
        reason: None,
        matching_pattern: None,
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

    #[test]
    fn path_in_sacred_region_is_detected() {
        let brief = sample_brief();
        let tmp = tempfile::TempDir::new().unwrap();
        let result = check_path(&brief, "src/auth/handler.rs", tmp.path());
        assert!(result.is_sacred);
        assert_eq!(result.reason.unwrap(), "Auth logic");
    }

    #[test]
    fn path_outside_sacred_region_is_ok() {
        let brief = sample_brief();
        let tmp = tempfile::TempDir::new().unwrap();
        let result = check_path(&brief, "src/api/routes.rs", tmp.path());
        assert!(!result.is_sacred);
    }

    #[test]
    fn migration_path_is_sacred() {
        let brief = sample_brief();
        let tmp = tempfile::TempDir::new().unwrap();
        let result = check_path(&brief, "migrations/001_init.sql", tmp.path());
        assert!(result.is_sacred);
        assert_eq!(result.reason.unwrap(), "Historical migrations");
    }

    #[test]
    fn a_sibling_directory_sharing_a_prefix_is_not_sacred() {
        let brief = sample_brief();
        let tmp = tempfile::TempDir::new().unwrap();
        for path in ["src/authz/x.rs", "migrations_old/001.sql"] {
            assert!(
                !check_path(&brief, path, tmp.path()).is_sacred,
                "`{path}` must not be sacred"
            );
        }
    }

    #[test]
    fn a_file_that_does_not_exist_yet_is_still_sacred() {
        let brief = sample_brief();
        let tmp = tempfile::TempDir::new().unwrap();
        assert!(check_path(&brief, "src/auth/brand_new.rs", tmp.path()).is_sacred);
    }
}
