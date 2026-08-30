# Cursor Backend

**Status:** Shipped — `brief emit cursor [--install] [--hooks]`. Format facts re-verified against [cursor.com/docs/context/rules](https://cursor.com/docs/context/rules) and [cursor.com/docs/hooks](https://cursor.com/docs/hooks) on 2026-08-30.

The emitter is not a CLAUDE.md wrapper. Cursor's project rules are `.mdc` files with their own frontmatter (`description`, `globs`, `alwaysApply`) and four activation modes. `--install` maps brief's constraint model onto those modes instead of dumping everything into one always-on rule.

## Target file format

`.cursor/rules/<name>.mdc` — Markdown body with YAML frontmatter. A plain `.md` file in `.cursor/rules` is ignored (no frontmatter). Nested folders under `.cursor/rules/` are organizational only; scoping is via `globs`, not location.

`AGENTS.md` is a separate emit target (`brief emit agents-md`), not part of this backend.

## Frontmatter schema

```yaml
---
description: short human-readable rule summary
globs: src/**/*.ts, src/**/*.tsx
alwaysApply: false
---
```

### Glob field format

`globs` is a **comma-separated string**, not a YAML array. Multiple patterns are joined with commas: `globs: docs/**/*.md, docs/**/*.mdx`. The emitter quotes the value only when YAML itself would misparse it (colon, `#`, quotes, …); glob metacharacters (`*`, `?`) stay unquoted so the documented form is preserved.

### One glob set per file

A single `.mdc` file carries **one `globs` set covering the whole file**. Different scopes become separate files — that is why `--install` fans out.

## Activation modes

| `alwaysApply` | `globs` / `description` | Activation |
|---|---|---|
| `true` | — | **Always** — present in every session (`globs` ignored) |
| `false` | `globs` set | **Apply to Specific Files** — loaded when matching files are in context |
| `false` | `description` only | **Apply Intelligently** — model consults `description` and decides |
| `false` | neither | **Apply Manually** — only via `@rule-name` |

## What `brief emit cursor` produces

**Stdout** (`brief emit cursor`) is a single lossless `alwaysApply: true` rule so piping stays complete: scoped constraints keep an inline `When working in …:` prefix; Ask First stays in the body.

**`--install`** writes into `.cursor/rules/` (directory created if missing). Brief owns the `brief*.mdc` namespace: prior `brief-*.mdc` files are swept, then rewritten. Hand-written rules without the `brief-` prefix are never touched.

| File | Activation | Contents |
|---|---|---|
| `brief.mdc` | Always | Goal, stack, `@context` includes, identity, unscoped Hard/Soft, sacred regions (with preamble), unvalidated assumptions, deliverable, unknown sections |
| `brief-ask-first.mdc` | Apply Intelligently | Unscoped Ask First. `alwaysApply: false`, description `Ask before proceeding: <goal>`, no `globs` |
| `brief-<slug>.mdc` | Auto Attached | One file per distinct constraint scope; `globs:` is the comma-joined scope set |

Register is **descriptive** (`## Required` / `## Preferred`), not Claude's `**IMPORTANT:**` / NEVER/MUST.

Context paths emit as Cursor `@path` includes (leading `./` stripped) so matching files can be pulled into the rule.

## Sacred-region hooks

`brief emit cursor --install --hooks` registers an idempotent project hook in `.cursor/hooks.json`:

```json
{
  "version": 1,
  "hooks": {
    "preToolUse": [
      { "command": "brief check --hook", "matcher": "Write" }
    ]
  }
}
```

`brief check --hook` detects Cursor events (`cursor_version` or `hook_event_name: preToolUse`) and denies with Cursor's `{ "permission": "deny", "user_message", "agent_message" }` payload. Claude Code's `hookSpecificOutput` protocol is unchanged for `PreToolUse` events.

Other entries in `hooks.json` are preserved.

## Soft size guidance

Keep individual rule files under ~500 lines (Cursor docs; not a parser cap). Brief warns when cursor emit exceeds 500 lines and never truncates.

## Mapping brief's taxonomy to Cursor activation

| Brief | Cursor activation |
|---|---|
| Unscoped `### Hard` + sacred + standing project context | `alwaysApply: true` (`brief.mdc`) |
| Scoped constraint (any tier) | `alwaysApply: false` + `globs:` (`brief-<slug>.mdc`) |
| Unscoped `### Ask First` | `alwaysApply: false`, description only (`brief-ask-first.mdc`) |

Unscoped Soft stays in the always-apply bundle so preferences are not left to chance under Apply Intelligently.

## Connection to other docs

- Format-level scoping: [open-questions.md](../../../open-questions.md) `[format]` scoped constraints (DECIDED, P8)
- Per-target tone: [emit-quality-refinements.md](../../../analysis/emit-quality-refinements.md) §3
