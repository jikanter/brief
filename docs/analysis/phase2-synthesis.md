# Phase 2 Synthesis: Active Roadmap

**Last updated:** 2026-04-11
**Original synthesis:** 2026-03-29 (revised 2026-03-30) by multi-agent analysis
**Historical record:** [archive/obsolete-features.md](./archive/obsolete-features.md) — completed Tier 0 work, removed proposals, and downgraded items from the original synthesis.

This document is the living Phase 2 roadmap for `brief`. It captures the
problem framing, prioritized work, and guardrails that came out of the
original multi-agent synthesis and its subsequent revision, minus anything
that has since been shipped, removed, or downgraded (those live in the
archived obsolete-features document linked above).

---

## The Problem (Revised Framing)

The original synthesis framed the gap as "brief captures ~50% of what makes a CLAUDE.md useful" and tracked progress via fidelity percentages (35% → 70% → 85%). This metric is retired. Structural coverage of CLAUDE.md sections does not correlate with agent task quality. With unknown section passthrough working, brief no longer drops content — fidelity is limited only by what the user writes.

The actual remaining gaps are:

1. **Enforcement** — Constraints and sacred regions are advisory text. Nothing prevents an agent from violating them, especially when contradicted by user instructions.
2. **Emit quality** — The emitter treats constraint rendering as a template problem when it is actually a prompt engineering problem. How constraints are framed, ordered, and positioned in the output directly affects agent compliance rates.
3. **Integration depth** — `--install` injects into CLAUDE.md, but Claude Code offers deeper integration surfaces (hooks, settings.json) that can make sacred regions deterministic rather than probabilistic.

---

## Revised Priority Architecture

### P0: Emit Quality — Constraint Framing and Section Ordering (effort: 1-2 days)

The highest-impact changes require zero format changes and no new features. They are emit-time transformations that improve agent behavioral compliance.

**Constraint language reframing:**

The current emitter uses "Hard Constraints" / "Soft Constraints" / "Ask First" labels with `**IMPORTANT:**` prefixes. These labels are a human taxonomy. LLMs have weak priors on what "Hard constraint" means behaviorally but strong priors on imperative/RFC-2119 language trained across millions of documents.

| Brief format | Current emit | Revised emit |
|---|---|---|
| `### Hard` items | `**IMPORTANT:** <constraint>` | `NEVER: <constraint> — <consequence>` or `MUST: <constraint>` |
| `### Soft` items | `<constraint>` | `PREFER: <constraint>` |
| `### Ask First` items | `<constraint>` | `STOP and confirm with the user before: <constraint>` |

The NEVER/MUST/PREFER/STOP framing uses imperative verbs that map directly to action suppression, preference weighting, and interruption behavior. Estimated compliance improvement: 5-15% for hard constraints.

**Sacred region framing:**

Current: "DO NOT MODIFY" with path and reason.
Revised:

