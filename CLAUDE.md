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
- `claude` — Emit a `CLAUDE.md` section with constraints formatted as Claude Code conventions. This is the primary target.
- `prompt` — Emit raw system prompt text suitable for API use.
- `agents-md` — Emit an `AGENTS.md` compatible section.
- `json` — Emit the parsed briefing as structured JSON (for tooling integration).

### `brief check <path>`
Check if a file path falls within a sacred region.
- `brief check src/auth/handler.rs` → exit 1, prints sacred reason.
- `brief check src/api/routes.rs` → exit 0.
- Useful for git hooks and CI integration.

### `brief diff <file1> <file2>`
Show semantic differences between two briefing files.
- Added/removed constraints, changed sacred regions, new assumptions.

## Build & Project Structure

```
brief/
├── Cargo.toml
├── CLAUDE.md              # This file
├── src/
│   ├── main.rs            # CLI entry point (clap)
│   ├── lib.rs             # Public API
│   ├── parse/
│   │   ├── mod.rs
│   │   ├── frontmatter.rs # YAML frontmatter extraction
│   │   └── body.rs        # Markdown heading tree parser
│   ├── model.rs           # Brief data structures
│   ├── validate.rs        # Validation logic
│   ├── emit/
│   │   ├── mod.rs
│   │   ├── claude.rs      # CLAUDE.md emitter
│   │   ├── prompt.rs      # Raw prompt emitter
│   │   ├── agents_md.rs   # AGENTS.md emitter
│   │   └── json.rs        # JSON emitter
│   ├── init.rs            # Repo analyzer + scaffolder
│   └── check.rs           # Sacred path checker
├── tests/
│   ├── fixtures/           # Sample .brief.md files
│   └── integration/
└── examples/
    └── sample.brief.md
```

## Dependencies

Use minimal, well-maintained crates:
- `clap` (derive) — CLI argument parsing
- `serde`, `serde_yaml` — frontmatter parsing
- `serde_json` — JSON emit
- `glob` — Sacred path matching
- `pulldown-cmark` — Markdown parsing (heading tree extraction)
- `colored` — Terminal output formatting
- `thiserror` or `anyhow` — Error handling

Do NOT use heavy frameworks. No `tokio` (this is synchronous). No `reqwest` (no network calls in Phase 1).

## Code Style

- Use `thiserror` for library errors, `anyhow` for CLI error propagation.
- Parse into a strongly-typed `Brief` struct, then operate on that.
- Emitters take `&Brief` and return `String`.
- Tests for every parser edge case (missing sections, malformed sacred entries, empty frontmatter).
- Integration tests that round-trip: parse a fixture → emit → verify output contains expected content.

## Commands

- Build: `cargo build`
- Test: `cargo test`
- Run: `cargo run -- <subcommand>`
- Lint: `cargo clippy`
- Format: `cargo fmt`
