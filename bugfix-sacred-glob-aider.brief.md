---
stack: [Rust, glob, clap]
context: [./CLAUDE.md, ./AGENTS.md, ./docs/bugs.md, ./docs/brief-format.md, ./src/check.rs, ./src/validate.rs, ./src/emit/aider.rs, ./src/skill.rs, ./tests/validate_tests.rs]
model: claude-opus-5
brief_version: "1"
skill_name: fix-sacred-glob-and-aider-merge
skill_description: "Fix sacred-path false positives in `brief check`, a false \"matches no files\" in `brief validate`, and comment loss in the aider config merge"
---

# Fix three bugs: sacred false positives in `brief check`, a false "matches no files" in `brief validate`, and comment loss in the aider config merge

All three are recorded in `docs/bugs.md` and were reproduced on 3194559 + the
provenance docs changes. Fix in the order below: bug 1 blocks legitimate edits
through the hooks, bug 3 damages a user's file, bug 2 is a wrong warning.

## Orientation (do this first, report before writing code)

1. Reproduce each bug with the recipe in its section. If one does not
   reproduce, stop and report.
2. Find every place a sacred path or a constraint scope glob is matched against
   a file path or against the filesystem: `src/check.rs`, `src/validate.rs`
   (sacred check and scope check), `Constraint::is_directory_prefix`, the hook
   entry point `run_check_hook` in `src/main.rs`, and any shell or JSON hook
   brief installs for Claude or Cursor that does its own matching. List them
   and say which share code today.
3. Read `merge_aider_conf` and `ensure_read_includes_conventions` in
   `src/emit/aider.rs` and their tests. Read `set_brief_source` in
   `src/skill.rs`: it is the in-repo precedent for editing a user-owned YAML
   block by line splice.

Report findings, then proceed. One commit per bug. Use the
test-driven-development skill: the failing test is the reproduction.

## Bug 1 — `brief check` marks non-sacred files sacred

Repro: brief with `` - `flat/**` — r ``; files `flat/a.md` and `flatter/x.md`.
`brief check flatter/x.md` exits 1 "is in sacred region `flat/**`".

Cause: `check_path` trims `/**` or `/*` from the pattern and tests
`file_path.starts_with(clean_pattern)` on strings, so `flat` prefixes
`flatter`. `src/auth/**` therefore also blocks `src/authz/…` and
`src/authentication/…`.

Expected:

| Pattern | Path | Sacred |
|---|---|---|
| `flat/**` | `flat/a.md` | yes |
| `flat/**` | `flat/sub/b.md` | yes |
| `flat/**` | `flatter/x.md` | no |
| `flat/**` | `flat` | yes |
| `flat/*` | `flat/a.md` | yes |
| `flat/*` | `flat/sub/b.md` | no |
| `migrations/` | `migrations/001.sql` | yes |
| `migrations/` | `migrations_old/001.sql` | no |
| `src/auth/**` | `src/authz/x.rs` | no |
| `**/auth/**` | `src/auth/h.rs` | yes |

- Match on path components, never on string prefixes.
- The result must not depend on whether the target file exists: hooks run
  `check` on files about to be created. Today the first branch walks the
  filesystem and only the string fallbacks cover new files.

## Bug 3 — aider config merge drops the user's comments

Repro: `.aider.conf.yml` containing

```yaml
# my comment
read: other.md # keep
auto-commits: false
```

`brief emit aider --install` rewrites it without either comment.

Cause: `merge_aider_conf` deserializes to a map and re-serializes.

Expected:

- Every byte outside the `read:` entry and an added `model:` line is unchanged:
  comments, blank lines, key order, quoting style, trailing newline state.
- Already satisfied (`read` includes `CONVENTIONS.md`, and `model` is set or the
  brief names none) returns the input byte-identical.