```
## Sacred Regions (DO NOT MODIFY)
The following files and directories must not be modified under any circumstances.
If a task requires changes to these paths, STOP and report the conflict.

- `src/auth/**` — Production SSO boundary; changes require security review
- `db/migrations/**` — Immutable migration history; append new migrations only
```

Key additions: "under any circumstances" removes ambiguity about exceptions. "STOP and report" gives the model an action to take instead of pure suppression, which is more reliable.

**Section ordering for attention dynamics:**

LLMs have primacy bias (first ~15% of context gets highest attention) and recency bias (last ~15%). The current emit order (goal → stack → constraints → sacred → unknown sections → deliverable) optimizes for human reading flow, not agent compliance.

Revised emit order for `prompt` target (maximum compliance):
1. Constraints (Hard) — primacy position
2. Sacred regions — primacy position
3. Goal — task frame
4. Stack — reference context
5. Constraints (Soft, Ask First) — middle position (fine for preferences)
6. Assumptions — reference material
7. Unknown sections — reference material
8. Deliverable — recency position (action directive)

The `claude` target should use a compromise order (goal first, then constraints) since CLAUDE.md is read by humans too. The `prompt` target should be aggressively optimized for compliance.

### P1: `validate-diff` — CI-Enforceable Sacred Regions (effort: 2-3 days)

This transforms brief from advisory documentation into a CI gate. The implementation:

```
brief validate-diff [--base <ref>] [--brief <file>] [--json]
```

Behavior:
1. Get changed files from `git diff --name-only <base>..HEAD`
2. Run each changed file through `brief check`
3. Exit non-zero if any file is in a sacred region
4. Output machine-readable JSON (`--json`) or human-readable text
5. Optionally accept a diff on stdin for use in hooks or CI

The CI integration writes itself: a GitHub Action that runs `brief validate-diff --base origin/main --json` and posts a comment on PRs that touch sacred regions.

### P2: Hooks Integration — Deterministic Sacred Region Enforcement (effort: 1-2 days)

Text-based sacred region instructions achieve ~90-95% compliance for simple cases, dropping to ~60-70% when the user's request directly contradicts the constraint. Hooks close this gap deterministically.

Claude Code hooks are shell commands configured in `.claude/settings.json` under a `hooks` key. A correct implementation:

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Edit|Write",
        "command": "brief check --hook \"$TOOL_INPUT_FILE\"",
        "timeout": 5000
      }
    ]
  }
}
```

The hook receives tool event data on stdin, runs `brief check`, and outputs a JSON decision (`allow` or `block` with reason). `brief check` already exists and exits non-zero for sacred files — the hook wraps it with the correct I/O protocol.

Implementation approach: `brief emit claude --install --hooks` both injects the CLAUDE.md section AND adds the PreToolUse hook to `.claude/settings.json`. This extends the `--install` paradigm naturally.

### P3: Context Window Budget Awareness (effort: 1 day)

Every token of brief output displaces conversation history, code context, or tool output. The emitter should:

- Report token count of emitted output (approximate, whitespace-split is sufficient)
- Offer a `--compact` mode that strips explanatory prose and emits only constraint/sacred/deliverable essentials
- Warn when emitted output exceeds a configurable threshold (default: 500 tokens for `prompt`, 2000 for `claude`)

This prevents the slow degradation of agent performance as briefs grow. A bloated brief consuming 3,000 tokens in the system prompt gets re-injected every turn.

### P4: Cross-Ecosystem Emit Targets (effort: 2-4 days total)

The original synthesis treated all four targets as equivalent effort. They are not:

| Target | Format | Effort | Notes |
|--------|--------|--------|-------|
| `copilot` | `.github/copilot-instructions.md` — Markdown | Trivial | Nearly identical to Claude emitter output |
| `windsurf` | `.windsurfrules` — Markdown | Trivial | Plain markdown, different file path |
| `aider` | `CONVENTIONS.md` — Markdown | Trivial | Plain markdown, different file path |
| `cursor` | `.cursor/rules/*.mdc` | Real work | YAML frontmatter (`description`, `globs`, `alwaysApply`) + markdown body. Meaningfully different format requiring a dedicated emitter |

Three trivial wrappers (1 day combined) + one real emitter for Cursor (2-3 days).

### P5: `--install` Enhancements (effort: 2-3 days)

**Injection position control:**

When brief content is injected into an existing CLAUDE.md, position matters for attention dynamics. Default to appending (non-destructive, safe), but support `--position top|bottom|after:<heading>` for users who want primacy positioning.

Top injection should include a reconciliation preamble: "The following task-specific constraints supplement the project instructions below." This tells the model how to reconcile potential conflicts.

**Unified install:**

A single `brief emit claude --install --full` that:
1. Injects the briefing section into CLAUDE.md (already done)
2. Installs a skill if `skill_name` is set in frontmatter
3. Adds hooks for sacred region enforcement
4. Generates permissions.allow entries for known-safe commands from `## Commands`

**Uninstall:**

`brief emit claude --uninstall` to remove the `<brief:generated>` / `</brief:generated>` section from CLAUDE.md (and any legacy `<!-- brief:start -->` / `<!-- brief:end -->` marker pair from older installs), remove hooks from settings.json, and remove the skill directory. Users need confidence they can reverse what brief did.

### P6: Emit Quality Refinements (effort: ongoing)

**Negative constraint framing:**

The emitter should strengthen hard constraints with automatic negative framing where applicable. A constraint "Use Result<T, AppError> for error handling" becomes "MUST use Result<T, AppError> for error handling. Do not use alternatives unless explicitly discussed."

**Constraint conflict detection:**

When `--install` injects into an existing CLAUDE.md, scan for potential contradictions. Full semantic dedup is hard, but keyword-overlap detection between brief constraints and existing CLAUDE.md content is tractable. Warn when a brief constraint appears to conflict with existing instructions.

**Constraint specificity validation:**

`brief validate` should flag vague constraints (heuristic: under 8 words, no concrete nouns or paths). Vague constraints ("follow best practices") have dramatically lower compliance than specific ones ("all public functions must return Result<T, AppError>").

**Attention anchoring:**

Generate a compact constraint summary block (3-5 lines, highest-priority constraints only) suitable for periodic re-injection by agent frameworks. This addresses attention decay in long multi-turn sessions without MCP complexity.

### Long-term / On-demand

| Item | When to build | Notes |
|------|---------------|-------|
| First-class `## Commands` parsing | When JSON emit consumers need structured command data | Auto-detection in `brief init` can be done independently |
| First-class `## Style` parsing | When a target needs to reformat style rules differently | Low priority — raw passthrough is sufficient |
| MCP server | When a user requests it or briefs routinely exceed 1000 tokens | Expose only `check_sacred(path)` if built |
| Emitter trait refactor | When the 7th emit target is added | Engineering hygiene, not user-facing |
| `environment` frontmatter field | When infrastructure context is a demonstrated need | Brief is not an infrastructure manifest |

---

## What NOT To Do

1. **Do not bump the format version.** All changes are additive. Version "1" remains correct.

2. **Do not try to replace CLAUDE.md entirely.** Existing CLAUDE.md files often contain project-specific prompt engineering developed through trial and error. Replacing this with generated content discards institutional knowledge about what framing actually works for that codebase and team. Brief augments; it does not replace.

3. **Do not use `permissions.deny` for sacred regions.** The permissions system matches tool invocation patterns, not file paths. Use hooks instead. (See [archive/obsolete-features.md](./archive/obsolete-features.md) for the full reasoning.)

4. **Do not generate subagent definitions.** This is not a Claude Code feature. Use skills for reusable behaviors.

5. **Do not build composition/inheritance.** The two-file approach (CLAUDE.md + .brief.md) with `--install` already provides composition without the complexity of union/override semantics.

6. **Do not optimize for "CLAUDE.md fidelity percentage."** Structural coverage does not predict agent compliance. Optimize for behavioral impact: constraint framing, section ordering, enforcement mechanisms.

7. **Do not add infrastructure details inline.** Connection strings, env var catalogs, deployment topology belong in dedicated files referenced via `context`.

---

## The Core Insight

The original synthesis concluded: "Brief's format is sound. The bug is that the extensibility mechanism (unknown sections) is broken at the emitter layer." That bug is now fixed. The revised insight:

> **The emitter is a prompt engineering problem, not a template rendering problem.** How constraints are framed (NEVER/MUST vs. Hard/Soft), where sections are positioned (primacy/recency vs. human reading order), and how sacred regions are enforced (hooks vs. text instructions) matter more than what sections exist in the output. The highest-impact improvements are all in emit quality and tooling integration, not format expansion.

Brief's next phase is not about structural completeness. It is about climbing the enforcement ladder:

```
Current:    Advisory text in CLAUDE.md (~90% compliance, simple cases)
                                       (~60-70% under contradictory user requests)

Phase 2a:   Optimized constraint framing (~95% compliance, simple cases)
            validate-diff CI gate (100% enforcement in CI)

Phase 2b:   PreToolUse hooks (100% enforcement at dev time)
            Unified --install (CLAUDE.md + hooks + permissions in one command)
```
