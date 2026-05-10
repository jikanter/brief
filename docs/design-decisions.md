# Design Decisions

This document captures the key decisions made during the design phase of `brief`. These are settled — the rationale is provided for context, not for revisiting.

## Problem Statement

We are at an inflection point where AI agents handle most technical execution. The remaining human value is judgment, direction, and constraint specification. The bottleneck is the human→agent interface: how efficiently a human can communicate intent, constraints, and sacred regions to an agent.

Existing tools fall into two camps:
- **Too unstructured**: `CLAUDE.md`, `AGENTS.md` — freeform prose, no schema, no validation, no enforcement.
- **Too programmatic**: IBM PDL (YAML prompt programming), LMQL (constraint decoding), Microsoft Guidance (token-level steering) — powerful but wrong abstraction for humans expressing intent.

`brief` fills the gap with a format that's writable in 60 seconds, machine-parseable, and constraint-native.

## Format: Markdown + YAML Frontmatter

**Not TOML**: TOML is ~50% more verbose for this schema. `[section]` headers and mandatory quoting add friction. `[[array_of_tables]]` syntax is unintuitive. TOML excels at flat config (Cargo.toml, pyproject.toml); this schema has natural nesting that TOML fights.

**Not pure YAML**: YAML's indentation sensitivity causes silent, catastrophic errors. Quoting rules are complex (`yes`, `no`, `on`, `off`, `null` all parse as non-strings). For the frontmatter (flat, machine-critical data), YAML is fine. For the body (human intent), it's hostile.

**Not a custom DSL**: Zero learning curve is a hard requirement. Every developer knows Markdown.

**Why this hybrid**: YAML frontmatter carries machine-critical structured data (stack arrays, file paths). Markdown body carries human intent using heading hierarchy as the taxonomy. The heading structure IS the constraint schema — `## Constraints > ### Hard > list items` maps directly to `constraints.hard: [...]`. Checkbox syntax (`- [ ]` / `- [x]`) provides assumption state tracking in plain text. Near-zero emit cost to `CLAUDE.md` (the primary target).

## Constraint Taxonomy: Hard / Soft / Ask First

Inspired by Addy Osmani's boundary taxonomy (✅ Always / ⚠️ Ask first / 🚫 Never) but formalized as parseable headings. Three levels:

- **Hard**: Non-negotiable. Agent must comply or abort.
- **Soft**: Preferred but flexible. Agent should follow unless there's a good reason not to.
- **Ask First**: Requires human approval before proceeding.

### Emit-time reframing (Phase 2 decision)

The three-tier taxonomy is the correct *authoring* model — it maps to how humans think about constraints. But the *emitted* language should be different. The Phase 2 ML architecture review found that LLMs have weak priors on "Hard constraint" as a behavioral signal but strong priors on imperative/RFC-2119 language:

| Authoring format | Emitted language | Why |
|---|---|---|
| `### Hard` | `NEVER:` / `MUST:` | Imperative verbs trained across millions of docs; maps to action suppression |
| `### Soft` | `PREFER:` / `AVOID:` | Calibrated preference signals |
| `### Ask First` | `STOP and confirm before:` | Interruption behavior the model is heavily trained on |

The format stays human-friendly. The emitter translates to LLM-effective. This separation of authoring model from emit model is a core design principle.

## Sacred Regions

Files/directories the agent must not modify. Encoded as `` `glob` — reason ``. The glob is machine-parseable; the reason is human context.

Enforcement follows a capability ladder, established during the Phase 2 evaluation:

1. **Advisory text** (Phase 1, implemented): Sacred regions are emitted as instructions in CLAUDE.md/system prompts. Text-based compliance is ~90-95% for simple cases but drops to ~60-70% when the user's request directly contradicts the constraint.
2. **CI enforcement** (Phase 2): `brief validate-diff` checks git diffs against sacred regions. Deterministic, 100% enforcement in CI pipelines.
3. **Dev-time enforcement** (Phase 2): Claude Code `PreToolUse` hooks run `brief check` before every Edit/Write, deterministically blocking writes to sacred paths. Configured in `.claude/settings.json`, not via permissions.

**Why hooks, not permissions:** Claude Code's `permissions.deny` matches tool invocation patterns (`ToolName(argument_pattern)`), not file-path globs. You cannot express "deny edits to `src/auth/**`" through permissions. Hooks are the correct enforcement surface for path-based rules.

**Why not an MCP server for enforcement:** An agent that forgets to call `check_sacred` before editing is no better than one whose attention to a text instruction has decayed. Hooks are framework-enforced (deterministic); MCP tools are agent-initiated (probabilistic).

## Language: Rust

Distribution decision, not performance decision. The tool parses Markdown and emits text — any language would be fast enough. Rust provides single-binary distribution (`cargo install`) and ecosystem fit (Claude Code users are developers).

## Phased Roadmap

- **Phase 1**: Format spec, parser, CLI (init, validate, emit, check, diff). Complete.
- **Phase 2**: Emit quality, enforcement, and integration depth. Current phase. See below.
- **Phase 3**: Cross-ecosystem emit targets and extended tooling.

### Phase 2 Priorities (revised 2026-03-30)

The original Phase 2 plan centered on an MCP server. The Phase 2 evaluation — conducted by six specialized agents across two rounds — concluded that the MCP server is low-ROI given `--install` and hooks. The revised priorities, ordered by impact on agent behavioral compliance:

