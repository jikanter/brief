# Emit Targets: Cross-Ecosystem Reference

**Status:** Cross-ecosystem reference. Per-backend specifics **re-verified 2026-06-12** (see the per-backend folders).
**Details**: For specific details on individual target formats, read the [Specific Targets Backend Documentation](design/backends/README.md)
**Source:** Originally extracted 2026-04-11 from the Phase 2 Integration Engineer report; the load-bearing format facts have since been re-verified against current vendor docs (Cursor, Copilot/VS Code, Windsurf, Aider, Claude Code) on 2026-06-12.
**Scope:** Cross-ecosystem framing for the emit targets brief ships. All five core targets (claude, cursor, copilot, windsurf, aider) **have shipped** under [analysis/phase2-synthesis.md](analysis/phase2-synthesis.md) P4; this document is the high-level cross-cutting view, while the per-backend folders carry the verified technical specifics.

> **Verification note.** The five core backends' format facts were re-verified 2026-06-12 (this corrected the earlier array-form `globs`, confirmed file-level scoping everywhere, and surfaced Claude Code's `.claude/rules` `paths:` glob surface). The `aichat/` backend is still audit-dated. Re-check vendor docs before acting on any specific config key — ecosystems move fast.

---

## Per-backend documentation

Each backend has its own design folder under [design/backends/](design/backends/) with file formats, frontmatter schemas, activation modes, character limits, mapping from brief's three-tier taxonomy, and per-backend open questions:

| Backend | Folder | One-line summary |
|---|---|---|
| Claude Code | [design/backends/claude/](design/backends/claude/) | Shipped. CLAUDE.md, skills, hooks; nested CLAUDE.md + `.claude/rules` `paths:` = native scoping. |
| Cursor | [design/backends/cursor/](design/backends/cursor/) | Shipped. `.cursor/rules/*.mdc`; `globs` is a comma-string, file-level. |
| GitHub Copilot | [design/backends/copilot/](design/backends/copilot/) | Shipped. `.github/copilot-instructions.md`; `applyTo` comma-string, file-level. |
| Windsurf | [design/backends/windsurf/](design/backends/windsurf/) | Shipped. `.windsurf/rules/*.md`; `trigger` taxonomy verified; `.devin/` migration. |
| Aider | [design/backends/aider/](design/backends/aider/) | Shipped. Two-file: `CONVENTIONS.md` + `.aider.conf.yml`. No native scoping. |

---

## Cross-Ecosystem Divergence Matrix

The single most reusable cross-backend artifact. A compact catalog of the axes along which no single unified emit format is possible. This is the design rationale for any future discussion of per-target overrides, constraint scoping, or `emit:` frontmatter hint maps. *(Verified 2026-06-12.)*

| Axis | Claude Code | Cursor | Copilot | Windsurf | Aider |
|---|---|---|---|---|---|
| File-scoped rules | **Yes** (`.claude/rules` `paths:`) + nested CLAUDE.md | Yes (`globs`) | Yes (`applyTo`) | Yes (`globs`) | No |
| Glob serialization | `paths:` YAML list | comma-string | comma-string | scalar | n/a |
| One glob set per file | yes (per rule file) | yes | yes | yes | n/a |
| Multi-file vs single | CLAUDE.md (+ skills, + `.claude/rules/*`) | Multi (`.cursor/rules/*.mdc`) | Either | Multi (`.windsurf/rules/*.md`) | Two-file (conventions + config) |
| Activation modes | Always-on + hooks + `paths`-scoped rules | 4 modes | 2 modes (global + glob) | 4 modes | Manual / auto-load |
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
