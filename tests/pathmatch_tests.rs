//! `docs/bugs.md`: `brief check` marked non-sacred files sacred because the
//! matcher compared string prefixes, so `flat` prefixed `flatter` and
//! `src/auth/**` blocked `src/authz/…`.
//!
//! The table below is the one in `bugfix-sacred-glob-aider.brief.md`, row for
//! row. It is answered without touching the filesystem: hooks run `brief check`
//! on files that are about to be created.

use brief_cli::pathmatch::path_matches_pattern;

#[test]
fn matches_on_path_components_never_on_string_prefixes() {
    let cases = [
        ("flat/**", "flat/a.md", true),
        ("flat/**", "flat/sub/b.md", true),
        ("flat/**", "flatter/x.md", false),
        ("flat/**", "flat", true),
        ("flat/*", "flat/a.md", true),
        ("flat/*", "flat/sub/b.md", false),
        ("migrations/", "migrations/001.sql", true),
        ("migrations/", "migrations_old/001.sql", false),
        ("src/auth/**", "src/authz/x.rs", false),
        ("**/auth/**", "src/auth/h.rs", true),
    ];
    for (pattern, path, want) in cases {
        assert_eq!(
            path_matches_pattern(pattern, path),
            want,
            "pattern `{pattern}` against `{path}`"
        );
    }
}

/// The regressions the old string-prefix branch caused, spelled out: a longer
/// sibling directory is not inside the pattern's directory.
#[test]
fn a_longer_sibling_directory_is_not_sacred() {
    for path in [
        "src/authz/x.rs",
        "src/authentication/h.rs",
        "src/auth_backup/h.rs",
    ] {
        assert!(
            !path_matches_pattern("src/auth/**", path),
            "`{path}` must not be inside `src/auth/**`"
        );
    }
    assert!(path_matches_pattern("src/auth/**", "src/auth/h.rs"));
}

/// A bare file path is a pattern too, and matches only itself.
#[test]
fn a_literal_file_pattern_matches_only_that_file() {
    assert!(path_matches_pattern("docs/bugs.md", "docs/bugs.md"));
    assert!(!path_matches_pattern("docs/bugs.md", "docs/bugs.md.bak"));
    assert!(!path_matches_pattern("docs/bugs.md", "docs/other.md"));
}

/// Both sides may be written with a leading `./`; a brief's sacred list and a
/// hook's payload do not agree on that.
#[test]
fn a_leading_dot_slash_is_ignored_on_both_sides() {
    assert!(path_matches_pattern("./src/auth/**", "src/auth/h.rs"));
    assert!(path_matches_pattern("src/auth/**", "./src/auth/h.rs"));
    assert!(!path_matches_pattern("./src/auth/**", "./src/authz/h.rs"));
}

/// Hooks ask about files that do not exist yet, in a directory that may not
/// exist either. The answer must be the same as for a file already on disk.
#[test]
fn the_answer_does_not_depend_on_the_file_existing() {
    assert!(path_matches_pattern(
        "no/such/dir/**",
        "no/such/dir/brand_new.rs"
    ));
    assert!(!path_matches_pattern(
        "no/such/dir/**",
        "no/such/dirx/brand_new.rs"
    ));
}
