# Emit Backends

Per-backend integration design documentation. Each subfolder captures one emit target's file formats, frontmatter schemas, activation modes, character limits, and any open questions about how brief should integrate with that ecosystem.

These docs are **forward-looking design references**. Most backends except `claude/` are not yet implemented — see [phase2-synthesis.md](../../analysis/phase2-synthesis.md) §P4 for the implementation roadmap.

## Backends

| Backend | Folder | Status |
|---|---|---|
| Claude Code | [claude/](claude/) | Shipped (CLAUDE.md, skills, --install). P2 hooks and P5 install enhancements pending. |
| Cursor | [cursor/](cursor/) | Planned (P4). Real work — meaningfully different format. |
| GitHub Copilot | [copilot/](copilot/) | Planned (P4). Trivial wrapper for the base case. |
| Windsurf | [windsurf/](windsurf/) | Planned (P4). Trivial wrapper for the base case. |
| Aider | [aider/](aider/) | Planned (P4). Two-file emit (CONVENTIONS.md + .aider.conf.yml). |
| aichat / eridian-ai | [aichat/](aichat/) | Proposed (off-roadmap). Agents-only target — multi-file directory emit + registry append. |

## Cross-Ecosystem Divergence Matrix

The most reusable cross-backend artifact. A compact catalog of the axes along which no single unified emit format is possible. This is the design rationale for any future discussion of per-target overrides, constraint scoping, or `emit:` frontmatter hint maps.

| Axis | Claude Code | Cursor | Copilot | Windsurf | Aider |
|---|---|---|---|---|---|
| File-scoped rules | No (hooks only) | Yes (`globs`) | Yes (`applyTo`) | Yes (`globs`) | No |
| Multi-file vs single | Single CLAUDE.md (+ skills) | Multi (`.cursor/rules/*.mdc`) | Either | Multi (`.windsurf/rules/*.md`) | Two-file (conventions + config) |
| Activation modes | Always-on + hooks | 4 modes | 2 modes (global + glob) | 4 modes [unverified] | Manual / auto-load |
| Operational config | `.claude/settings.json` | Inline in `.mdc` | None | Inline in frontmatter | `.aider.conf.yml` |
| Commands/workflows | Skills | N/A | N/A | N/A | `/read` commands |
| Idiomatic register | Imperative (NEVER/MUST) | Descriptive | Descriptive | Descriptive | Conversational |

**Implication 1.** Brief's current uniform NEVER/MUST/PREFER/STOP imperative register (phase2-synthesis P0) is correct for Claude Code but potentially mis-tuned for Aider and Copilot, where the idiomatic style is more descriptive. See [analysis/emit-quality-refinements.md](../../analysis/emit-quality-refinements.md) §3 (per-target tone adaptation).

**Implication 2.** Any attempt to unify these axes under a single brief format will leak in one of two directions: either the brief format grows a glob-scoping concept (a real format change — see [open-questions.md](../../open-questions.md) `[format]` scoped constraints), or the emit layer grows per-target hint infrastructure (an `emit:` frontmatter map — see emit-quality-refinements §5).

## Verification posture

Most claims in these docs are dated to the March 2026 audit that produced them. Third-party ecosystems evolve quickly. Before acting on any specific config key, file path, or character limit, cross-check against the target ecosystem's current documentation. Items the original audit could not verify are flagged inline as `[open-question]`.

## Action checklist when implementing a new backend

For anyone sitting down to build a P4 emitter, this is the minimal pass before writing code:

1. **Confirm the target file path and frontmatter schema** against current ecosystem docs. Most `[open-question]` items in these docs are frontmatter fields and character limits — the most likely things to have changed.
2. **Decide on scope.** Trivial base case (single file, no globs) vs. full integration (two-file aider, per-glob cursor/copilot, hierarchical claude) changes effort by 3–5×.
3. **Match the idiomatic register.** The NEVER/MUST reframing is Claude-specific tuning. For Aider and Copilot, render Soft constraints as plain bulleted preferences rather than `PREFER:` markers.
4. **Enforce target-specific limits.** Character limits and rule-file size guidance should fail the emit or warn — never silently truncate.
