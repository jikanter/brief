# Cursor Backend

**Status:** Shipped (phase2-synthesis P4) — `brief emit cursor [--install]` is implemented in `src/emit/cursor.rs`, currently emitting a single bundled rule with `alwaysApply: true` (no `globs`, pending scoped constraints). Was the one P4 emitter flagged as "real work — meaningfully different format requiring a dedicated emitter."

**Format facts verified against [cursor.com/docs/context/rules](https://cursor.com/docs/context/rules) on 2026-06-12.** The 2026-03 audit's array-form `globs` was wrong — see "Glob field format" below.

## Target file format

`.cursor/rules/<name>.mdc` — a Markdown body with YAML frontmatter. The legacy `.cursorrules` single-file format (~12,000 character limit) still works but is superseded by `.mdc` rules. Rule files can be organized in nested folders under `.cursor/rules/` (e.g. `.cursor/rules/frontend/components.mdc`) — but subfolders are organizational only; they do **not** auto-scope a rule to that directory (scoping is via `globs`, not location).

## Frontmatter schema

```yaml
---
description: short human-readable rule summary
globs: "src/**/*.ts, src/**/*.tsx"
alwaysApply: false
---
```

### Glob field format (corrected 2026-06-12)

`globs` is a **comma-separated string**, NOT a YAML array. Multiple patterns are joined with commas within one string value: `globs: "docs/**/*.md, docs/**/*.mdx"`. (The earlier `globs: ["src/**/*.ts"]` array form in this doc was from the March 2026 audit and is incorrect for current Cursor — the emitter must serialize a comma-joined string.)

### One glob set per file

A single `.mdc` file carries **one `globs` set covering the whole file**. There is no way to scope different rules to different globs within one file — to do that you write **separate `.mdc` files**, one per scope. This is the forcing function behind brief's scoped-constraint emit (split-by-scope file fan-out).

## Activation modes

The combination of `alwaysApply` and `globs` produces four activation modes (current Cursor names in parentheses):

| `alwaysApply` | `globs`/`description` | Activation |
|---|---|---|
| `true` | — | **Always** — present in every session (`globs` parsed but ignored) |
| `false` | `globs` set | **Apply to Specific Files** (Auto Attached) — loaded when editing matching files |
| `false` | `description` only | **Apply Intelligently** (Agent Requested) — model consults `description` and decides |
| `false` | neither | **Apply Manually** — only via `@rule-name` in chat |

## Soft size guidance

Best-practice guidance is to keep individual rule files under ~500 lines.

[open-question] Is the 500-line guidance a hard limit enforced by Cursor, or community convention? The original audit cited it as best-practice. The answer determines whether the brief emitter should fail/warn/ignore when output exceeds it.

## Why this is "real work" in P4

Three concrete reasons the cursor emitter is not a trivial wrapper:

1. **Frontmatter translation.** Cursor's frontmatter schema is meaningfully different from brief's (`description`/`globs`/`alwaysApply` vs. brief's `stack`/`context`/`version`). The emitter has to construct it from scratch rather than pass-through.
2. **No native scoping in brief.** Cursor's main feature is per-rule glob scoping. Brief's current flat constraint model has no concept of scope, so every emitted rule defaults to `alwaysApply: true` until brief itself learns about scoped constraints. This throws away most of what makes Cursor's rule format useful.
3. **Multi-file emit.** Idiomatically, a project would have several `.mdc` files clustered by purpose (e.g. `auth-rules.mdc`, `ui-rules.mdc`, `test-conventions.mdc`). Brief's single-source model has to decide whether to bundle everything into one rule file or split by some axis.

## Mapping brief's three-tier taxonomy to Cursor activation modes

A reasonable mapping if/when scoped constraints exist in brief:

| Brief tier | Cursor activation |
|---|---|
| `### Hard` (sacred + non-negotiable) | `alwaysApply: true` |
| `### Soft` with a glob hint | `alwaysApply: false` + `globs` |
| `### Ask First` | `alwaysApply: false`, no globs (model-decision via description) |

[open-question] Without scoped constraints in brief, should the cursor emitter produce one bundled `alwaysApply: true` rule, or split by tier (one rule per Hard/Soft/Ask First)? Splitting by tier is probably closer to idiomatic Cursor usage but loses the connection between related constraints across tiers.

[open-question] Should brief add format-level scoped constraints to make the cursor emitter idiomatic? See [open-questions.md](../../../open-questions.md) `[format]` scoped constraints for the broader format-level question. The cursor backend is the strongest forcing function for resolving that question.

## Connection to other docs

- Format-level scoping question: [open-questions.md](../../../open-questions.md) `[format]` scoped constraints
- Per-target hint mechanism that could provide cursor-specific emit metadata: [emit-quality-refinements.md](../../../analysis/emit-quality-refinements.md) §5 (`emit:` frontmatter map)
