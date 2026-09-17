//! `docs/bugs.md`: `brief validate` warned that `dir/**` matched no files
//! whenever `dir` held only files and no subdirectories, because `glob::glob`
//! yields nothing for a trailing `/**` over a flat directory. One helper now
//! answers the question for the sacred check and the constraint-scope check.

use brief_cli::pathmatch::pattern_matches_any_file;

/// `docs/bugs.md`: `brief validate` warned that `flat/**` matched no files
/// whenever `flat` had no subdirectories, because `glob::glob` yields nothing
/// for a trailing `/**` over a flat directory.
#[test]
fn reports_whether_a_pattern_matches_at_least_one_file() {
    let dir = tempfile::TempDir::new().unwrap();
    let base = dir.path();
    std::fs::create_dir_all(base.join("flat")).unwrap();
    std::fs::write(base.join("flat/a.md"), "x").unwrap();
    std::fs::create_dir_all(base.join("nested/sub")).unwrap();
    std::fs::write(base.join("nested/sub/b.md"), "x").unwrap();
    std::fs::create_dir_all(base.join("empty")).unwrap();
    std::fs::create_dir_all(base.join("migrations")).unwrap();
    std::fs::write(base.join("migrations/001.sql"), "x").unwrap();
    std::fs::create_dir_all(base.join("src")).unwrap();
    std::fs::write(base.join("src/notes.txt"), "x").unwrap();

    let cases = [
        ("flat/**", true),
        ("nested/**", true),
        ("empty/**", false),
        ("missing/**", false),
        ("migrations/", true),
        ("src/*.rs", false),
        ("**/*.md", true),
        ("flat/a.md", true),
    ];
    for (pattern, want) in cases {
        assert_eq!(
            pattern_matches_any_file(base, pattern).unwrap(),
            want,
            "pattern `{pattern}`"
        );
    }
}

#[test]
fn an_invalid_pattern_is_an_error_not_a_false() {
    let dir = tempfile::TempDir::new().unwrap();
    assert!(pattern_matches_any_file(dir.path(), "src/[").is_err());
}

/// A directory alone is not a file. `empty/**` is dead weight and must say so,
/// which is the one place this helper disagrees with the old scope check.
#[test]
fn a_directory_holding_only_directories_matches_no_files() {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::create_dir_all(dir.path().join("shell/inner")).unwrap();
    assert!(!pattern_matches_any_file(dir.path(), "shell/**").unwrap());
}
