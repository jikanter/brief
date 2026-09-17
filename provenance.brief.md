---
stack: [Rust, serde, serde-saphyr, serde_json, schemars, clap]
context: [./CLAUDE.md, ./AGENTS.md, ./docs/design/provenance-schema.md, ./docs/schema/SPEC.md, ./docs/brief-format.md, ./docs/reference/standards/agentskills/specification.md, ./src/validate.rs, ./src/skill.rs, ./src/parse/frontmatter.rs, ./src/emit/aider.rs, ./tests/fixtures]
model: claude-opus-5
brief_version: "1"
skill_name: implement-document-provenance
skill_description: "Implement the document provenance vocabulary v0.1 in `brief validate`, and replace the archived `serde_yaml` with `serde-saphyr`"
---

# Implement the provenance vocabulary v0.1 in `brief validate`, and replace `serde_yaml` with `serde-saphyr`

The spec is `docs/design/provenance-schema.md`. It is the contract. Where this
brief and the spec disagree, stop and report; do not pick one.

## Orientation (do this first, report before writing code)

1. Read `CLAUDE.md`, `AGENTS.md`, and the spec end to end (§1.1, §3–§7 are
   normative; §8–§9 are rationale).
2. List every `serde_yaml` call site (`grep -rn serde_yaml src tests`) and say,
   per site, what it needs: typed deserialize, a dynamic value, or serialize.
   Expect: `src/parse/frontmatter.rs`, `src/skill.rs`, `src/emit/aider.rs`
   (`Mapping`/`Value` + `to_string`), `src/emit/cursor.rs` tests,
   `tests/frontmatter_schema_tests.rs`.
3. Read `src/validate.rs` (`validate(&Brief, base_dir)`, `Severity`), the
   `Validate` arm in `src/main.rs`, and `git_changed_files` in `src/main.rs`
   (the existing pattern for shelling out to git).
4. Read `src/skill.rs` `get_brief_source` / `set_brief_source` /
   `is_brief_owned` and find where a generated `SKILL.md` is written. Name the
   function where a `brief.dateModified` stamp would go, and say how that path
   decides today whether to rewrite an existing file.

Report findings, then proceed phase by phase. One commit per phase. Use the
test-driven-development skill: failing test first, every phase.

## Phases

### Phase 1 — swap the YAML parser (pure refactor)

- Remove `serde_yaml`; add `serde-saphyr` (1.x). It has no dynamic `Value`
  type; use `serde_json::Value` / `serde_json::Map` where one is needed, with
  serde_json's `preserve_order` feature so `merge_aider_conf` keeps key order.
- No behavior change. Every existing test passes unmodified except for import
  and type names. `tests/frontmatter_schema_tests.rs` stays green and
  `docs/schema/brief-frontmatter-v1.schema.json` stays byte-identical.
- Add one regression test per scalar-typing risk: a `stack:` entry that looks
  like a number or boolean (`3.12`, `no`), and a date-like string, parse to the
  same `Frontmatter` as before.

### Phase 2 — `src/provenance.rs`: read and resolve (pure, no I/O)

- Input: the frontmatter text of any Markdown file. Output: a resolved struct
  `{ date_modified, date_created, is_based_on, superseded_by, brief_written }`
  plus a list of address disagreements.
- Read the frontmatter as a map and resolve by hand. Do **not** model aliases
  with `#[serde(alias)]`: serde rejects a document that carries both a canonical
  key and its alias as a duplicate field, and the spec requires "canonical wins,
  disagreement is a warning" (§4).
- Closed alias list and resolution order exactly as spec §4. No case- or
  separator-insensitive matching. Aliases never apply inside `metadata`.
- `metadata.brief.dateModified` and `metadata.brief.source` are the fall-through
  addresses. `brief_written` = any `metadata.brief.*` key present (§5 rule 3).
- Calendar day = the first 10 characters of a value that starts with
  `YYYY-MM-DD`. No timezone conversion, no date crate. Anything else resolves
  as unset, silently.
- A reference value matching `^[a-z][a-z0-9+.-]*://` is a URL (covers
  `https://` and `ace://prefix/id`). Otherwise it is a path relative to the
  carrying document's directory.
- A non-string value (list, map, number) for a provenance key resolves as unset.
- No frontmatter, unclosed frontmatter, or YAML that fails to parse: empty
  result, no diagnostic. These are other people's files.

### Phase 3 — `brief validate` checks

- Run the provenance pass over the brief file itself and over each local
  `context:` entry that is a readable Markdown file. Skip URLs and directories.
- Implement the §6 table. Everything is `Warning` except the newer-than-brief
  check, which needs a new `Severity::Hint` and is reported only under a new
  `brief validate --hints` flag. Hints never affect the exit code. Nothing in
  this phase may produce an `Error`.
- Git drift: `git log -1 --format=%cs -- <file>`, following the
  `git_changed_files` pattern. Not a work tree, git missing, or file untracked:
  skip silently. Apply to human-owned `dateModified` only; a brief-written
  file's stamp is governed by Phase 4, not by this check.
