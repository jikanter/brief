# Emit Backends

Per-backend integration design documentation. Each subfolder captures one emit target's file formats, frontmatter schemas, activation modes, character limits, and any open questions about how brief should integrate with that ecosystem.

These docs are design references for each ecosystem. The five core targets (claude, cursor, copilot, windsurf, aider) have **shipped** under phase2-synthesis P4; the per-backend folders now carry verified format facts (re-verified 2026-06-12). See [phase2-synthesis.md](../../analysis/phase2-synthesis.md) for the roadmap and [../../emit-targets-reference.md](../../emit-targets-reference.md) for the cross-ecosystem framing.

## Backends

| Backend | Folder | Status |
|---|---|---|
| Claude Code | [claude/](claude/) | Shipped — CLAUDE.md, skills, hooks (P2). `--position`/`--uninstall`/`--full` and skill scaffold/install in flight (P5/P7). |
| Cursor | [cursor/](cursor/) | Shipped (P4) — single bundled `.mdc` rule (no `globs` yet, pending scoped constraints). |
| GitHub Copilot | [copilot/](copilot/) | Shipped (P4) — single `.github/copilot-instructions.md`. |
| Windsurf | [windsurf/](windsurf/) | Shipped (P4) — single `always_on` workspace rule. |
| Aider | [aider/](aider/) | Shipped (P4) — two-file emit (CONVENTIONS.md + .aider.conf.yml). |
| aichat / eridian-ai | [aichat/](aichat/) | Proposed (off-roadmap). Agents-only target — multi-file directory emit + registry append. |

## Cross-Ecosystem Divergence Matrix

The most reusable cross-backend artifact. A compact catalog of the axes along which no single unified emit format is possible. This is the design rationale for any future discussion of per-target overrides, constraint scoping, or `emit:` frontmatter hint maps.

| Axis | Claude Code | Cursor | Copilot | Windsurf | Aider |
|---|---|---|---|---|---|
| File-scoped rules | **Yes** (`.claude/rules` `paths:`) + nested CLAUDE.md | Yes (`globs`) | Yes (`applyTo`) | Yes (`globs`) | No |
| Glob form | `paths:` YAML list | comma-string | comma-string | scalar | n/a |
| Multi-file vs single | Single CLAUDE.md (+ skills, + `.claude/rules/*`) | Multi (`.cursor/rules/*.mdc`) | Either | Multi (`.windsurf/rules/*.md`) | Two-file (conventions + config) |
| Activation modes | Always-on + hooks + `paths`-scoped rules | 4 modes | 2 modes (global + glob) | 4 modes | Manual / auto-load |
| Operational config | `.claude/settings.json` | Inline in `.mdc` | None | Inline in frontmatter | `.aider.conf.yml` |
| Commands/workflows | Skills | N/A | N/A | N/A | `/read` commands |
| Idiomatic register | Imperative (NEVER/MUST) | Descriptive | Descriptive | Descriptive | Conversational |

**Implication 1.** Brief's current uniform NEVER/MUST/PREFER/STOP imperative register (phase2-synthesis P0) is correct for Claude Code but potentially mis-tuned for Aider and Copilot, where the idiomatic style is more descriptive. See [analysis/emit-quality-refinements.md](../../analysis/emit-quality-refinements.md) §3 (per-target tone adaptation).

**Implication 2.** Any attempt to unify these axes under a single brief format will leak in one of two directions: either the brief format grows a glob-scoping concept (a real format change — see [open-questions.md](../../open-questions.md) `[format]` scoped constraints), or the emit layer grows per-target hint infrastructure (an `emit:` frontmatter map — see emit-quality-refinements §5).

## Verification posture

The five core backends (claude, cursor, copilot, windsurf, aider) had their load-bearing format facts **re-verified against current vendor docs on 2026-06-12** — `globs`/`applyTo` serialization, activation modes, file-level scoping, and the Claude `.claude/rules` `paths:` surface. The `aichat/` backend remains audit-dated (2026-04-27). Third-party ecosystems still move fast — re-check before acting on any specific config key, file path, or character limit; remaining unverified items are flagged inline as `[open-question]`.

## Action checklist when implementing a new backend

For anyone sitting down to build a P4 emitter, this is the minimal pass before writing code:

1. **Confirm the target file path and frontmatter schema** against current ecosystem docs. Most `[open-question]` items in these docs are frontmatter fields and character limits — the most likely things to have changed.
2. **Decide on scope.** Trivial base case (single file, no globs) vs. full integration (two-file aider, per-glob cursor/copilot, hierarchical claude) changes effort by 3–5×.
3. **Match the idiomatic register.** The NEVER/MUST reframing is Claude-specific tuning. For Aider and Copilot, render Soft constraints as plain bulleted preferences rather than `PREFER:` markers.
4. **Enforce target-specific limits.** Character limits and rule-file size guidance should fail the emit or warn — never silently truncate.