- `read` absent: append it. Scalar `read`: promote to a list holding the
  original value then `CONVENTIONS.md`; an inline comment on the scalar line
  stays with the original value's item. Block list: append an item at the
  siblings' indentation. Flow list (`read: [a, b]`): insert before `]`.
- `model` is added only when the brief names one and the file has none. A
  user's `model` is never overwritten. (Unchanged behavior.)
- A shape the splicer cannot edit safely (anchors or aliases on `read`,
  multi-document file, `read` nested under another key, tabs for indentation)
  is an error that names the file and writes nothing. Never fall back to
  re-emitting.
- Parsing the file read-only to decide which case applies is fine. Writing from
  the parsed value is not.
- Re-running on its own output is a byte-for-byte no-op for every case above.

## Bug 2 — `brief validate` says `dir/**` matches no files when `dir` is flat

Repro: `flat/a.md` exists, no subdirectories, sacred entry `flat/**`.
`brief validate` warns "Sacred path `flat/**` matches no files".

Cause: `glob::glob` yields only directories for a trailing `**`. The
constraint-scope check a few lines below already guards with
`directory_prefix_exists`; the sacred check does not.

Expected:

| Sacred or scope pattern | Filesystem | Warning |
|---|---|---|
| `flat/**` | `flat/a.md` | none |
| `nested/**` | `nested/sub/b.md` | none |
| `empty/**` | `empty/` with nothing in it | "matches no files" |
| `missing/**` | no such directory | "matches no files" |
| `migrations/` | `migrations/001.sql` | none |
| `src/*.rs` | no `.rs` files in `src/` | "matches no files" |

- One helper answers "does this pattern match at least one file" for both the
  sacred check and the scope check. If the `empty/**` row changes what the
  scope check reports today, say so in the report.

## Constraints

### Hard

- `brief check` exit codes and message format are unchanged for every case that
  was already correct. Hooks parse them.
- No new crates: `Cargo.toml` `[dependencies]` is unchanged.
- A file brief does not own is never deserialized and re-emitted.
- Each bug's commit contains its failing test, the fix, and the `docs/bugs.md`
  line flipped to `[x]`.
- `cargo test`, `cargo clippy -- -D warnings`, and `cargo fmt --check` pass at
  every commit.

### Soft

- One path-matching function shared by `check`, `validate`, and the hook entry
  point, so they cannot disagree again.
- Prefer `glob::Pattern::matches_path_with` over walking the filesystem when
  the question is "does this path match".
- Table-driven tests that mirror the tables above row for row.
- Reuse the line-splice helpers in `src/skill.rs` if they fit; do not
  generalize them if they do not.

### Ask First

- Changing what a bare directory pattern (`migrations/`, no glob characters)
  means.
- Changing the YAML that `merge_aider_conf` produces when there is no existing
  file.
- Touching any installed hook script's matching logic, if Orientation finds one
  that does not go through `check_path`.

## Sacred

- `tests/fixtures/golden/*` — golden emit output; none of these fixes changes an emitted byte
- `docs/schema/*` — machine schemas; out of scope

## Assumptions

- [ ] Every installed hook reaches `check_path`; no hook script matches paths on its own
- [ ] `glob::Pattern` with `require_literal_separator: true` gives the `flat/*` rows above
- [ ] No existing test or fixture depends on the string-prefix behavior of bug 1
- [x] All three bugs reproduce on the current tree (2026-09-17)

## Deliverable

Three commits, each green, each with its reproduction as a test. `brief check`
matches by path component and gives the same answer for existing and
not-yet-created files. `brief emit aider --install` leaves every byte it does
not own untouched and is a no-op when re-run. `brief validate` no longer warns
on a flat directory. A short report: the matcher inventory from Orientation,
how each assumption resolved, and any behavior change to the scope check.

Sequencing: land this before `provenance.brief.md`. With bug 3 fixed,
`src/emit/aider.rs` no longer needs a dynamic YAML value to write, which
shrinks that brief's Phase 1.
