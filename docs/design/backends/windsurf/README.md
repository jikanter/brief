# Windsurf Backend

**Status:** Planned (phase2-synthesis P4). Trivial wrapper for the base case. Several details from the original audit are unverified — flagged inline as `[open-question]`.

## Target file locations

| File | Scope |
|---|---|
| `.windsurf/rules/<name>.md` | Workspace-level rules (per project) |
| `~/.codeium/windsurf/memories/global_rules.md` | Global rules (per user) |

[open-question] Are these still the canonical paths in current Windsurf? The audit was dated March 2026; the path under `~/.codeium/` in particular has historically moved as Windsurf has evolved.

## Frontmatter schema (unverified)

The audit reported that workspace rule files carry YAML frontmatter with a `trigger` field selecting one of four activation modes:

```yaml
---
trigger: always_on   # or: model_decision | glob | manual
globs: ["src/**/*.rs"]
---
```

| `trigger` value | Activation |
|---|---|
| `always_on` | Always present in the system prompt |
| `model_decision` | Model decides when to load based on description / context |
| `glob` | Loaded when editing files matching `globs` |
| `manual` | Only loaded by explicit user invocation |

[open-question] Is this trigger taxonomy accurate for current Windsurf? Audit-cited but unverified. The four-mode structure mirrors Cursor closely, which makes it plausible but not confirmed.

## Character limits (unverified)

The audit cited two limits:
- 12,000 characters per workspace rule file
- 6,000 characters per global rule file

[open-question] Are the 12,000 / 6,000 character limits accurate for current Windsurf? These should be enforced by the brief emitter (fail or warn on overrun, never silently truncate) — but only after verification.

## Why this is "trivial" in P4

The base case is a single `always_on` workspace rule file rendered as plain Markdown — equivalent to the claude emit output minus the `<brief:generated>` marker envelope. No frontmatter translation, no multi-file split, no register translation needed beyond toning down the imperative voice.

The "trivial" characterization holds **only if** the typical brief output stays under the cited character limits. If briefs routinely exceed them, the emitter has to either fail loudly or split across multiple rule files, which is no longer trivial.

[open-question] What happens if a brief's emit exceeds Windsurf's per-file character limit? Options: (a) fail with an error and ask the user to reduce, (b) truncate with a warning, (c) split into multiple files automatically, (d) emit to global_rules.md instead. Each has trade-offs; no obvious right answer.

## Mapping brief tiers to Windsurf triggers

| Brief tier | Windsurf trigger |
|---|---|
| `### Hard` + sacred regions | `always_on` |
| `### Soft` with glob hint | `glob` (if scoped constraints exist) |
| `### Soft` without scope | `always_on` or `model_decision` |
| `### Ask First` | `manual` |

[open-question] Without scoped constraints in brief, should the windsurf emitter bundle everything into one `always_on` rule, or split by tier? Same trade-off as the cursor backend — splitting is more idiomatic but loses cross-tier coherence.

## Idiomatic register

Like Cursor and Copilot, Windsurf conventions trend descriptive rather than imperative. Render Soft constraints as plain bulleted preferences. See [emit-quality-refinements.md](../../../analysis/emit-quality-refinements.md) §3 for per-target tone adaptation.

## Action checklist for implementation

1. **Verify everything in this doc** against current Windsurf documentation. Most of the per-backend specifics are `[open-question]`.
2. Confirm character limits and decide overflow behavior before writing the emitter.
3. Decide whether to support `global_rules.md` emit at all, or only workspace-level files.
