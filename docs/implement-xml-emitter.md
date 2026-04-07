# Implementation Prompt: `brief emit xml` Target

## Context

`brief` is a Rust CLI that parses `.brief.md` files (Markdown + YAML frontmatter) and emits them to multiple target formats. The existing emit targets are:

- `claude` — Markdown for CLAUDE.md injection (`src/emit/claude.rs`)
- `prompt` — Plain uppercase-labeled text for API system prompts (`src/emit/prompt.rs`)
- `agents-md` — AGENTS.md format (`src/emit/agents_md.rs`)
- `json` — Structured JSON (`src/emit/json.rs`)
- `skill` — Claude Code SKILL.md with YAML frontmatter (`src/emit/skill.rs`)

All emitters follow the same pattern: a public function that takes `&Brief` and returns `String`. The `Brief` struct is defined in `src/model.rs`.

Research conducted in `docs/analysis/terse-prompt-format-research.md` evaluated six terse prompt formats and recommended Anthropic-style XML tags as the next emit target. This is supported by:
- Anthropic's own prompting docs recommending XML for structured instructions
- arxiv 2411.10541 showing up to 40% performance variation by format
- ~20-30% token reduction vs Markdown
- 1:1 mapping between brief's data model and XML tag structure

## Task

Add a new `xml` emit target to brief: `brief emit xml`.

## What to build

### 1. New emitter: `src/emit/xml.rs`

Create `src/emit/xml.rs` with a public `emit_xml(brief: &Brief) -> String` function.

The output format should be:

```xml
<brief>
<goal>{{brief.goal}}</goal>
<stack>{{brief.frontmatter.stack, comma-separated}}</stack>

<context>
<file>{{each context file, stripped of leading ./}}</file>
</context>

<constraints>
<hard>
<rule>NEVER/MUST: {{each hard constraint, reframed with imperative verbs}}</rule>
</hard>
<soft>
<rule>PREFER/AVOID: {{each soft constraint}}</rule>
</soft>
<ask-first>
<rule>STOP and confirm before: {{each ask-first constraint}}</rule>
</ask-first>
</constraints>

<sacred>
<region path="{{glob}}">{{reason}}</region>
</sacred>

<assumptions>
<unvalidated>{{each unvalidated assumption}}</unvalidated>
<validated>{{each validated assumption}}</validated>
</assumptions>

<deliverable>{{deliverable text}}</deliverable>

<section name="{{heading}}">{{content}}</section>
</brief>
```

**Design rules for the emitter:**

- Wrap everything in a top-level `<brief>` tag.
- Omit empty sections entirely (same pattern as all other emitters — if `brief.sacred` is empty, emit no `<sacred>` block).
- Omit `<stack>` if the stack is empty.
- Omit `<context>` if context is empty.
- Apply the Phase 2 constraint reframing from `docs/design-decisions.md`:
  - Hard constraints: prefix with `NEVER:` or `MUST:` (use the original text — if the constraint already starts with an imperative verb, don't double-prefix; if it doesn't, prepend `MUST:`)
  - Soft constraints: prefix with `PREFER:` or `AVOID:` (same logic)
  - Ask-first constraints: prefix with `STOP and confirm before:`
- Separate validated and unvalidated assumptions into distinct sub-tags.
- Unknown sections become `<section name="heading">content</section>`.
- Context file paths: strip leading `./` (same as `claude.rs`).
- XML-escape `&`, `<`, `>` in user content to produce well-formed output. A simple `fn escape_xml(s: &str) -> String` helper is sufficient — no need for an XML library dependency.
- Do NOT add an XML declaration (`<?xml ...?>`) — this is not meant to be parsed by XML tools; it's prompt content for an LLM.

### 2. Wire into CLI

In `src/main.rs`:
- Add `Xml` variant to the `EmitTarget` enum (look for `#[derive(...)] enum EmitTarget`).
- Add `EmitTarget::Xml => emit::emit_xml(&brief)` to the match in `cmd_emit()`.

In `src/emit/mod.rs`:
- Add `pub mod xml;`
- Add `pub use xml::emit_xml;`

### 3. Unit tests in `src/emit/xml.rs`

Follow the pattern in `claude.rs` and `prompt.rs`. At minimum:

- `xml_contains_goal_and_stack` — basic output structure
- `xml_contains_constraints_with_reframing` — hard constraints get MUST/NEVER, soft get PREFER/AVOID, ask-first get STOP prefix
- `xml_contains_sacred_regions` — `<region path="...">` format
- `xml_separates_assumptions` — unvalidated and validated in separate sub-tags
- `xml_omits_empty_sections` — no `<sacred>` block if no sacred entries, etc.
- `xml_contains_unknown_sections` — `<section name="...">` passthrough
- `xml_escapes_special_characters` — `&`, `<`, `>` in content are escaped
- `xml_contains_context_files` — context files listed, `./` prefix stripped
- `xml_omits_stack_when_empty` — no `<stack>` tag if stack is empty

### 4. Integration test in `tests/emit_tests.rs`

Add a test that parses the `full.brief.md` fixture and emits via `emit_xml`, then asserts:
- Output starts with `<brief>` and ends with `</brief>`
- Contains `<goal>Build real-time collaborative document editor</goal>`
- Contains `<stack>` with all 5 stack items
- Contains `<hard>`, `<soft>`, `<ask-first>` blocks
- Contains `<sacred>` with `<region path="src/core/crdt-engine/**">`
- Contains `<unvalidated>` and `<validated>` assumption tags
- Contains `<deliverable>`
- Contains `<section name="Commands">` and `<section name="Code Style">` for unknown sections
- Contains `<context>` with `<file>` entries

Also add a minimal fixture test (`emit_xml_from_minimal_fixture`) asserting goal and basic structure.

### 5. Test fixture (optional)

No new fixture files needed — reuse `full.brief.md` and `minimal.brief.md`.

## What NOT to build

- No `--install` support for xml. This target is for piping into API system prompts, not for injecting into files.
- No XML declaration or DTD.
- No new dependencies. String formatting is sufficient.
- No changes to the parser, model, or other emitters.
- No emitter trait refactor (per design-decisions.md: "Build when the 7th emit target justifies the abstraction cost"). This is target #6.

## Files to modify

| File | Change |
|------|--------|
| `src/emit/xml.rs` | **New file.** `emit_xml()` + `escape_xml()` + unit tests |
| `src/emit/mod.rs` | Add `pub mod xml;` and `pub use xml::emit_xml;` |
| `src/main.rs` | Add `Xml` to `EmitTarget` enum, add match arm in `cmd_emit()` |
| `tests/emit_tests.rs` | Add `emit_xml_from_full_fixture` and `emit_xml_from_minimal_fixture` |

## Verification

After implementation, all of the following must pass:

```bash
cargo build
cargo test
cargo clippy
cargo run -- emit xml tests/fixtures/full.brief.md
cargo run -- emit xml tests/fixtures/minimal.brief.md
```

Inspect the output of the last two commands to verify the XML structure looks correct and all sections are present.

## Reference

- Research analysis: `docs/analysis/terse-prompt-format-research.md`
- Design decisions: `docs/design-decisions.md` (especially "Emit-time reframing" and "Emit as Prompt Engineering")
- Phase 2 synthesis: `docs/analysis/phase2-synthesis.md`
- Existing emitter to follow as pattern: `src/emit/prompt.rs` (simplest), `src/emit/claude.rs` (closest in structure)
- Brief data model: `src/model.rs`
