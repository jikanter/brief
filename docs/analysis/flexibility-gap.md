# The Flexibility Gap: Evolving brief to Produce Full-Featured CLAUDE.md

**Date:** 2026-03-29
**Focus:** Data model, parser, emitters, and what needs to change so that `brief emit claude` can produce a CLAUDE.md that actually covers what a working project needs.

---

## 1. The Flexibility Gap

A production CLAUDE.md (including this project's own) contains categories of information that the current `.brief.md` format has no place for. Examining the `CLAUDE.md` in this repository as a concrete case, the gap is stark.

### Categories Present in Real CLAUDE.md Files That brief Cannot Express

**Build and test commands.** `cargo build`, `cargo test`, `cargo clippy`, `cargo fmt`. These are the first thing an agent reads. The current format has no `## Commands` section and no frontmatter field for them. Constraints could technically hold "Run `cargo test` before committing," but that conflates imperative instructions with declarative constraints.

**Code style rules.** "Use `thiserror` for library errors, `anyhow` for CLI error propagation." "Emitters take `&Brief` and return `String`." "Parse into a strongly-typed `Brief` struct, then operate on that." These are neither hard constraints nor soft constraints -- they are standing conventions that describe how code in this project should look and feel. They are positive patterns, not boundaries.

**Dependency policies.** "Do NOT use heavy frameworks. No `tokio`. No `reqwest`." These are negative patterns (anti-patterns), distinct from hard constraints because they describe what not to reach for during implementation, not what a deliverable must satisfy. The current constraint model mixes these together.

**Project structure descriptions.** The tree diagram showing where modules live and what each file does. This is reference context that an agent needs for navigation. The `context` frontmatter field points to files to read, but there is no inline way to describe the architecture of the project itself.

**Architecture context and decision records.** "Format-first architecture. The `.brief.md` format is the product." "These decisions were reached through extensive design analysis. Do not revisit them." This is neither a constraint nor a sacred region. It is background understanding that calibrates agent behavior.

**Behavioral instructions.** "Tests for every parser edge case." "Integration tests that round-trip: parse a fixture, emit, verify output contains expected content." These tell the agent how to work, not what to build. They are process instructions, and the format has no section for them.

**Format specifications and conventions.** The detailed heading parsing rules, the sacred entry syntax, the YAML frontmatter field definitions. These are domain knowledge that the agent needs when working on the parser -- reference material, not constraints.

### Quantifying the Gap

The current `CLAUDE.md` for this project has roughly 170 lines of meaningful content. Of that, approximately:
- 15% is expressible as frontmatter (stack, context)
- 10% maps to the goal (project summary)
- 15% maps to constraints (hard/soft/ask-first)
- 5% maps to sacred regions
- 0% maps to assumptions (none in the CLAUDE.md)
- 5% maps to deliverable
- **50% has no home in the current format** -- commands, code style, dependency policy, project structure, architecture decisions, format specifications

That 50% is not marginal content. It is the substance of what makes a CLAUDE.md actually useful.

---

## 2. Approach A: Expand Sections

Add first-class sections to the data model and parser for each missing category.

### New Sections Required

1. **`## Commands`** with H3 subsections (`### Build`, `### Test`, `### Lint`, `### Run`). Data model: `commands: BTreeMap<String, Vec<String>>` mapping category names to command strings.

2. **`## Code Style`** as a list of conventions. Data model: `code_style: Vec<String>`.

3. **`## Anti-Patterns`** for negative constraints ("do not use X"). Data model: `anti_patterns: Vec<String>`.

4. **`## Architecture`** for free-form architecture description. Data model: `architecture: Option<String>`.

5. **`## Decisions`** for architecture decision records. Data model: `decisions: Vec<Decision>` where `Decision` has `text: String` and optionally `rationale: String`.

### New Frontmatter Fields

- `commands` as a YAML map (e.g., `build: "cargo build"`, `test: "cargo test"`). This is structured data that fits frontmatter better than prose.
- `dependencies_policy` (string) for a short dependency stance.

### Impact Assessment

**Parser complexity:** Each new first-class section adds a variant to the `Section` enum, a branch in the H2 match, and item-processing logic. The current parser has 5 section variants plus `Unknown`. Adding 5 more (Commands, CodeStyle, AntiPatterns, Architecture, Decisions) nearly doubles the state machine. The H3 handling under Commands would mirror Constraints, adding another level of complexity.

**Emitter maintenance burden:** Every new section must be handled in all 5 emitters. With 5 new sections across 5 emitters, that is 25 new code blocks, each with its own formatting logic. The skill emitter needs different phrasing from the claude emitter, which needs different phrasing from the prompt emitter. This is the existing problem (hand-coded string builders) multiplied.

**60-second authoring target:** Currently, a `.brief.md` scaffold has 6 sections. Adding 5 more means the scaffolded template becomes 11 sections, many of which will be empty placeholders. The user stares at a document that is more template than content. For a quick bug fix, most of these sections are irrelevant noise. The authoring speed drops because the user must decide what to skip.

**Section sprawl risk:** High. Once sections are first-class, there is pressure to add more: Diagnosis, Verification, Team Context, Phases, Environment, Anti-Patterns. Each addition repeats the parser + 5-emitter update cycle. The format starts to look like a configuration language, not a briefing format.

**Verdict:** This approach works for 2-3 high-value additions but does not scale. Beyond that, it fights the design principle of brevity.

---

## 3. Approach B: Better Documentation of Existing Sections

Lean on `UnknownSection` as the universal extension mechanism. Document conventions for common section names but do not parse them into typed structures.

### How UnknownSection Works Today

The parser captures any H2 heading that is not `Constraints`, `Sacred`, `Assumptions`, or `Deliverable` into `Vec<UnknownSection>`, where each entry has a `heading: String` and `content: String`. The content is collected from paragraphs and list items as plain text.

### How Emitters Handle Unknown Sections Today

They do not. Examining every emitter:

- **`emit_claude`** (`src/emit/claude.rs`): No reference to `unknown_sections`. They are silently dropped.
- **`emit_prompt`** (`src/emit/prompt.rs`): No reference to `unknown_sections`. Dropped.
- **`emit_agents_md`** (`src/emit/agents_md.rs`): No reference to `unknown_sections`. Dropped.
- **`emit_skill`** (`src/emit/skill.rs`): No reference to `unknown_sections`. Dropped.
- **`emit_json`** (`src/emit/json.rs`): Serialized via `serde`, so they appear in JSON output. This is the only emitter that preserves them.

This means the extensibility escape hatch exists in the data model and parser but is a dead end in 4 out of 5 emitters. A user who adds `## Commands` with build instructions will find them silently dropped from `brief emit claude`.

### What Changes Are Needed

1. **All text emitters must emit unknown sections.** The simplest approach: after all known sections, append each unknown section as `## {heading}\n{content}\n\n`. This is a 5-10 line change per emitter.

2. **Parser must preserve richer content.** Currently, `UnknownSection.content` is collected via `segments_to_plain_text`, which strips Markdown formatting. List items become `"- text\n"` strings but lose sub-lists, code blocks, emphasis, and links. For unknown sections to be useful as passthrough content, the parser needs to preserve the raw Markdown between H2 boundaries, not the flattened text.

3. **Conventions must be documented.** Publish a "Recommended Sections" guide listing `## Commands`, `## Code Style`, `## Anti-Patterns`, `## Architecture`, `## Decisions` with examples. Make these discoverable via `brief init --full` which scaffolds them as optional sections.

### Limitations

- No validation. A `## Commands` section with malformed content gets no feedback. The validator cannot check that command strings are syntactically valid because it does not know the section's schema.
- No semantic awareness in emitters. The claude emitter cannot reformat commands into Claude Code conventions because it sees opaque strings.
- No structured access via JSON emit. Unknown sections serialize as `{heading, content}` pairs -- consuming tools cannot reliably extract the build command without string parsing.

**Verdict:** This is the minimum viable fix. It solves the immediate problem (content is no longer silently dropped) but does not enable intelligent, target-specific reformatting.

---

## 4. Approach C: Hybrid

Promote a small number of high-impact categories to first-class status. Leave everything else to UnknownSection with passthrough emission.

### First-Class Promotions (3 sections)

**1. Commands.** Build/test/lint/run commands are the most universally needed content in any CLAUDE.md. They are also highly structured (name-to-command mapping) and benefit from validation (check that executables exist). Add to frontmatter:

```yaml
---
stack: [Rust]
commands:
  build: cargo build
  test: cargo test
  lint: cargo clippy
  format: cargo fmt
---
```

Data model change in `Frontmatter`:
```rust
#[serde(default)]
pub commands: BTreeMap<String, String>,
```

This fits frontmatter because it is structured key-value data, not prose. Emitters format it per-target: the claude emitter outputs `## Commands\n- Build: \`cargo build\`\n...`, the prompt emitter outputs `COMMANDS:\n- build: cargo build\n...`, the skill emitter outputs `## Build & Test\nRun \`cargo test\` to verify...`.

**2. Code Style.** Style conventions directly affect every line of code an agent writes. They deserve first-class status because emitters should format them prominently (they are standing instructions, not one-time constraints). Add as a known H2 section:

```markdown
## Style
- Use `thiserror` for library errors, `anyhow` for CLI
- Emitters take `&Brief` and return `String`
- Tests for every parser edge case
```

Data model: `pub style: Vec<String>`. Parser: add `Style` to the `Section` enum, match on `"Style"` or `"Code Style"` at H2 level. Emitters: output as a dedicated section with target-appropriate framing.

**3. Anti-Patterns.** Negative constraints are fundamentally different from positive constraints and should be separated. Mixing "Must pass CI" with "Never use lodash" in the same list is confusing to both humans and agents. Add as a known H2 section:

```markdown
## Anti-Patterns
- No `tokio` -- this is synchronous
- No `reqwest` -- no network calls in Phase 1
- Avoid Factory patterns in new code
```

Data model: `pub anti_patterns: Vec<String>`. Emitters: format with strong negative framing ("DO NOT", "NEVER", "AVOID") appropriate to each target.

### UnknownSection Passthrough (everything else)

Architecture descriptions, decision records, team context, diagnosis, phases -- these vary wildly between projects and do not benefit from structured parsing. They should flow through as raw Markdown.

Changes required:
1. Store raw Markdown in `UnknownSection.content` instead of flattened text.
2. Emit unknown sections in all text emitters after known sections.
3. Document recommended naming conventions.

### Impact Assessment

- **Parser complexity:** Two new `Section` enum variants (Style, AntiPatterns) plus frontmatter commands. Minimal -- about 40 lines of parser changes.
- **Emitter maintenance:** Three new blocks per emitter (commands, style, anti-patterns) = 15 new blocks total, plus the unknown-section passthrough (~5 lines each). Manageable.
- **60-second authoring:** The scaffold gains two optional sections. `brief init` produces the minimal template; `brief init --full` produces the extended template with all sections. The quick path stays quick.
- **Extensibility:** Unknown sections become the documented extension mechanism, not a silent dead end.

---

## 5. The Emitter Architecture Problem

Regardless of which approach is chosen, the emitter architecture needs structural improvement. The current pattern -- five independent functions, each hand-building strings with no shared abstraction -- has concrete costs.

### Current State

Each emitter is a standalone function: `fn emit_X(brief: &Brief) -> String`. They share no code. The formatting logic for each section is duplicated 5 times with target-specific variations. There is no way to add a new section or field without touching all 5 files.

### The Trait Abstraction

Introduce an `Emitter` trait:

```rust
pub trait Emitter {
    fn emit_header(&self, brief: &Brief) -> String;
    fn emit_commands(&self, commands: &BTreeMap<String, String>) -> String;
    fn emit_constraints(&self, constraints: &Constraints) -> String;
    fn emit_sacred(&self, sacred: &[SacredEntry]) -> String;
    fn emit_style(&self, style: &[String]) -> String;
    fn emit_anti_patterns(&self, anti_patterns: &[String]) -> String;
    fn emit_assumptions(&self, assumptions: &[Assumption]) -> String;
    fn emit_deliverable(&self, deliverable: &str) -> String;
    fn emit_unknown_section(&self, section: &UnknownSection) -> String;

    fn emit(&self, brief: &Brief) -> String {
        // Default implementation calls each method in order,
        // skipping empty sections. Implementors override
        // individual methods, not the full pipeline.
    }
}
```

This has three benefits:

1. **New sections require one trait method with a default implementation**, not 5 independent code blocks. Emitters that do not need special handling inherit the default.

2. **Unknown section passthrough gets a default implementation.** The base `emit_unknown_section` emits `## {heading}\n{content}\n\n`. Emitters that need special formatting override it.

3. **Testing becomes compositional.** Test each section-emission method independently rather than parsing full output strings.

### Should Emitters Be Pluggable/Configurable?

Not yet. The 5 current targets have meaningfully different formatting needs (CLAUDE.md vs. raw prompt vs. AGENTS.md vs. JSON vs. SKILL.md). A configuration-driven template system would need to express those differences, which amounts to reimplementing the emitters in a template language. The trait abstraction provides the right level of extensibility for Phase 2: new targets implement the trait, override the methods they need, and inherit sensible defaults for the rest.

A template system (e.g., Tera or Handlebars) could be considered for Phase 3 when community contributors want to add targets without writing Rust, but that is premature now.

### Raw Markdown Preservation

The most impactful single change to the emitter pipeline is upstream of the emitters: the parser must preserve raw Markdown for unknown sections. Currently, `body.rs` collects text from `pulldown-cmark` events and strips formatting. For passthrough emission, the parser should instead record the byte range (start offset, end offset) of each unknown section in the original input, allowing emitters to splice the raw Markdown directly. This avoids the lossy text-flattening path entirely.

An alternative: keep the current event-based collection but preserve Markdown syntax (emit backticks around code spans, preserve list markers, preserve emphasis markers). This is more work per event type but avoids the complexity of tracking byte offsets through pulldown-cmark's event stream.

The pragmatic middle ground: for unknown sections specifically, capture the raw input substring between the H2 heading and the next H2 (or EOF). This is a simple string-slicing operation on the original input, requires no changes to the pulldown-cmark event processing for known sections, and gives emitters lossless content for passthrough.

---

## 6. Concrete Recommendations

Ranked by impact (ratio of user-facing value to implementation cost):

### 1. Emit Unknown Sections in All Text Emitters

**Impact: High. Cost: Low (20-30 lines across 4 files).**

This is the single change that most immediately closes the flexibility gap. Users can add `## Commands`, `## Code Style`, `## Architecture`, or any other section to their `.brief.md` today, and they will appear in emitted output. No data model changes, no parser changes, no new dependencies.

Files changed: `src/emit/claude.rs`, `src/emit/prompt.rs`, `src/emit/agents_md.rs`, `src/emit/skill.rs`. Each gains a loop over `brief.unknown_sections` appending `## {heading}\n{content}\n\n`.

Prerequisite: fix the parser's unknown-section content capture to preserve raw Markdown instead of flattened text (see recommendation 2).

### 2. Preserve Raw Markdown in Unknown Sections

**Impact: High. Cost: Medium (parser refactor for unknown sections).**

Change `UnknownSection.content` from flattened plain text to the raw Markdown substring between the section's H2 heading and the next H2 or EOF. This requires passing the original input string into `parse_body` and tracking heading positions, or switching to a two-pass approach: first pass identifies H2 boundaries, second pass extracts known sections via pulldown-cmark and unknown sections via string slicing.

This makes recommendation 1 lossless. Without it, unknown sections lose code blocks, emphasis, links, sub-headings, and tables.

### 3. Add Commands to Frontmatter

**Impact: High. Cost: Low (frontmatter field + emitter updates).**

Add `commands: BTreeMap<String, String>` to `Frontmatter`. Update all emitters to format commands per-target. Update `brief init` to detect common commands from the project (e.g., if `Cargo.toml` exists, scaffold `build: cargo build`, `test: cargo test`). Update the validator to optionally check that command executables exist on PATH.

This is the single highest-value first-class addition because build/test commands appear in virtually every CLAUDE.md and benefit from structured representation.

### 4. Introduce the Emitter Trait

**Impact: Medium. Cost: Medium (refactor 5 emitters to trait implementations).**

Define the `Emitter` trait with per-section methods and a default `emit` orchestrator. Refactor existing emitters as trait implementations. This pays for itself the moment a sixth emitter is added (Copilot, Cursor, or Windsurf targets are on the roadmap) and reduces the cost of adding new first-class sections.

### 5. Add Style and Anti-Patterns as First-Class Sections

**Impact: Medium. Cost: Low (two new Section variants, parser branches, emitter blocks).**

These two categories appear in most production CLAUDE.md files and benefit from distinct emitter treatment. Style conventions should be emitted prominently as standing instructions. Anti-patterns should be emitted with strong negative framing. Mixing them into generic unknown sections loses that semantic distinction.

---

## Summary Table

| Recommendation | Impact | Cost | Prerequisite |
|---|---|---|---|
| 1. Emit unknown sections in text emitters | High | Low | #2 for lossless output |
| 2. Preserve raw Markdown in unknown sections | High | Medium | None |
| 3. Add commands to frontmatter | High | Low | None |
| 4. Introduce emitter trait | Medium | Medium | None |
| 5. First-class Style and Anti-Patterns sections | Medium | Low | None |

Recommendations 1-3 can be implemented independently and in parallel. Recommendation 4 is best done before adding new emit targets. Recommendation 5 is a small addition that can happen at any point.

The overarching principle: **promote to first-class only what benefits from structured parsing and target-specific reformatting. Everything else should flow through as raw Markdown via unknown sections that are actually emitted.**
