---
stack: [Rust, pulldown-cmark]
context: [./CLAUDE.md, ./AGENTS.md, ./docs/bugs.md, ./docs/brief-format.md, ./docs/schema/SPEC.md, ./src/parse/body.rs, ./src/model.rs, ./src/emit, ./tests/fixtures, ./tests/parse_tests.rs, ./tests/emit_tests.rs]
model: claude-opus-5
brief_version: "1"
---

# Stop the body parser silently dropping inline markdown from goals, constraints and the deliverable

One cause, four symptoms, recorded as the first entry in `docs/bugs.md`. The
body parser rebuilds text from a whitelist of `pulldown-cmark` events, so every
event it does not name is discarded without a diagnostic. Authors get emitted
output that is quietly missing what they wrote.

Unlike `bugfix-sacred-glob-aider.brief.md`, this fix **is expected to change
emitted bytes**, so golden fixtures move with it. Land that brief first if both
are queued; they do not overlap in the files they touch.

## Orientation (do this first, report before writing code)

1. Reproduce every row of the symptom table below. If one does not reproduce,
   stop and report.
2. Read `src/parse/body.rs` and inventory every `Event::` arm and its guard
   (`in_heading`, `in_item`, `in_paragraph`). Produce a grid: event type across
   the three contexts, marking handled or dropped. That grid is the fix's
   checklist.
3. Say which struct fields carry the affected text (`goal`, `Constraint.text`,
   `Sacred.reason`, `Assumption.text`, `deliverable`, `identity`,
   `unknown_sections[].content`) and, for each, whether emitters re-wrap it or
   pass it through.
4. Confirm that no fixture in `tests/fixtures` contains a link, emphasis, or a
   code span inside a heading. Report what that means for the golden files.

Report findings, then proceed. One commit per symptom group, plus a first
commit that only adds failing fixtures and tests.

## Symptoms (all confirmed on c2571c0)

| # | Input | Parses as | Should be |
|---|---|---|---|
| 1 | H1 ``# Add `brief emit xml` to the tool`` | `Add  to the tool` | ``Add `brief emit xml` to the tool`` |
| 2 | H2 ``## Notes on `src/check.rs` `` | `Notes on` | ``Notes on `src/check.rs` `` |
| 3 | `- See [the design doc](./docs/design.md) for context` | `See the design doc for context` | `See [the design doc](./docs/design.md) for context` |
| 4 | ``- Use `serde` and **never** `unwrap` `` | ``Use `serde` and never `unwrap` `` | ``Use `serde` and **never** `unwrap` `` |
| 5 | Deliverable: two paragraphs | `Para one.Para two.` | `Para one.\n\nPara two.` |

Symptom 1 and 2 are the sharpest: an `Event::Code` arm exists for `in_item` but
not for `in_heading`, so a code span in a heading is not merely unwrapped, it is
deleted along with its contents. Symptom 3 is the existing "strips markdown
links from soft sections" entry, which is under-scoped: it affects the goal,
all three constraint tiers, sacred reasons, assumptions and the deliverable.

## Expected behavior

**Rule: parsed text is the author's source markdown for that span.** A round
trip through the parser preserves what the author typed, minus the block
syntax that the heading grammar itself consumes (`#`, `- `, `- [ ] `).

- Inline code keeps its backticks, in every context. An item already does this.
- A link is reconstructed as `[label](url)`; a title, if present, is kept.
  A bare autolink `<https://x>` and a reference-style link keep their source
  spelling.
- `**strong**` and `_emphasis_` keep their markers. Reconstruct with the
  delimiter the author used where the event stream allows it; if it does not,
  normalize to `**` and `_` and say so in the report.
- Paragraphs inside a prose section are joined with a blank line. A soft break
  inside one paragraph stays a single newline, which is today's behavior.
- An event type still not handled after this fix must not fail silently. Add a
  catch-all that preserves the text payload where one exists.

Out of scope: nested structures inside a list item (a sub-list, a fenced block
inside an item). If one appears, keep today's behavior and note it.

## Consequences to handle

- Golden fixtures under `tests/fixtures/golden/` change wherever a fixture
  gains inline markup. Regenerate them in the same commit as the fix that
  changes them, and state in the report which files moved and why.
- `brief emit xml` escapes text for XML. Markdown that now survives into a
  constraint must still be escaped correctly; a link URL containing `&` is the
  test case.
- `brief check` matches sacred paths parsed from `` `path` `` spans. Confirm
  the sacred parse path is unaffected, since it strips backticks by its own
  rule rather than through the code-span arm.
- Token budgets rise slightly because markers are no longer dropped.
  `tests/budget_cli.rs` may need new expected numbers; do not relax a budget
  threshold to accommodate it.

## Constraints

### Hard

- No new crates: `Cargo.toml` `[dependencies]` is unchanged.
- No change to `docs/schema/brief-v1.schema.json`,
  `docs/schema/brief-frontmatter-v1.schema.json`, or to any field name or type
  in `src/model.rs`. This fixes the *contents* of existing string fields only.
- No `brief_version` bump. Author-facing grammar in `docs/brief-format.md` §3
  is unchanged; only what the parser retains changes.
- A golden fixture is only regenerated when this work genuinely changes it.
  Diff each one and justify it in the report; never bless a golden without
  reading the diff.
- The first commit adds failing fixtures and tests and changes no `src/` file.
- `cargo test`, `cargo clippy -- -D warnings`, and `cargo fmt --check` pass at
  every commit.

### Soft

- Prefer one reusable inline-span collector used by the heading, item and
  paragraph contexts over three parallel match arms that drift apart again.
- Prefer extending `tests/parse_tests.rs` with a table mirroring the symptom
  table row for row.
- Add the missing coverage to an existing fixture rather than creating a new
  one, so every emitter sees the markup through its existing golden.
- Keep the collector free of emitter concerns: it reconstructs markdown, and
  each emitter decides how to render or escape it.

### Ask First

- Any change to `Constraint.text` semantics as documented in
  `docs/schema/SPEC.md` §3 ("the authored sentence").
- Rendering markdown into a register an emitter cannot escape, if one is found
  that cannot take a link.
- Changing the deliverable paragraph separator to anything other than a blank
  line.
- Touching `docs/brief-format.md` §3 body grammar.

## Sacred

- `docs/schema/*` — machine schemas; this fix changes field contents, never field shapes
- `docs/reference/standards/*` — vendored third-party specs

## Assumptions

- [ ] `pulldown-cmark`'s event stream exposes enough to distinguish `*`/`_` and `**`/`__` delimiters; if not, normalizing is acceptable
- [ ] No emitter or downstream consumer depends on link URLs being absent from constraint text
- [ ] The sacred-entry parse path strips backticks independently and is unaffected
- [x] All five symptoms reproduce on c2571c0
- [x] No existing fixture contains a link, emphasis, or a code span in a heading, so no test protects the current behavior

## Deliverable

A parser that returns the author's source markdown for every inline span, in
headings, list items and prose alike, with paragraphs separated by a blank
line. Commit one adds the failing fixtures and tests; the following commits fix
the parser and regenerate only the goldens that genuinely moved. The first
entry in `docs/bugs.md` is flipped to `[x]`. A short report: the event grid
from Orientation, how each assumption resolved, every golden file that changed
with its reason, and any emitter that needed an escaping fix.
