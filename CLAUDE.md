# brief — Structured Briefing Format for AI Agents

## Project Summary

`brief` is a CLI tool that creates, validates, and emits structured briefing files (`.brief.md`) for AI coding agents. It solves the gap between unstructured `CLAUDE.md`/`AGENTS.md` prose and overly-programmatic prompt languages like PDL or LMQL.

The core insight: as AI agents handle more technical execution, the human→agent interface becomes the bottleneck. `brief` provides a fast, familiar format (Markdown + YAML frontmatter) for humans to express intent, constraints, and sacred regions — and a CLI to validate, compose, and emit those briefings to multiple agent runtimes.

## Architecture Decisions (Already Made)

These decisions were reached through extensive design analysis. Do not revisit them.

1. **Format: Markdown with YAML frontmatter and defined heading conventions.** Not TOML (too verbose, fights natural nesting), not pure YAML (indentation footguns), not a custom DSL. Markdown was chosen because: fastest to author (~60s), zero learning curve, LLM-native readability, near-zero emit cost to CLAUDE.md, git-friendly diffs, and checkbox syntax for assumption tracking.

2. **Language: Rust.** For single-binary distribution (`cargo install brief-cli`) and ecosystem fit. Not a performance decision.

3. **Tool, not framework.** `brief` is a CLI that reads `.brief.md` files and emits to targets. It is not a prompt programming language, not an agent framework, not a constraint enforcement engine. It provides cooperative tooling that makes constraint compliance easy, not mandatory.

4. **Format-first architecture.** The `.brief.md` format is the product. Runtime integrations (Claude Code, AGENTS.md, system prompts, MCP) are emit targets — plugins, not the core.

5. **Extend, do not replace** The `.brief.md` should extend, but not replace, existing well established conventions for agentic coding. In particular, brief 
should be an asset to the Claude Code ecosystem.

## .brief.md Format Specification

A `.brief.md` file has two parts:

### YAML Frontmatter

Machine-critical structured data that doesn't read well as prose:

```yaml
---
stack: [Python 3.12, PostgreSQL 16, Kafka 3.7, GCP/k8s]
context: [./performance-baseline.csv, ./current-architecture.md]
---
```

Frontmatter fields:
- `stack` (string[]): Technologies, languages, frameworks. Required.
- `context` (string[]): File paths or URLs providing reference material. Optional.
- `model` (string): Preferred model identifier. Optional.
- `version` (string): Brief format version, currently "1". Optional, defaults to "1".

### Markdown Body

Human-authored intent using a defined heading convention:

```markdown
# <Goal statement as H1>

## Constraints

### Hard
- <non-negotiable constraint>

### Soft
- <preferred but flexible constraint>

### Ask First
- <requires human approval before proceeding>

## Sacred
- `<glob pattern>` — <reason>

## Assumptions
- [ ] <unvalidated assumption>
- [x] <validated assumption>

## Deliverable
<free-text description of what "done" looks like>
```

**Heading parsing rules:**
- H1 (`#`) = goal statement. Exactly one required.
- H2 (`##`) = top-level section. Known sections: Constraints, Sacred, Assumptions, Deliverable.
- H3 (`###`) under Constraints = constraint type: Hard, Soft, Ask First.
- List items under headings = the actual content.
- Sacred items follow the pattern: `` `<path>` — <reason> `` (backtick-wrapped path, em dash or double hyphen, then reason).
- Assumption items use Markdown checkbox syntax: `- [ ]` unvalidated, `- [x]` validated.
- Unknown H2 sections are preserved as-is (extensibility).

## CLI Commands (Phase 1)

### `brief init`
Analyze the current directory and scaffold a `.brief.md` with sensible defaults.
- Detect stack from `Cargo.toml`, `pyproject.toml`, `package.json`, `go.mod`, `Gemfile`, `docker-compose.yml`, etc.
- Detect existing sacred candidates (common patterns: `**/auth/**`, `**/migrations/**`, files with license headers).
- Populate `context` with README, architecture docs if found.
- Output: `.brief.md` in current directory.

### `brief validate`
Check the current `.brief.md` against the codebase.
- Verify sacred paths actually exist (warn if glob matches nothing).
- Verify context files exist and are readable.
- Check for common format errors (missing H1, unknown constraint types, malformed sacred entries).
- Exit 0 if valid, exit 1 with diagnostics if not.

### `brief emit <target>`
Transform `.brief.md` into a target format.

Targets:
- `claude` — Emit a `CLAUDE.md` section with constraints formatted as Claude Code conventions. Supports `--install` (idempotently injects/replaces a `<brief:generated>
# Briefing: Brief, a best-in-class structured file format for agents

**Stack:** Rust

## Reference Context

Read these files for background before starting work:

<context>
- @README.md
</context>

## Constraints

### Hard (Non-negotiable)

<rules priority="required">
- **IMPORTANT:** Format takes less then sixty seconds to author
- **IMPORTANT:** System reduces the human cognitive burden of interacting with agents
- **IMPORTANT:** Distributed as a single binary, or single-binary plus configuration files.
- **IMPORTANT:** Tooling interoprates and does not replace existing file formats (Claude.md, .cursorrules, etc.)
- **IMPORTANT:** Example: Brief uses a an xml tag 'brief:generated' to encapsulate generated content for CLAUDE.md files.
</rules>

### Soft (Preferred)

<rules priority="preferred">
- The format should not be high cognitive load, as there is much variance in complexity between "reducing the cognitive burden of humans", and "low cognitive load" formats.
- The format should be dogfoodable. In other words, the .brief.md of the brief project should be the first use-case.
</rules>

## Deliverable

<deliverable>
A human-authorable format with multiple outputs that can control different pieces of the runtime. Secondarily,
the format should be extensible in order to support new use cases and new formats.
</deliverable>
</brief:generated>

