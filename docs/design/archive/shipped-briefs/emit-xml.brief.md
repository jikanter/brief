---
stack: [Rust]
context: [./CLAUDE.md, ./AGENTS.md, ./README.md, ./docs, ./src, ./tests/fixtures, ./examples/sample.brief.md]
---

# Add `brief emit xml`: a hard-boundary XML envelope target for injection-adjacent contexts

## Why this target exists (read before designing)

This is **not** a token-efficiency play. Wrapper format is worth ~1% of context
budget; `--budget` is where savings live. XML earns its place for exactly one
reason: **explicit closing tags give each section a terminus.** A Markdown
heading ends only when the next heading appears, which is a real failure mode
when a briefing is concatenated with untrusted or bulky content — a diff, a file
bundle, tool output — inside a CI prompt. This target is for that case.

Design every decision against that goal. If a choice doesn't make boundaries
harder to misread, it isn't justified.

## Orientation (do this first, report before writing code)

1. Read `CLAUDE.md` and `AGENTS.md`.
2. Pick the two most dissimilar existing emit targets in `src/` (suggest `claude`
   and `anchor`) and describe the actual shape of a target: its trait/enum
   registration, how `--install` / `--uninstall` / `--budget` / `--compact` are
   wired, and how output is tested. Name files and types.
3. Report how much of a target is boilerplate vs. genuinely target-specific.
   If adding a target means duplicating traversal logic across all sections,
   **say so and stop** — that's the IR extraction problem and I want to know
   before you hand-roll a seventh emitter.
4. State whether an install surface exists for this target. My position: it does
   **not** — see Hard constraints.

Report findings, then proceed.

## Deliverable

`brief emit xml` producing a deterministic, well-formed XML envelope of the
briefing, at full parity with existing stdout-mode targets, plus tests and docs.

## Constraints

### Hard

- **No `--install` / `--uninstall` for this target.** There is no canonical
  on-disk location for it; it is a stdout/pipe target consumed by a CI step.
  Do not invent a dotfile path. `--install` must fail with a clear message
  saying why and pointing at `brief emit claude --install`.
- **Well-formed XML, always.** All content is escaped so that `Record<T>`,
  `&`, `"`, and glob patterns survive intact and cannot terminate a tag early.
  Choose escaping vs. CDATA deliberately and justify it in a comment.
  A fixture containing angle brackets, ampersands, and a fenced code block
  MUST round-trip through a real XML parser in tests.
- **Deterministic output.** Byte-identical across runs for identical input.
  No timestamps, no hostnames, no absolute paths, no HashMap iteration order.
- **Semantically lossless vs. the prompt target.** Any Hard constraint, Sacred
  path, or Deliverable present in `brief emit prompt` must be present here.
  Nothing silently dropped.
- **Sacred paths emit verbatim**, one element per pattern, as the normative
  value. Do not resolve or expand globs against the working tree.
- `--budget` and `--compact` behave consistently with other stdout targets.
  `--compact` strips reference prose; it must never strip Hard constraints or
  Sacred paths, and must not produce malformed XML.
- Do not change the output of any existing target. Existing tests pass unchanged.
- No new dependencies without asking (see Ask First).

### Soft

- Tag vocabulary should read like Anthropic's documented shape: lowercase,
  semantic, no namespace prefixes, no XML declaration unless you can justify it.
- Prefer attributes for enumerable metadata (constraint severity), elements for
  authored prose. Be consistent.
- Match the existing error type and CLI help conventions rather than inventing new ones.
- Keep the emitter readable enough that a future `--dialect` flag could reuse
  its section ordering. Do not build that flag now.

### Ask First

- Adding an XML-writer crate (e.g. `quick-xml`) rather than escaping manually.
  I lean toward a dependency for correctness, but ask.
- Any change to the IR, parser, or frontmatter schema.
- Adding `--dialect`, a generic template/Jinja layer, or a plugin ABI.
  **All three are out of scope and I will reject them.**
- Emitting resolved/expanded Sacred globs as a derived field.

## Sacred

- `src/**/emit_claude*`, `src/**/emit_agents*`, `src/**/emit_prompt*`,
  `src/**/emit_anchor*` — existing targets are untouched by this change
- `tests/fixtures/**` — do not edit existing fixtures; add new ones alongside

## Assumptions (confirm or correct before coding)

- [ ] Targets are registered in one place and adding one is a bounded change
- [ ] There is a shared code path for stdout-mode emit that this can join
- [ ] `--budget` reporting is target-agnostic and needs no per-target work
- [ ] Golden-file comparison is the established way emit output is tested here

## Tests (required)

- Golden-file test for a representative brief, including the full section set.
- **Well-formedness test**: parse the output with a real XML parser, for every
  fixture in `tests/fixtures` that is expected to parse.
- **Escaping test**: new fixture containing `<`, `>`, `&`, `"`, `'`, a fenced
  code block, and a Sacred glob like `src/**/*.rs` — assert values survive
  a parse/extract round-trip byte-exact.
- **Determinism test**: emit twice, assert byte equality.
- **Parity test**: every Hard constraint and Sacred path in the fixture appears
  in the XML output.
- **`--compact` test**: still well-formed; Hard + Sacred retained.
- **`--install` test**: fails with the explanatory message, writes nothing.

## Done when
`cargo test` green, `brief emit xml` on this repo's own `.brief.md` is
well-formed and stable across runs, README's emit list and docs updated, and
the "why XML" rationale above is captured in a doc comment on the emitter so
the next person doesn't re-litigate it as a token optimization.