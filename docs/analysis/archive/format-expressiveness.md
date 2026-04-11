# Format Expressiveness Analysis: Brief vs. Full-Featured CLAUDE.md

**Date:** 2026-03-29
**Role:** Context Architect
**Scope:** Gap analysis, purpose clarification, section proposals, versioning strategy

---

## 1. Gap Analysis: Brief vs. CLAUDE.md

A full-featured CLAUDE.md (such as brief's own `CLAUDE.md`) typically contains nine categories of content. Here is how each maps to brief's current format.

### Natural Fits (content has a clear home)

**Project summary / architecture overview.** The H1 goal statement captures a single-sentence summary. The `context` frontmatter field points to architecture documents. But neither the goal nor the frontmatter can hold a multi-paragraph project description. The goal is a directive ("Fix the login bug"), not a description ("This is a web application that..."). A full CLAUDE.md often opens with two to three paragraphs describing what the project is and how its pieces fit together. Brief has no place for this. The `context` field says "read this file" but cannot inline the information the agent needs when those files do not exist yet.

**Constraints and dependency policies.** Hard/soft/ask-first constraints are brief's strongest section. Dependency policies ("Do NOT use heavy frameworks. No tokio. No reqwest.") map cleanly to hard constraints. This works well. The only awkwardness is that constraints are unscoped: a constraint like "WCAG 2.1 AA compliance" applies to UI components only, but brief's model treats it as global. This is a minor expressiveness gap, not a structural one.

**Sacred regions.** Directly supported with path-plus-reason syntax. This is one of brief's genuinely novel contributions.

**Assumptions.** Directly supported with checkbox syntax. Works as designed.

### Awkward Fits (content can be shoehorned, but poorly)

**Build, test, lint, format commands.** A real CLAUDE.md lists concrete commands: `cargo build`, `cargo test`, `cargo clippy`, `cargo fmt`. These are not constraints, not sacred regions, not assumptions, and not deliverables. They are operational instructions. Today, a brief author would stuff them into a hard constraint ("Always run cargo fmt before committing") or an unknown section (`## Commands`). Neither is satisfying. Constraints are behavioral intent, not command references. Unknown sections are preserved in the data model as `UnknownSection { heading, content }` but are invisible to all four emitters: `emit_claude`, `emit_prompt`, `emit_agents_md`, and `emit_json` all silently drop unknown sections. The JSON emitter serializes them, but the three prose emitters do not render them. This means a `## Commands` section in a brief file will disappear when emitted to CLAUDE.md, which is exactly the use case where commands matter most.

**Code style conventions.** Error handling patterns ("Use thiserror for library errors, anyhow for CLI error propagation"), naming conventions, import ordering, test expectations. These are partially expressible as soft constraints ("Prefer composition over inheritance") but a list of ten style rules crammed into the soft constraints section loses its identity. The items are not constraints in the sense of negotiable-vs-non-negotiable; they are standing conventions that always apply. They deserve their own semantic space.

**Behavioral instructions for the agent.** Tone, scope limitations, workflow preferences ("Parse into a strongly-typed Brief struct, then operate on that"). These blend with constraints but are categorically different. A constraint says "do not break backward compatibility." A behavioral instruction says "when you encounter an ambiguous requirement, ask before proceeding" or "prefer small, focused commits." These are meta-instructions about how the agent should work, not what the work product should look like.

### No Fit (content has nowhere to go)

**Project structure descriptions.** The directory tree in brief's own CLAUDE.md is structural context the agent needs for navigation. It is not a constraint, not a goal, not a deliverable. It does not fit in frontmatter (which is key-value, not prose). An unknown section could hold it, but unknown sections are not emitted. This is a hard gap.

**Environment and infrastructure context.** "Use PostgreSQL 16 with pgvector extension, deployed on GCP/k8s, Redis for caching." The `stack` frontmatter field captures technology names but not their configuration, relationships, or deployment topology. An agent that knows "PostgreSQL 16" but not "PostgreSQL 16 with read replicas behind pgbouncer" will produce code that works locally and fails in production.

**Testing philosophy and requirements.** "Tests for every parser edge case. Integration tests that round-trip: parse a fixture, emit, verify output contains expected content." These are not individual constraints but a testing worldview. They could be split into multiple soft constraints, but doing so fragments the author's intent and loses the coherence of "here is how we think about testing."

---

## 2. The Purpose Question

### Option A: Brief replaces CLAUDE.md

In this model, `brief emit claude` produces a complete, standalone CLAUDE.md. The `.brief.md` file becomes the single source of truth, and the CLAUDE.md is a generated artifact that should never be hand-edited.

**Arguments for:** Single source of truth is always simpler. No drift between brief and CLAUDE.md. Validation works end-to-end. `brief validate` can check everything because everything is in the brief. Cross-target emit from a single source is the original design promise.

**Arguments against:** This requires brief to capture everything a CLAUDE.md can express, which means the format must grow substantially. The 60-second authoring promise dies. A full-featured CLAUDE.md like brief's own is 150+ lines of nuanced prose. No structured format can make that fast to write without losing information. Brief would become a competing Markdown dialect rather than a lightweight overlay. Furthermore, CLAUDE.md is not the only target; the format also emits to AGENTS.md, system prompts, and JSON. Making the format rich enough for full CLAUDE.md replacement makes it over-specified for the other targets. Finally, teams that already have a well-maintained CLAUDE.md would need to port everything into brief syntax, which is migration friction with no clear benefit.

### Option B: Brief augments CLAUDE.md for specific tasks

In this model, a project has a hand-written CLAUDE.md for standing context (architecture, commands, style, structure) and uses `.brief.md` files for specific tasks, sprints, or agent sessions. Brief captures what is unique to this task: the goal, the constraints that differ from defaults, the sacred regions that matter now, the assumptions to validate. The CLAUDE.md provides the stable background; the brief provides the foreground.

**Arguments for:** Preserves the 60-second authoring target. Brief stays focused on its core insight: structured intent for a specific task. Teams keep their existing CLAUDE.md workflow. Brief files become composable task tickets, not monolithic project descriptions. Multiple brief files can coexist (one per task, one per sprint, one per agent role). This is how the tool is actually designed today: it emits a "section" for CLAUDE.md, not a complete replacement.

**Arguments against:** Two files means potential drift. An agent needs to reconcile the brief with the CLAUDE.md. The `brief emit claude` output is incomplete by design, which could confuse users who expect it to produce a working CLAUDE.md.

### Recommendation: Option B, with a bridge

Brief should augment, not replace. But it should grow just enough to cover the most common categories of context that users currently have no structured way to express. The key insight is that there are two kinds of content in a CLAUDE.md:

1. **Standing context** that rarely changes: project summary, architecture, commands, code style, project structure. This belongs in the CLAUDE.md itself (or in a base `.brief.md` that uses composition/inheritance, a Phase 3 feature).

2. **Task context** that changes per session: goal, constraints for this work, sacred regions to watch, assumptions to validate, deliverables. This is brief's core.

The bridge is a small number of new sections that straddle the two categories: content that is task-relevant but more structured than prose constraints. Specifically: commands, style conventions, and workflow instructions. These three categories are stable enough to live in a base brief but task-specific enough to vary between projects.

---

## 3. Section Additions

### 3.1. `## Commands` (recommended: first-class parsed section)

```markdown
## Commands

- **Build:** `cargo build`
- **Test:** `cargo test`
- **Lint:** `cargo clippy`
- **Format:** `cargo fmt`
- **Run:** `cargo run -- <subcommand>`
```

**What goes here:** Named commands the agent should know about and use. Build, test, lint, format, deploy, run, migrate. Each is a label-colon-command pair.

**Parsing:** Items follow a `**Label:** \`command\`` pattern. Parse into `Vec<Command>` where `Command { label: String, command: String }`. Freeform items without the pattern are preserved as-is.

**Emit mapping:**
- `claude` -- Renders as a "## Commands" section with the exact commands listed. This is the most directly useful mapping: CLAUDE.md files almost universally include a commands section.
- `prompt` -- Renders as `COMMANDS:` block with label-command pairs.
- `agents-md` -- Renders as "## Commands" section.
- `json` -- Serialized as `commands: [{ label, command }]`.

**Required or optional:** Optional. If absent, emitters skip it.

**Why first-class:** Commands are the single most universally present section in real CLAUDE.md files. Every project has build/test/lint commands. Agents use them constantly. Treating them as an unknown section means they vanish from emitted output. They also benefit from structured parsing: a `brief verify` command (Phase 2) could actually run these commands.

### 3.2. `## Style` (recommended: first-class parsed section)

```markdown
## Style

- Use `thiserror` for library errors, `anyhow` for CLI error propagation
- Parse into strongly-typed structs, then operate on those
- Tests for every parser edge case
- Integration tests that round-trip: parse -> emit -> verify
```

**What goes here:** Code conventions, patterns to follow, testing philosophy, naming conventions, import ordering. Anything that is a standing instruction about how code should look and behave.

**Parsing:** List items, parsed as `Vec<String>`. No sub-categorization needed in Phase 2. A future version could add H3 sub-sections (### Error Handling, ### Testing, ### Naming) but the flat list is sufficient and fast to author.

**Emit mapping:**
- `claude` -- Renders as "## Code Style" section.
- `prompt` -- Renders as `CODE STYLE:` block.
- `agents-md` -- Renders as "## Style Guide" section.
- `json` -- Serialized as `style: [string]`.

**Required or optional:** Optional.

**Why first-class:** Style conventions are the second most common section in CLAUDE.md files after commands. They are also the content most likely to be violated by agents that do not receive them. A hard constraint saying "Use thiserror for errors" is categorically different from a style convention saying "Use thiserror for errors" -- the first implies there is an alternative being rejected, the second says this is how we do things here. The semantic difference matters for emit: constraints get "NON-NEGOTIABLE" annotations, style items do not.

### 3.3. `## Context` (recommended: documented UnknownSection convention)

```markdown
## Context

This is a CLI tool that reads `.brief.md` files and emits them to
multiple agent runtimes. The parser extracts YAML frontmatter and
a Markdown heading tree. Emitters transform the parsed Brief struct
into target-specific formats.

### Project Structure
- `src/parse/` -- YAML frontmatter and Markdown body parsers
- `src/emit/` -- Target-specific emitters (claude, prompt, agents-md, json)
- `src/model.rs` -- Brief data structures
```

**What goes here:** Architecture overview, project structure, how things work. The background knowledge an agent needs before starting.

**Why UnknownSection, not first-class:** This content is inherently freeform prose. Parsing it into structured data provides no benefit -- the value is in passing it through to the agent verbatim. The real fix is making unknown sections emit correctly (they currently do not). Once unknown sections are rendered by all emitters, `## Context` as a convention works without any parser changes.

**What needs to change:** The three prose emitters (`emit_claude`, `emit_prompt`, `emit_agents_md`) must render unknown sections. This is a code change but not a data model change.

### 3.4. `## Workflow` (recommended: documented UnknownSection convention)

```markdown
## Workflow

- Make small, focused commits with descriptive messages
- Run tests after each significant change
- When encountering ambiguity, ask before proceeding
- Prefer surgical fixes over broad refactors unless told otherwise
```

**What goes here:** Meta-instructions about how the agent should work. Scope, approach, behavioral preferences.

**Why UnknownSection, not first-class:** Like Context, these are freeform instructions that do not benefit from structured parsing. The content is passed through to agents as-is. The convention name matters more than the data model.

### 3.5. `## Diagnosis` (recommended: documented UnknownSection convention)

```markdown
## Diagnosis

The login endpoint returns 500 on the third request after cache flush.
I suspect the session store is not being rehydrated after eviction.
Tried adding a cache warmup in the middleware but it did not help.
```

**What goes here:** Problem hypothesis, what has been tried, known failure modes. Particularly valuable for debugging tasks.

**Why UnknownSection:** Again, freeform prose. Structured parsing would not add value. The prior analysis document (context-architect.md) suggested this as a first-class section, but the actual content is always going to be unstructured narrative. What matters is that it reaches the agent, not that it is parsed into fields.

### 3.6. `## Phases` (recommended: out of scope for Phase 2)

Task decomposition and sequencing are genuinely complex features that require dependency tracking, ordering, and completion state. This is better addressed through brief composition (multiple brief files, one per phase) than through a single section with embedded sequencing. Defer to Phase 3.

### 3.7. `## Environment` (recommended: frontmatter extension)

```yaml
---
stack: [Python 3.12, PostgreSQL 16, Redis 7]
context: [./docs/architecture.md]
environment:
  services: [PostgreSQL 16 with pgvector, Redis 7, AWS SQS]
  config: [.env.example]
  infrastructure: GCP/k8s
---
```

**Why frontmatter:** Environment is structured key-value data, not prose. Services, config files, and infrastructure topology are machine-parseable. Frontmatter is the right place for structured metadata.

**Required or optional:** Optional.

---

## 4. Documentation vs. Code Changes Summary

| Gap | Recommendation | What Changes |
|-----|---------------|--------------|
| Build/test/lint commands | First-class `## Commands` section | `model.rs`: add `commands: Vec<Command>`. `body.rs`: parse Commands section. All emitters: render commands. `validate.rs`: optionally check commands exist. `init.rs`: detect commands from Makefile, package.json scripts, Cargo.toml. |
| Code style conventions | First-class `## Style` section | `model.rs`: add `style: Vec<String>`. `body.rs`: parse Style section. All emitters: render style. |
| Architecture/project description | Documented convention for `## Context` as UnknownSection | All prose emitters: render unknown sections. Document convention in CLAUDE.md spec. No model changes. |
| Agent behavioral instructions | Documented convention for `## Workflow` as UnknownSection | Same emitter fix as Context. Document convention. |
| Problem hypothesis | Documented convention for `## Diagnosis` as UnknownSection | Same emitter fix. Document convention. |
| Task decomposition | Out of scope | None. Defer to brief composition in Phase 3. |
| Environment/infrastructure | Frontmatter `environment` field | `model.rs`: add `environment` to Frontmatter. Emitters: render if present. |
| Project structure | Subsumed by `## Context` convention | No separate changes needed. |
| Testing philosophy | Subsumed by `## Style` section | No separate changes needed. |

The single most important code change enabling three of these recommendations is: **make unknown sections render in all emitters.** Today, `emit_claude`, `emit_prompt`, and `emit_agents_md` silently drop `unknown_sections`. Fixing this one behavior unlocks Context, Workflow, and Diagnosis without any parser or model changes.

---

## 5. The 60-Second Test

The original promise: author a `.brief.md` in 60 seconds. How do proposed additions affect this?

**Commands (first-class):** Adds 15-20 seconds if written from scratch. But `brief init` already detects stack -- it should also detect commands from Makefile, package.json `scripts`, Cargo.toml `[[bin]]` targets, and common patterns. With auto-detection, the Commands section is pre-populated and the author spends 5 seconds confirming or editing. Net impact: minimal.

**Style (first-class):** This is the one section that genuinely takes time to write well. A thorough style section might take 2-3 minutes. But it is optional, and most projects already have this content written somewhere (CLAUDE.md, CONTRIBUTING.md, .editorconfig). For a quick task brief, omitting Style is fine. For a project-level brief that serves as a base, the investment pays off. Net impact: optional, author chooses their time budget.

**Context, Workflow, Diagnosis (unknown section conventions):** These are write-what-you-need. For a simple bug fix, skip them. For a complex debugging session, a 30-second Diagnosis section dramatically improves agent effectiveness. They do not add to the minimum viable brief. Net impact: zero for quick tasks, proportional to complexity for complex tasks.

**Environment (frontmatter):** One additional YAML field, auto-detectable from docker-compose.yml, .env.example, and infrastructure configs. Net impact: 5 seconds if auto-detected, 15 seconds if manual.

The critical design principle: **every new section is optional, and the two first-class additions (Commands, Style) are auto-detectable by `brief init`.** The minimum viable brief remains: frontmatter with stack, an H1 goal, at least one constraint, and a deliverable. That still takes 60 seconds. The new sections are there for authors who need more expressiveness and are willing to invest more time.

---

## 6. Format Version Strategy

### Current State

The frontmatter has a `version` field defaulting to `"1"`. No version-specific parsing or validation logic exists -- the field is stored but never checked.

### Proposal

Do not bump the version for additive changes. The format should follow a compatibility model similar to HTML: new sections are simply ignored by older parsers (they already are, via `UnknownSection`), and new frontmatter fields are ignored by older deserializers (serde's `#[serde(default)]` already handles this).

**Version "1" continues to mean:** YAML frontmatter with defined fields, H1 goal, known H2 sections (Constraints, Sacred, Assumptions, Deliverable), unknown H2 sections preserved. Adding Commands and Style as known sections is backward-compatible: a version-1 parser that does not know about them simply treats them as unknown sections.

**When to bump to version "2":** Only when a breaking change requires it -- for example, if the constraint model changes from `Vec<String>` to `Vec<Constraint>` with structured fields, or if a currently-optional section becomes required, or if frontmatter field semantics change. None of the proposed additions require this.

**Practical implications:**
- `brief validate` should accept briefs with or without Commands/Style sections.
- `brief init` in a new version should scaffold the new sections, but old brief files without them remain valid.
- Emitters should gracefully handle missing optional sections (they already do for all current sections).
- The `version` field should be checked at parse time with a warning if it is unrecognized, but parsing should proceed anyway. This prevents hard failures when a newer brief is read by an older tool.

### Migration Path

Users upgrading `brief-cli` get the new sections in `brief init` output and new rendering in `brief emit`. Existing `.brief.md` files continue to work unchanged. Users who were already using `## Commands` or `## Style` as unknown sections get an automatic upgrade: their content moves from silent-drop territory to properly emitted output. This is a pure improvement with no breaking change.

---

## Summary of Recommendations

1. **Make unknown sections emit in all targets.** This is the highest-leverage single change. It unblocks three convention-based sections (Context, Workflow, Diagnosis) with minimal code changes.

2. **Add `## Commands` as a first-class parsed section.** It is the most universally needed content category, benefits from structured parsing (label-command pairs), and can be auto-detected by `brief init`.

3. **Add `## Style` as a first-class parsed section.** It is the second most common content category and has a meaningfully different semantic role from constraints.

4. **Add `environment` to frontmatter.** Structured infrastructure data belongs in YAML, not prose.

5. **Document conventions for Context, Workflow, and Diagnosis as recommended unknown section names.** Publish these in the format specification so authors discover them.

6. **Do not bump the format version.** All changes are additive and backward-compatible under version "1".

7. **Keep Phases out of scope.** Task decomposition is better solved through brief composition than section complexity.

Brief should remain a tool that is fast to write and structurally opinionated about the categories of information agents need. It should not try to be a complete replacement for a hand-written CLAUDE.md. The gap between the two is intentional: brief handles structured intent, CLAUDE.md handles freeform project knowledge, and `brief emit claude` produces a section that slots into the larger document.
