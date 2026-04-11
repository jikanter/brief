# Obsolete and Rejected Features

**Original synthesis:** 2026-03-29 (revised 2026-03-30)
**Archived:** 2026-04-11
**Active roadmap:** [../phase2-synthesis.md](../phase2-synthesis.md)

Historical record of proposals that emerged from the Phase 2 synthesis and were
then shipped, removed, or downgraded. Kept for provenance and so that future
readers can see what was considered and why it was rejected. This document is
**not** current guidance — for the active Phase 2 roadmap, see the link above.

Original input reports (all in this archive):
[flexibility-gap.md](./flexibility-gap.md),
[format-expressiveness.md](./format-expressiveness.md),
[emit-integration-audit.md](./emit-integration-audit.md),
[devops-engineer.md](./devops-engineer.md).

---

## Completed (Post-Tier 0)

The following items from the original synthesis have been **shipped** and are
no longer roadmap items:

- Unknown sections emitted in all text emitters (claude, prompt, agents_md, skill) — see [demos/2026-03-29-unknown-sections.md](../../demos/2026-03-29-unknown-sections.md)
- Raw Markdown preserved in unknown sections
- Context files emitted as `@` references in Claude target
- Hard constraints get `**IMPORTANT:**` prefix in Claude target
- `brief emit skill` fully implemented with YAML frontmatter, rules/preferences, verification steps
- `--install` flag for `brief emit claude` — idempotent injection into CLAUDE.md. Originally shipped with HTML-comment markers (`<!-- brief:start -->` / `<!-- brief:end -->`), later migrated to `<brief:generated>` / `</brief:generated>` XML-style markers because Claude Code strips HTML comments before the model reads CLAUDE.md. Legacy markers are still recognized on read for migration.
- `--install` flag for `brief emit skill` — writes to `.claude/skills/<name>/SKILL.md`
- `brief diff` implemented for semantic comparison between briefing files

With Tier 0 complete and `--install` working, the project's priorities shifted
significantly. The original tier ordering optimized for structural completeness
of emitted documents. The revised ordering (captured in the active phase 2
roadmap) optimizes for **agent behavioral compliance** and **enforcement
capability**.

---

## Items Removed (Technically Wrong or Misguided)

### 1. `permissions.deny` from sacred regions — REMOVED

The original synthesis proposed generating `permissions.deny` rules from sacred entries. This is the wrong abstraction. Claude Code's `permissions.allow` and `permissions.deny` are lists of tool-call patterns (`Bash(cargo build:*)`, `Edit(/path/to/file.rs)`), not file-path ACLs. You cannot write a permission rule that says "deny Edit on any file matching `src/auth/**`" because the pattern matching operates on tool invocation strings, not a glob engine over file paths. Sacred regions require hooks, not permissions.

### 2. Subagent definition generation — REMOVED

Claude Code does not have a declarative "subagent definition" file format. The Agent tool is invoked programmatically during conversations, not configured via files. There is no artifact `brief emit claude-agent` could produce. The closest analog — a skill that reviews constraints — is already covered by `brief emit skill`.

### 3. Brief composition/inheritance — REMOVED

The `extends:` frontmatter proposal creates the CSS specificity problem for AI agents. When a sacred region inherited from an org-level brief conflicts with a team-level constraint, the resolution semantics are ambiguous. In practice, the two-file approach (CLAUDE.md for project context + `.brief.md` for task intent) already provides composition. The `--install` flag makes this concrete: brief injects its section alongside whatever else is in CLAUDE.md.

---

## Items Downgraded

### 4. `## Commands` and `## Style` as first-class parsed sections — DOWNGRADED from Tier 1 to optional

With unknown section passthrough working, these sections flow through all emitters without any parser changes. The marginal value of first-class parsing is limited to:
- Auto-detection in `brief init` (can be done independently — just generate the heading as text in the scaffold)
- Structured data in JSON emit (moderate value)
- Target-specific reformatting (low value — the raw Markdown is already the right format)

Both reviewers independently concluded these are not worth the parser/model/emitter complexity when passthrough already handles the 80% case.

### 5. MCP server — DOWNGRADED from Tier 4 to long-term/on-demand

Static context injection via `--install` is superior to runtime MCP tools for most brief use cases. The total constraint set is typically small (under 500 tokens), where static injection wins on every axis: no latency, no tool-call budget consumption, no behavioral dependency on the agent knowing when to query. An agent that forgets to call `check_sacred` before editing is no better than one whose attention to the static instruction has decayed.

The sole MCP use case with genuine value — `check_sacred(path)` invoked automatically before every file edit — is better implemented as a PreToolUse hook, which is deterministic and framework-enforced rather than agent-initiated.

### 6. Emitter trait refactor — DOWNGRADED to "pay when needed"

This has zero impact on agent behavior. It should happen organically when the number of emitters justifies the abstraction cost (7+ targets), not as a planned milestone.

---

## Agent Team (Original Synthesis)

| Agent | Focus | Key Contribution |
|-------|-------|------------------|
| Context Engineer (original) | Data model, parser, emitters | Identified unknown section passthrough as the critical fix |
| Context Architect (original) | Format expressiveness, section design | Established augment-not-replace principle |
| Integration Engineer (original) | Ecosystem emit targets, Claude Code | Audited 5 ecosystems; identified deep integration surfaces |
| DevOps Engineer (original) | CI/CD, operational context | Designed validate-diff; argued Commands resolves operational gap |
| AI Applications Engineer (revision) | Claude Code integration accuracy | Corrected permissions/subagent misconceptions; designed hooks integration; identified `--install` as paradigm shift; recalibrated cross-ecosystem effort estimates |
| ML Architect (revision) | Context engineering, attention dynamics | Reframed emit as prompt engineering; proposed NEVER/MUST/PREFER/STOP constraint language; identified section ordering for attention optimization; added context budget awareness; retired fidelity percentage metric |
