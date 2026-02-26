use std::path::Path;

use crate::model::Brief;

/// Result of checking a path against sacred regions.
pub struct CheckResult {
    pub is_sacred: bool,
    pub reason: Option<String>,
    pub matching_pattern: Option<String>,
}

/// Check if a file path falls within any sacred region defined in the brief.
pub fn check_path(brief: &Brief, file_path: &str, base_dir: &Path) -> CheckResult {
    let target = base_dir.join(file_path);

    for entry in &brief.sacred {
        let pattern = base_dir.join(&entry.path).to_string_lossy().to_string();
        if let Ok(matches) = glob::glob(&pattern) {
            for matched in matches.flatten() {
                if matched == target {
                    return CheckResult {
                        is_sacred: true,
                        reason: Some(entry.reason.clone()),
                        matching_pattern: Some(entry.path.clone()),
                    };
                }
            }
        }

        // Also check if the target is a prefix match (for directory patterns like `migrations/`)
        let pattern_path = Path::new(&entry.path);
        let file_path_obj = Path::new(file_path);
        if file_path_obj.starts_with(pattern_path) {
            return CheckResult {
                is_sacred: true,
                reason: Some(entry.reason.clone()),
                matching_pattern: Some(entry.path.clone()),
            };
        }

        // Check without glob characters for directory prefix matching
        let clean_pattern = entry.path.trim_end_matches("/**").trim_end_matches("/*");
        if file_path.starts_with(clean_pattern) {
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
}
