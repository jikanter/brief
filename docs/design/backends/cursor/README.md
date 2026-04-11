# Cursor Backend

**Status:** Planned (phase2-synthesis P4). The one P4 emitter explicitly flagged as "real work — meaningfully different format requiring a dedicated emitter."

## Target file format

`.cursor/rules/<name>.mdc` — a Markdown body with YAML frontmatter. The legacy `.cursorrules` single-file format (~12,000 character limit) still works but is superseded by `.mdc` rules.

## Frontmatter schema

```yaml
---
description: short human-readable rule summary
globs: ["src/**/*.ts"]
alwaysApply: false
---
```

## Activation modes

The combination of `alwaysApply` and `globs` produces four distinct activation modes:

| `alwaysApply` | `globs` set | Activation |
|---|---|---|
| `true` | — | Always present in the system prompt |
| `false` | yes | Applied only when editing files matching the globs |
| `false` | no | Model consults `description` and decides whether to load |
| — | — | Manual invocation via `@rule-name` in chat |

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
