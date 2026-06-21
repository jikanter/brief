# The `.brief.md` Format Specification

**Status:** Canonical. This is the authoritative specification for the `.brief.md`
format. Version **1**. Last updated 2026-06-12.

`.brief.md` is a structured briefing format for AI coding agents: Markdown with
YAML frontmatter and a defined heading convention. It is human-authored (target:
under 60 seconds to write) and machine-parsed into a strongly-typed `Brief` that
the `brief` CLI validates and emits to agent runtimes (CLAUDE.md, AGENTS.md,
Cursor, Copilot, Windsurf, Aider, raw prompt, XML, JSON, skills).

The format is deliberately **not** a DSL. It rides on Markdown so any developer
can author it with zero learning curve and any tool can render it. Structure
comes from headings and a small set of conventions, not from new syntax.

---

## 1. File shape

A `.brief.md` file has two parts, in order:

1. **YAML frontmatter** — a `---`-delimited block at the very top. Machine-
   critical structured data that does not read well as prose.
2. **Markdown body** — human-authored intent under a defined heading convention.

```markdown
---
stack: [Python 3.12, PostgreSQL 16]
context: [./docs/architecture.md]
---

# Redesign the event pipeline for 10M events/day

## Constraints

### Hard
- v2 API backward compatibility

### Soft
- Prefer async patterns

### Ask First
- Database schema changes

## Sacred
- `db/migrations/**` — Immutable migration history

## Assumptions
- [ ] Redis can handle the fan-out at peak
- [x] The existing REST API can coexist

## Deliverable
A pipeline sustaining 10M events/day with no v2 API breakage.
```

---

## 2. Frontmatter fields

| Field | Type | Required | Meaning |
|---|---|---|---|
| `stack` | `string[]` | **yes** | Technologies, languages, frameworks. |
| `context` | `string[]` | no | File paths or URLs of reference material. |
| `model` | `string` | no | Preferred model identifier. |
| `version` | `string` | no | Brief format version. Defaults to `"1"`. |
| `skill_name` | `string` | no | kebab-case name for an emitted Agent Skill. |
| `skill_description` | `string` | no | One-line description for an emitted Agent Skill. |

Unknown frontmatter keys are ignored (forward-compatible). New fields are added
only when they clear the YAGNI bar in
[design/frontmatter-additions.md](design/frontmatter-additions.md).

---

## 3. Body grammar

The body is parsed into a heading tree. The rules:

- **H1 (`#`) = the goal statement.** Exactly one is required. It is the task
  frame — what "done" is about.
- **H2 (`##`) = a top-level section.** Recognized sections (case-sensitive
  headings): `Constraints`, `Sacred`, `Assumptions`, `Deliverable`. Any other H2
  is an **unknown section** and is preserved verbatim (see §5).
- **H3 (`###`) under `## Constraints` = a constraint tier.** Recognized tiers:
  `Hard`, `Soft`, `Ask First`.
- **List items** under a heading are the content of that heading.

### 3.1 Constraints

Three tiers, each a bulleted list under an H3:

| Tier | Meaning |
|---|---|
| `### Hard` | Non-negotiable. Must hold. |
| `### Soft` | Preferred but flexible. |
| `### Ask First` | Requires human approval before proceeding. |

A constraint is a single list item of free text. The emit layer reframes each
tier by register at emit time (see §6) — authors write plain statements.

### 3.2 Sacred

A bulleted list under `## Sacred`. Each item names a protected path and the
reason it must not be modified, in the form:

```
- `<glob>` — <reason>
```

The path is backtick-wrapped; the separator is an em dash (`—`) or a double
hyphen (`--`); the rest is the reason. Malformed entries (missing backticks or
separator) are flagged by `brief validate`.

### 3.3 Assumptions

A bulleted list of Markdown checkboxes under `## Assumptions`:

```
- [ ] <unvalidated assumption>
- [x] <validated assumption>
```

`- [ ]` is unvalidated, `- [x]` is validated. Items without checkbox syntax are
a validation error.

### 3.4 Deliverable

Free-text under `## Deliverable` describing what "done" looks like. Not a list —
prose.

### 3.5 Identity (optional)

An optional leading H2 that names the project/agent identity, carried through to
emit. Recognized when present; its heading text and body are preserved.

---

## 4. Parsing rules summary

- Exactly one H1; it is the goal.
- Recognized H2 sections are parsed into typed fields; everything else is an
  unknown section.
- Recognized H3 tiers apply only under `## Constraints`.
- Sacred entries follow `` `path` — reason ``.
- Assumptions use `- [ ]` / `- [x]`.
- The parser is tolerant: unrecognized structure is preserved, not rejected.

---

## 5. Extensibility: unknown sections

Any H2 the parser does not recognize (e.g. `## Commands`, `## Code Style`,
`## Architecture`) is preserved **verbatim** and passed through to every emit
target. This is the format's primary extension mechanism: authors are never
blocked by the fixed vocabulary. Unknown sections are emitted as-is, without
register reframing.

---

## 6. The format vs. the emit layer

A load-bearing separation:

- **The `.brief.md` file is plain Markdown.** Authors write plain statements
  ("Use `Result<T, AppError>`", "Do not break the v2 API").
- **The emit layer is a compiler.** It reframes and reorders per target — e.g.
  the `claude`/`prompt` emitters apply RFC-2119 register (`NEVER:`/`MUST:`/
  `PREFER:`/`STOP`) and attention-ordering; the descriptive targets
  (`cursor`/`copilot`/`windsurf`/`aider`) keep a plain register. None of this
  lives in the source file.

Authoring stays simple; the compiler owns the prompt engineering. See
[analysis/phase2-synthesis.md](analysis/phase2-synthesis.md) (P0/P6) and
[analysis/emit-quality-refinements.md](analysis/emit-quality-refinements.md).

---

## 7. Conventions (settled, low-ceremony)

These are conventions, not parser primitives:

- **Prose-form constraints are legitimate.** A multi-sentence constraint that
  reads as a paragraph is valid; the emitter must not shatter authored prose into
  bullet-per-sentence. (A testing *worldview* is one constraint, not three.)

---

## 8. Format extensions

§8.1 is **shipped in version 1**. §8.2 is still proposed in
[open-questions.md](open-questions.md) and **not** implemented — do not author
against it yet.

### 8.1 Scoped constraints `[format]` — shipped

A constraint may apply only to files matching a path scope, rather than the whole
project. Authoring form: an optional leading bracket carries a comma-separated
list of glob patterns the constraint is scoped to.

```markdown
### Hard
- Must pass CI                                      ← global (no scope)
- [`src/ui/**`] WCAG 2.1 AA on all components        ← scoped
- [`src/api/**`, `src/lib/**`] Return `Result<T, AppError>`
```

**Authoring rule:** wrap any glob containing `*` in backticks (as shown). Bare
`**` is Markdown bold and gets mangled — backticks make the glob a code span that
survives parsing, the same convention sacred paths already use. A leading bracket
with no globs (`[]`) is treated as literal text, not a scope.

In the model a constraint is `Constraint { text, scope: Vec<Glob> }`; an empty
`scope` means project-wide (the historical behavior). The canonical store is the
glob list — each emitter serializes its own form.

Two distinct scoping mechanisms exist across target ecosystems and the model
serves both (see the research note in [open-questions.md](open-questions.md)
`[format]` Scoped Constraints):

1. **Glob-frontmatter scoping** — Cursor `globs`, Copilot `applyTo`, Windsurf
   `trigger: glob`, and Claude Code `.claude/rules/*.md` `paths:`. All are
   **file-level**: one glob set per rule file, so a scoped emit fans out one rule
   file per distinct scope.
2. **Directory-hierarchy scoping** — nested `CLAUDE.md` / `AGENTS.md`: a config
   file's location scopes it to that subtree.

A scope that is a **directory prefix** (`src/api/**`) has a native home in both
mechanisms; an **arbitrary glob** (`**/*.test.ts`) only has a home in the
glob-frontmatter mechanism. `Constraint::is_directory_prefix` exposes the
distinction so the emitter can pick hierarchy vs. glob-file vs. prose.

**Emit behavior (shipped):**

- **Native glob fan-out — `cursor`.** `brief emit cursor --install` writes the
  always-apply `brief.mdc` bundle (project-wide constraints only) plus one
  `brief-<slug>.mdc` per distinct scope, each carrying native `globs:`
  frontmatter and `alwaysApply: false`. The brief owns the `brief*.mdc`
  namespace; re-install sweeps stale scoped files. The non-install stdout form
  stays a single lossless bundle with scope shown inline.
- **Inline prose — all other emitters.** Targets with no native per-rule glob
  home keep scoped constraints non-destructive by prefixing the rendered line
  with `When working in `glob`, `glob`: …`. The `xml` target instead carries a
  structured `scope="…"` attribute on each `<rule>`. JSON emits the `scope`
  array (omitted when empty).
- **Validation.** `brief validate` warns (never errors) when a scope glob matches
  no files — dead weight that never activates. Directory-prefix scopes fall back
  to a directory-existence check since the `glob` crate does not expand a
  trailing `/**` to a directory's direct children.

*Deferred (tracked):* native fan-out for `copilot` (`applyTo`), `windsurf`
(`trigger: glob`), and Claude `.claude/rules/` `paths:` — the `cursor` emitter
proves the mechanism; the remaining glob-frontmatter targets reuse it when taken
up.

### 8.2 Behavioral instructions `[format]`

A possible reserved section (`## Workflow` or `## Behavior`) for "how the agent
should conduct itself" (e.g. "ask before proceeding on ambiguity", "prefer small
commits") as distinct from constraints on the work product. Today these live as
Soft constraints or unknown sections. Undecided.

---

## 9. Authoring checklist

- One H1 goal.
- `stack` in frontmatter.
- Constraints split into Hard / Soft / Ask First as appropriate.
- Sacred paths backtick-wrapped with a reason.
- Assumptions as checkboxes.
- Run `brief validate` — it flags missing goal/stack, malformed sacred entries,
  unchecked assumptions, missing context files, and vague constraints.

For the canonical project decisions behind this format, see
[design-decisions.md](design-decisions.md).
