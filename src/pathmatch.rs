//! The one place brief decides whether a path matches a pattern.
//!
//! Two questions are asked of a sacred path or a constraint scope, and until
//! `docs/bugs.md` entries 2 and 3 they were answered by three different pieces
//! of code that disagreed:
//!
//! 1. *Is this file inside this pattern?* — [`path_matches_pattern`], used by
//!    `brief check`, the `--hook` entry point, and `brief validate --diff`.
//!    Pure: hooks ask about files that are about to be created, so consulting
//!    the filesystem would give a different answer before and after a write.
//! 2. *Does this pattern cover at least one real file?* — [`pattern_matches_any_file`],
//!    used by `brief validate` for both the sacred check and the scope check.
//!
//! Matching is on path components. String prefixes were the old bug: `flat`
//! prefixes `flatter`, so `flat/**` blocked `flatter/x.md`, and `src/auth/**`
//! blocked every edit under `src/authz/`.

use std::path::Path;

use glob::{MatchOptions, Pattern, PatternError};

/// Characters that make a pattern a glob rather than a literal path.
const GLOB_META: [char; 3] = ['*', '?', '['];

/// `require_literal_separator` is what keeps `flat/*` from reaching
/// `flat/sub/b.md`: a single `*` stays inside one path component.
const MATCH_OPTIONS: MatchOptions = MatchOptions {
    case_sensitive: true,
    require_literal_separator: true,
    require_literal_leading_dot: false,
};

/// True when `path` falls inside `pattern`.
///
/// Both sides are repository-relative and may carry a leading `./`. A pattern
/// with no glob characters is a literal file or a directory prefix: it matches
/// that path and, for a directory, everything beneath it. A trailing `/**` also
/// covers the directory itself, so a brief can mark `src/auth/**` sacred and
/// have `brief check src/auth` agree.
///
/// Never touches the filesystem.
pub fn path_matches_pattern(pattern: &str, path: &str) -> bool {
    let pattern = normalize(pattern);
    let path = normalize(path);
    if pattern.is_empty() || path.is_empty() {
        return false;
    }

    if !pattern.contains(GLOB_META) {
        // A literal path: itself, or a directory holding this path.
        let prefix = pattern.trim_end_matches('/');
        return path == prefix || is_component_prefix(path, prefix);
    }

    if matches_glob(pattern, path) {
        return true;
    }

    // `dir/**` names the directory as well as its contents.
    if let Some(prefix) = pattern.strip_suffix("/**") {
        return path == prefix || matches_glob(prefix, path);
    }

    false
}

/// True when at least one existing *file* under `base_dir` matches `pattern`.
///
/// A pattern that matches only directories matches no files: `empty/**` over an
/// empty directory is dead weight and says so. `Err` is reserved for a pattern
/// the glob crate cannot compile, which callers report differently from a
/// pattern that simply covers nothing.
pub fn pattern_matches_any_file(base_dir: &Path, pattern: &str) -> Result<bool, PatternError> {
    let normalized = normalize(pattern);
    // Compile first: an unparseable pattern is an error, not a `false`.
    Pattern::new(normalized)?;

    // Only walk the subtree the pattern can possibly reach.
    let root = base_dir.join(literal_root(normalized));
    Ok(walk_until_match(&root, base_dir, normalized))
}

/// The leading path components of `pattern` that contain no glob characters.
/// `src/api/**` yields `src/api`; `**/*.test.ts` yields `""`, meaning the whole
/// tree. This is only a walk optimization — the verdict still comes from
/// [`path_matches_pattern`].
fn literal_root(pattern: &str) -> String {
    let mut root = Vec::new();
    for component in pattern.split('/') {
        if component.contains(GLOB_META) {
            break;
        }
        root.push(component);
    }
    // A literal file pattern's last component is the file itself; walking from
    // its parent is equivalent and keeps the walk rooted at a directory.
    if root.len() == pattern.split('/').count() {
        root.pop();
    }
    root.join("/")
}

/// Depth-first walk under `root`, stopping at the first file that matches.
/// Symlinked directories are not followed, so a cycle cannot hang validate.
fn walk_until_match(root: &Path, base_dir: &Path, pattern: &str) -> bool {
    let Ok(entries) = std::fs::read_dir(root) else {
        return false;
    };
    let mut dirs = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_dir() {
            if entry.file_name() == ".git" {
                continue;
            }
            dirs.push(path);
            continue;
        }
        if let Ok(rel) = path.strip_prefix(base_dir)
            && path_matches_pattern(pattern, &rel.to_string_lossy())
        {
            return true;
        }
    }
    dirs.iter().any(|d| walk_until_match(d, base_dir, pattern))
}

fn matches_glob(pattern: &str, path: &str) -> bool {
    Pattern::new(pattern)
        .map(|p| p.matches_with(path, MATCH_OPTIONS))
        .unwrap_or(false)
}

/// True when `path` sits under the directory `prefix`, comparing whole
/// components: `migrations` is a prefix of `migrations/001.sql` but not of
/// `migrations_old/001.sql`.
fn is_component_prefix(path: &str, prefix: &str) -> bool {
    !prefix.is_empty()
        && path.len() > prefix.len()
        && path.starts_with(prefix)
        && path.as_bytes()[prefix.len()] == b'/'
}

fn normalize(s: &str) -> &str {
    s.trim_start_matches("./")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literal_root_stops_at_the_first_glob_character() {
        assert_eq!(literal_root("src/api/**"), "src/api");
        assert_eq!(literal_root("**/*.test.ts"), "");
        assert_eq!(literal_root("src/*.rs"), "src");
        assert_eq!(literal_root("migrations/"), "migrations");
        // A literal file: walk from its parent directory.
        assert_eq!(literal_root("docs/bugs.md"), "docs");
    }

    #[test]
    fn component_prefix_rejects_a_longer_sibling() {
        assert!(is_component_prefix("migrations/001.sql", "migrations"));
        assert!(!is_component_prefix("migrations_old/001.sql", "migrations"));
        assert!(!is_component_prefix("migrations", "migrations"));
    }
}