1. **Emit quality** — Reframe constraint language (NEVER/MUST/PREFER/STOP instead of Hard/Soft labels), reorder sections for LLM attention dynamics (constraints in primacy position, deliverable in recency position), strengthen sacred region framing.
2. **`validate-diff`** — CI-enforceable sacred regions. Check git diffs against sacred entries, exit non-zero on violations.
3. **Hooks integration** — `--install --hooks` adds `PreToolUse` hooks to `.claude/settings.json` for deterministic sacred region enforcement at dev time.
4. **Context budget awareness** — Token count reporting, `--compact` mode, size warnings to prevent context window bloat.
5. **`--install` enhancements** — Injection position control (`--position top|bottom`), unified install (CLAUDE.md + hooks + skill in one command), uninstall capability.

### What was deprioritized and why

- **MCP server**: Static context injection via `--install` is superior when the constraint set is small (under 500 tokens). Build only if briefs routinely exceed 1000 tokens and users request it.
- **Composition/inheritance**: The `extends:` frontmatter proposal creates ambiguous resolution semantics. The two-file approach (CLAUDE.md + .brief.md) with `--install` already provides composition without union/override complexity.
- **First-class `## Commands` / `## Style` sections**: Unknown section passthrough already handles these. Auto-detection in `brief init` can be added independently without parser changes.
- **Emitter trait refactor**: Zero impact on agent behavior. Build when the 7th emit target justifies the abstraction cost.

## Emit as Prompt Engineering (Phase 2 Decision)

The Phase 2 evaluation established that the emitter is a prompt engineering problem, not a template rendering problem. This has concrete implications:

**Section ordering**: LLMs have primacy bias (first ~15% of context) and recency bias (last ~15%). The `prompt` target should place hard constraints and sacred regions first (primacy) and the deliverable last (recency). The `claude` target uses a compromise order since CLAUDE.md is read by humans too.

**Target-specific rendering**: Different targets are consumed differently. A system prompt (`prompt` target) occupies the highest-privilege position in the context hierarchy and should be aggressively optimized for compliance. A CLAUDE.md section is injected alongside other project context and should balance human readability with agent effectiveness.

**Context budget**: Every token of brief output displaces conversation history, code context, or tool output. The emitter should be context-window-aware: token count reporting, compact mode, size warnings. A bloated brief in the system prompt gets re-injected every turn.

## Augment, Not Replace (Phase 2 Decision)

Brief augments existing CLAUDE.md files; it does not replace them. This is both a scope decision and an ML decision:

- **Scope**: Brief handles task-specific structured intent (goal, constraints, sacred regions, assumptions). Standing project context (architecture, full style guides, project structure) belongs in the CLAUDE.md itself.
- **ML**: Existing CLAUDE.md files often contain project-specific prompt engineering developed through trial and error. Replacing this with generated content discards institutional knowledge about what framing works for that codebase and team.

### What belongs in brief vs. CLAUDE.md

The scope split above is the principle. Applied to the content categories users actually encounter:

| Content | Belongs in | Why |
|---|---|---|
| Goal of the current task | `.brief.md` | Task-specific intent — the definitional case |
| Hard/Soft/Ask-First constraints for the task | `.brief.md` | Task-specific constraint spec |
| Sacred regions (paths not to modify) | `.brief.md` | Task-specific enforcement target |
| Unvalidated assumptions | `.brief.md` | Task-specific epistemic state |
| Build/test/run commands | `CLAUDE.md` | Standing operational context, not task-specific |
| Code style rules (naming, formatting, error handling patterns) | `CLAUDE.md` | Standing conventions — do not re-author per task |
| Project structure / directory tree | `CLAUDE.md` | Standing spatial context for navigation |
| Architecture narrative, ADRs | `CLAUDE.md` | Standing project context — too expensive to re-author |
| Dependency policies ("no tokio", "minimize deps") | `CLAUDE.md` | Standing conventions unless the task is dependency-related |
| Behavioral instructions ("ask before ambiguous", "prefer small commits") | Skill or `CLAUDE.md` | Agent conduct, not work product spec — see [open-questions.md](open-questions.md) `[format]` Behavioral Instructions |

The common confusion: users try to put standing project conventions into a brief's Soft constraints, and then re-author the same list every task. If it doesn't change task-to-task, it belongs in CLAUDE.md, not brief.

The `--install` flag implements this via `<brief:generated>` / `</brief:generated>` XML-style markers for idempotent injection. The brief section is placed alongside (not instead of) existing CLAUDE.md content. 
(Earlier versions used HTML-comment markers, but Claude Code strips HTML comments from CLAUDE.md before the model sees it, 
which rendered briefings invisible to the agent. Legacy markers are still recognized on read and migrated to the new format on the next `--install`.)

## `--install` as the Integration Paradigm (Phase 2 Decision)

Rather than generating files for users to manually place, brief's `--install` flag directly configures the target environment:

- `brief emit claude --install` — injects into CLAUDE.md with markers
- `brief emit skill --install` — writes to `.claude/skills/<name>/SKILL.md`
- `brief emit claude --install --hooks` (planned) — also configures `.claude/settings.json`

This pushes brief from "tool that emits text" toward "CLI that configures AI agent environments from structured briefing files." The project's self-description should reflect this evolution. Brief remains a tool, not a framework — but it is a tool that writes to multiple integration surfaces, not just stdout.

## Evaluated Alternatives

| Tool | What it solves | Why not for us |
|------|---------------|----------------|
| IBM PDL | YAML prompt programming | Too programmatic; developer tool, not human briefing |
| LMQL | Constrained output decoding | Wrong problem; constrains output, not input intent |
| Microsoft Guidance | Token-level generation steering | Wrong abstraction layer; Python library, not briefing format |
| Prompt Decorators | Behavioral mode switches | Solves behavioral tuning, not intent/constraint spec |
| AGENTS.md/CLAUDE.md | Freeform agent instructions | Complementary; brief augments these with structured intent, constraints, and enforcement — not a replacement |
| Showboat | Agent→human demo artifacts | Complementary (output), not competitive (input) |
