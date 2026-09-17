---
stack: [Rust, serde, serde_yaml, schemars, jsonschema]
context: [./CLAUDE.md, ./README.md, ./docs, ./src, ./tests/fixtures, ./examples/sample.brief.md]
---

# Add `brief_version` to the format and generate the frontmatter JSON Schema from Rust types

## Orientation (do this first, report before writing code)

Read `CLAUDE.md` and `AGENTS.md`. Then inventory what already exists:

1. Locate the frontmatter parsing path in `src/` — the struct(s) that `stack` and
   `context` deserialize into. Name the file and type in your report.
2. `git log --oneline -- docs/ tests/` and inspect the commit described as
   "schema tests plus new design docs". There may already be a schema effort,
   design doc, or test harness in tree. **Extend it; do not duplicate it.**
   If it conflicts with this briefing, stop and tell me.
3. List every fixture in `tests/fixtures` and classify each well-formed / malformed.

Report findings, then proceed.

## Deliverable

1. `brief_version` as a frontmatter key, parsed, validated, and round-tripped.
2. A JSON Schema for the frontmatter, **generated from the Rust types**, committed
   in-repo and embedded in the binary via `include_str!`.
3. A test that fails if the committed schema drifts from the derived one.
4. Docs updated: README format section + `examples/sample.brief.md`.

## Constraints

### Hard

- The schema describes **frontmatter only**. Do not attempt to model Markdown body
  structure (headings, Constraints buckets, Sacred, Assumptions) in JSON Schema.
  Body validation stays in the existing parser/lint path.
- Schema is **derived**, not hand-written.