- `superseded_by` chain: follow local paths up to 8 hops, detect cycles, name
  the end of the chain in the `context:` warning.
- The brief's own top-level provenance keys are not in the `Brief` model and
  must not be added to it. Pass the raw file text (or its path) to the
  provenance pass alongside `&Brief`.

### Phase 4 — stamp generated `SKILL.md`

- Generalize `set_brief_source` into a `metadata.brief.<key>` line-splice
  setter; `set_brief_source` becomes a caller. Same preservation guarantees,
  same tests, plus tests for a second key.
- Write `brief.dateModified` as a quoted ISO 8601 UTC string
  (`"2026-04-11T19:22:00Z"`), derived from `SystemTime` with a small
  days-to-civil helper.
- Rewrite the stamp only when the generated content, compared with the stamp
  line excluded, differs from the file on disk (§5 rule 5). Test: emit twice
  with a fake clock one day apart, second run is a byte-for-byte no-op.
- The clock must be injectable so tests and golden fixtures stay deterministic.

### Phase 5 — docs

- Spec header: `Status: Implemented in <crate version>`.
- `docs/brief-format.md` §2 and `README.md`: one sentence each on what
  `brief validate` now reports and the `--hints` flag.
- `CLAUDE.md` Dependencies list: `serde_yaml` → `serde-saphyr`. Edit outside the
  `<brief:generated>` region only.

## Constraints

### Hard

- Provenance keys are not `Frontmatter` fields. No change to the `Frontmatter`
  struct, to `docs/schema/brief-frontmatter-v1.schema.json`, to
  `docs/schema/brief-v1.schema.json`, or to `brief emit json` output.
- No `brief_version` bump.
- Brief writes provenance only under `metadata.brief.*`, as flat dotted keys
  with quoted string values. Never a top-level provenance key, in any file.
- No provenance check produces an error or changes the exit code.
- Emitters stay pure: `&Brief` in, `String` out. No emitter opens a context file.
- Files brief does not own are never deserialized and re-emitted. Writes are
  text splices that leave every other byte unchanged.
- Unknown top-level keys produce no diagnostic of any kind.
- One YAML parser in the binary after Phase 1. `serde_yaml` is gone from
  `Cargo.toml` and `Cargo.lock`.
- No network calls. URL-valued references are never fetched.
- Brief does not validate ACE prefixes or tag ids; it only recognizes the URI
  shape.
- `cargo test`, `cargo clippy -- -D warnings`, and `cargo fmt --check` pass at
  every phase commit.

### Soft

- Prefer comparing `YYYY-MM-DD` strings over introducing date types.
- Prefer extending `tests/validate_tests.rs` and `tests/skill_commands.rs` over
  new test files; add fixtures under `tests/fixtures/provenance/`.
- Keep `src/provenance.rs` free of `std::fs` and `std::process`; do I/O in the
  validate layer so the resolver is unit-testable from strings.
- Diagnostic messages name the file and the key as the author spelled it
  (`updated_at`, not `dateModified`).

### Ask First

- Adding any crate other than `serde-saphyr` (a date/time crate in particular).
- Enabling a serde_json feature other than `preserve_order`.
- Any edit to `docs/design/provenance-schema.md` beyond the Phase 5 status line.
- Changing `merge_aider_conf` behavior. Its comment-loss bug is fixed by
  `bugfix-sacred-glob-aider.brief.md`, which should land first. If it has, the
  merge is a line splice and Phase 1 only ports its read-only parse and its
  tests. If it has not, port the re-emit faithfully and do not fix it here.
- A flag name other than `--hints`.

## Sacred

- `docs/schema/*` — machine schemas; this work adds nothing to them and the frontmatter schema must stay byte-identical
- `docs/reference/standards/**` — vendored third-party specs
- `tests/fixtures/golden/*` — golden emit output; provenance must not change any emitted byte

## Assumptions

- [ ] `serde-saphyr` parses every file in `tests/fixtures` and `examples/` to the same `Frontmatter` as `serde_yaml` 0.9
- [ ] `serde-saphyr` deserializes an unquoted `2026-04-11` or `2026-04-11T14:22:00-05:00` into `serde_json::Value::String`, not a number or a tagged type
- [ ] `git log --format=%cs` is available on the git versions this tool targets (added in git 2.21)
- [ ] The skill-emit path has a single place where `SKILL.md` frontmatter is finalized, so the stamp has one write site
- [x] Brief emits `context:` entries as path references and never inlines them (`src/emit/claude.rs`, `src/emit/prompt.rs`)
- [x] `validate` resolves `context:` paths against the brief file's directory (`src/validate.rs`)

## Deliverable

`brief validate` reports the §6 checks for the brief and its local context docs,
as warnings, with the newer-than-brief hint behind `--hints`; nothing errors and
no emitted output changes. A generated `SKILL.md` carries
`metadata.brief.dateModified`, rewritten only when content changes. `serde_yaml`
is gone. Five commits, one per phase, each green. A short report listing: the
four assumptions and how each resolved, any spec ambiguity hit, and the
`merge_aider_conf` finding.
