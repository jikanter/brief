# Emit Targets: Cross-Ecosystem Reference

**Status:** Forward-looking reference, partially unverified.
**Details**: For specific details on individual target formats, read the [Specific Targets Backend Documentation](design/backends/README.md)
**Source:** Extracted 2026-04-11 from [analysis/archive/emit-integration-audit.md](analysis/archive/emit-integration-audit.md), the Phase 2 Integration Engineer report.
**Scope:** Cross-ecosystem framing for every emit target brief either ships today or plans to ship under [analysis/phase2-synthesis.md](analysis/phase2-synthesis.md) P4. The live roadmap treats copilot/windsurf/aider as "trivial wrappers" and cursor as "real work" — this document is the high-level reason why; the per-backend folders carry the technical specifics.

> **Verification note.** Most claims in the per-backend docs are dated to the March 2026 audit. Third-party ecosystems evolve quickly. Before acting on any specific config key, file path, or character limit, cross-check against the target ecosystem's current documentation. Items the audit could not verify are flagged inline as `[open-question]` in the per-backend docs.

---

## Per-backend documentation

Each backend has its own design folder under [design/backends/](design/backends/) with file formats, frontmatter schemas, activation modes, character limits, mapping from brief's three-tier taxonomy, and per-backend open questions:

| Backend | Folder | One-line summary |
|---|---|---|
| Claude Code | [design/backends/claude/](design/backends/claude/) | Shipped. CLAUDE.md, skills, hooks, monorepo + handler-type questions outstanding. |
| Cursor | [design/backends/cursor/](design/backends/cursor/) | Real work. `.cursor/rules/*.mdc` with frontmatter + 4 activation modes. |
| GitHub Copilot | [design/backends/copilot/](design/backends/copilot/) | Trivial base case. Path-scoped variant requires brief format growth. |
| Windsurf | [design/backends/windsurf/](design/backends/windsurf/) | Trivial base case. Several details unverified. |
| Aider | [design/backends/aider/](design/backends/aider/) | Two-file emit: `CONVENTIONS.md` + `.aider.conf.yml`. Conversational register. |

---

## Cross-Ecosystem Divergence Matrix

The single most reusable cross-backend artifact. A compact catalog of the axes along which no single unified emit format is possible. This is the design rationale for any future discussion of per-target overrides, constraint scoping, or `emit:` frontmatter hint maps.

| Axis | Claude Code | Cursor | Copilot | Windsurf | Aider |
|---|---|---|---|---|---|
| File-scoped rules | No (hooks only) | Yes (`globs`) | Yes (`applyTo`) | Yes (`globs`) | No |
| Multi-file vs single | Single CLAUDE.md (+ skills) | Multi (`.cursor/rules/*.mdc`) | Either | Multi (`.windsurf/rules/*.md`) | Two-file (conventions + config) |
| Activation modes | Always-on + hooks | 4 modes | 2 modes (global + glob) | 4 modes [unverified] | Manual / auto-load |
| Operational config | `.claude/settings.json` | Inline in `.mdc` | None | Inline in frontmatter | `.aider.conf.yml` |
| Commands/workflows | Skills | N/A | N/A | N/A | `/read` commands |
| Idiomatic register | Imperative (NEVER/MUST) | Descriptive | Descriptive | Descriptive | Conversational |

**Implication 1.** Brief's current uniform NEVER/MUST/PREFER/STOP imperative register (phase2-synthesis P0) is correct for Claude Code but mis-tuned for Aider, Copilot, and arguably Cursor and Windsurf, where the idiomatic style is more descriptive. See [analysis/emit-quality-refinements.md](analysis/emit-quality-refinements.md) §3 (per-target tone adaptation).

**Implication 2.** Any attempt to unify these axes under a single brief format will leak in one of two directions: either the brief format grows a glob-scoping concept (a real format change — see [open-questions.md](open-questions.md) `[format]` scoped constraints), or the emit layer grows per-target hint infrastructure (an `emit:` frontmatter map — see emit-quality-refinements §5).

---

## Action checklist when implementing a new backend

For anyone sitting down to build a P4 emitter, this is the minimal pass before writing code:

1. **Read the per-backend folder.** All target-specific specifics (file paths, frontmatter schemas, character limits, activation modes) live there.
2. **Confirm against current ecosystem docs.** Most `[open-question]` items in the per-backend docs are frontmatter fields and character limits — the most likely things to have changed since the March 2026 audit.
3. **Decide on scope.** Trivial base case (single file, no globs) vs. full integration (two-file aider, per-glob cursor/copilot, hierarchical claude) changes effort by 3–5×.
4. **Match the idiomatic register.** The NEVER/MUST reframing is Claude-specific tuning. For Aider and Copilot, render Soft constraints as plain bulleted preferences rather than `PREFER:` markers.
5. **Enforce target-specific limits.** Character limits and rule-file size guidance should fail the emit or warn — never silently truncate.
