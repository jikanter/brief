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

## Sacred Regions

Files/directories the agent must not modify. Encoded as `` `glob` — reason ``. The glob is machine-parseable; the reason is human context. Enforcement is cooperative (the agent is told, not blocked), with the MCP server providing tool-level checking in Phase 2.

## Language: Rust

Distribution decision, not performance decision. The tool parses Markdown and emits text — any language would be fast enough. Rust provides single-binary distribution (`cargo install`), ecosystem fit (Claude Code users are developers), and a shared core if the MCP server (Phase 2) is built in Rust.

## Phased Roadmap

- **Phase 1**: Format spec, parser, CLI (init, validate, emit, check). This phase.
- **Phase 2**: MCP server exposing `get_briefing`, `check_path`, `log_decision`, `get_constraints` tools.
- **Phase 3**: Composition/inheritance (project-level defaults with per-directory overrides).

## Evaluated Alternatives

| Tool | What it solves | Why not for us |
|------|---------------|----------------|
| IBM PDL | YAML prompt programming | Too programmatic; developer tool, not human briefing |
| LMQL | Constrained output decoding | Wrong problem; constrains output, not input intent |
| Microsoft Guidance | Token-level generation steering | Wrong abstraction layer; Python library, not briefing format |
| Prompt Decorators | Behavioral mode switches | Solves behavioral tuning, not intent/constraint spec |
| AGENTS.md/CLAUDE.md | Freeform agent instructions | Right intent, wrong format; no schema, no validation |
| Showboat | Agent→human demo artifacts | Complementary (output), not competitive (input) |
