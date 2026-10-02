# brief — Structured Briefing Format for AI Agents

## Project Summary

`brief` is a CLI tool that creates, validates, and emits structured briefing files (`.brief.md`) for AI coding agents. It solves the gap between unstructured `AGENTS.md`/`CLAUDE.md` prose and overly-programmatic prompt languages like PDL or LMQL.

**Intent:** brief is the source of record for human intent to AI systems. An author writes the intent once, in 60 seconds. brief checks it, compiles it into each runtime's native artifact (briefings for coding agents, definitions for aichat), installs those artifacts reversibly, and gates work against them. It never executes. See `docs/design-decisions.md` "Intent".

The core insight: as AI agents handle more technical execution, the human→agent interface becomes the bottleneck. `brief` provides a fast, familiar format (Markdown + YAML frontmatter) for humans to express intent, constraints, and sacred regions — and a CLI to validate, compose, and emit those briefings to multiple agent runtimes.

brief serves two groups of consumers. The external agent runtimes (Claude Code, Codex via `AGENTS.md`, Cursor, Copilot, Windsurf, Aider) get their native instruction files. The integrated Eridian stack gets its artifacts: brief is one of six repos (aichat, llm-functions, brief, astrophage, rocky, pi), and aichat names it the stack's primary composition language. The brief → aichat seam is emitted files only; today that is promptfoo replay providers for astrophage, and next an aichat role file. An agent-directory emitter is deferred until agent formats settle. The system picture from brief's side is `docs/architecture/README.md`.

## Architecture Decisions (Already Made)

These decisions were reached through extensive design analysis. Do not revisit them.

1. **Format: Markdown with YAML frontmatter and defined heading conventions.** Not TOML (too verbose, fights natural nesting), not pure YAML (indentation footguns), not a custom DSL. Markdown was chosen because: fastest to author (~60s), zero learning curve, LLM-native readability, near-zero emit cost to CLAUDE.md, git-friendly diffs, and checkbox syntax for assumption tracking.

2. **Language: Rust.** For single-binary distribution (`cargo install brief-cli`) and ecosystem fit. Not a performance decision.

3. **Tool, not framework.** `brief` is a CLI that reads `.brief.md` files and emits to targets. It is not a prompt programming language, not an agent framework, not a constraint enforcement engine. It provides cooperative tooling that makes constraint compliance easy, not mandatory.

4. **Format-first architecture.** The `.brief.md` format is the product. Runtime integrations (Claude Code, AGENTS.md, system prompts, MCP) are emit targets — plugins, not the core.

5. **Extend, do not replace** The `.brief.md` should extend, but not replace, existing well established conventions for agentic coding. In particular, brief
should be an asset to the `AGENTS.md` standard, which Claude Code now reads alongside the other agents, and to the Claude Code ecosystem (hooks, skills, permissions).

6. **Authoring layer of the integrated stack (2026-09-27).** In the six-repo Eridian system, brief declares and emits; aichat executes; astrophage owns the wire; llm-functions owns tools; rocky reads traces; pi is the REPL. brief's seams are files it writes, never calls in either direction. Emitted output is byte-stable, because aichat hashes an Entity's definition into its `lock_hash`. A change a sibling repo needs is documented in this repo and applied in that one. Hermes is reached only through aichat's metadata refinery, never by a brief emit target.

## .brief.md Format Specification

A `.brief.md` file has two parts:

### YAML Frontmatter

Machine-critical structured data that doesn't read well as prose:

```yaml
---
stack: [Python 3.12, PostgreSQL 16, Kafka 3.7, GCP/k8s]
context: [./performance-baseline.csv, ./current-architecture.md]
brief_version: "1"
---
```

The machine schema for frontmatter is generated from the Rust types, never
hand-written: `docs/schema/brief-frontmatter-v1.schema.json`, embedded via
`include_str!` in `src/schema.rs`. Regenerate with
`BLESS_SCHEMA=1 cargo test --test frontmatter_schema_tests`.

Frontmatter fields:
- `stack` (string[]): Technologies, languages, frameworks. Required.
- `context` (string[]): File paths or URLs providing reference material. Optional.
- `model` (string): Preferred model identifier. Optional.
- `brief_version` (string): `.brief.md` format version, currently "1". Optional, defaults to "1". An unknown value is a `brief validate` error. The legacy spelling `version` still parses.
- `cassettes` (string[]): astrophage cassette set names the brief's eval replays. Optional. Merged with `## Fixtures` items (`- cassette: <set> — <reason>`, which stay an unknown section) by `src/fixtures.rs`. See `docs/brief-format.md` §8.3.

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
- A constraint item may carry an optional leading **scope**: a bracketed, comma-separated list of backtick-wrapped globs the constraint applies to — `` - [`src/ui/**`] WCAG 2.1 AA ``. Empty scope (the common case) = project-wide. See `docs/brief-format.md` §8.1.
- Sacred items follow the pattern: `` `<path>` — <reason> `` (backtick-wrapped path, em dash or double hyphen, then reason).
- Assumption items use Markdown checkbox syntax: `- [ ]` unvalidated, `- [x]` validated.
- Unknown H2 sections are preserved as-is (extensibility).

## CLI Commands

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

The target list is authored in `.brief.md` (`## Emit Targets`) and generated into the managed section at the end of this file. Edit it there, then run `brief emit claude --install` and `brief emit agents-md --install`.

### `brief check <path>`
Check if a file path falls within a sacred region.
- `brief check src/auth/handler.rs` → exit 1, prints sacred reason.
- `brief check src/api/routes.rs` → exit 0.
- Useful for git hooks and CI integration.

### `brief diff <file1> <file2>`
Show semantic differences between two briefing files.
- Added/removed constraints, changed sacred regions, new assumptions.

### `brief validate-diff`
Gate a set of changed files (a git range or stdin) against the sacred regions. Exit non-zero on a violation; the CI form of `brief check`.

### `brief skill <init|validate|emit|search|scaffold|install|uninstall>`
Author and install Agent Skills. brief owns only `metadata.brief.source` inside a skill; the rest is hand-editable.

## Build & Project Structure

The module map and data flow are in `docs/architecture/README.md` §3. In short:

```
brief/
├── .brief.md              # The charter; generates the managed section at the end of this file
├── src/
│   ├── main.rs            # CLI entry point (clap)
│   ├── parse/             # frontmatter.rs (serde-saphyr) + body.rs (heading tree)
│   ├── model.rs           # The Brief IR
│   ├── validate.rs        # + provenance_check.rs, fixtures.rs
│   ├── emit/              # One module per target, plus markers.rs (managed sections)
│   ├── framing.rs         # Hard/Soft/Ask First → NEVER/PREFER/STOP register
│   ├── budget.rs          # --budget, --compact
│   ├── hooks.rs, skill.rs, commands.rs   # Install surfaces
│   └── check.rs, validate_diff.rs, pathmatch.rs   # Sacred gates
├── docs/                  # Format spec, schema, architecture, design
└── tests/                 # One file per feature; fixtures/ holds sample briefs
```

## Dependencies

Use minimal, well-maintained crates:
- `clap` (derive) — CLI argument parsing
- `serde`, `serde-saphyr` — frontmatter parsing (`serde_yaml` is archived and comment-lossy)
- `serde_json` — JSON emit
- `glob` — Sacred path matching
- `pulldown-cmark` — Markdown parsing (heading tree extraction)
- `colored` — Terminal output formatting
- `thiserror` or `anyhow` — Error handling
- `schemars` — Derives the frontmatter JSON Schema from the model types

Do NOT use heavy frameworks. No `tokio` (this is synchronous). No `reqwest`, `hyper` or `ureq`: brief makes no network calls, ever (aichat's anti-roadmap holds brief to this too).

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

<brief:generated>
# Briefing: Brief: the source of record for human intent to AI systems

**Stack:** Rust

## Constraints

### Hard (Non-negotiable)

<rules priority="required">
- MUST: A new `.brief.md` takes under sixty seconds to author.
- MUST: brief declares and emits files; it never executes. No model calls, sessions or tool loops, and no network crates (`tokio`, `reqwest`, `hyper`, `ureq`).
- MUST: Distributed as a single binary, or a single binary plus configuration files.
- MUST: Emitted files fit each target's existing conventions (`AGENTS.md`, `CLAUDE.md`, `.cursor/rules/`, aichat roles, promptfoo providers) and never replace hand-authored content.
- MUST: Installs are non-destructive, idempotent and reversible: generated content lives inside `<brief:generated>` markers or in files brief owns, and `--uninstall` removes exactly that.
- MUST: The same brief emits byte-identical output on every run: no timestamps or random values in emitted files. aichat hashes an Entity's definition into its `lock_hash`, so unstable output reads as a new version.
- MUST: The seams with the sibling repos (aichat, astrophage, llm-functions, rocky, pi) are emitted files only. A change a sibling repo needs is documented in this repo and applied in that one.
- MUST: The format extends through unknown `##` sections and new emit targets, without a grammar change.
</rules>

### Soft (Preferred)

<rules priority="preferred">
- PREFER: Keep the format low cognitive load, not merely lower burden than prose: an author should not need `docs/brief-format.md` open to write a valid brief.
- PREFER: Dogfood brief: this file is its first use case, and `brief emit claude --install` generates this repo's `CLAUDE.md` section from it.
- PREFER: A new frontmatter field clears the YAGNI bar in `docs/design/frontmatter-additions.md` before it is built.
</rules>

### Ask First (Requires approval)

<rules priority="ask-first">
- STOP and confirm with the user before: Adding an emit target for a runtime brief does not support yet
- STOP and confirm with the user before: Changing the `.brief.md` grammar or bumping `brief_version`
</rules>

## Sacred Regions (Do Not Modify)

The following files and directories must not be modified under any circumstances. If a task requires changes to these paths, STOP and report the conflict.

<protected_files>
- `docs/schema/brief-frontmatter-v1.schema.json` — generated from the Rust Frontmatter type; regenerate with BLESS_SCHEMA=1 cargo test --test frontmatter_schema_tests, never edit by hand
</protected_files>

## Reference Context

Read these files for background before starting work:

<context>
- @README.md
- @docs/architecture/README.md
</context>

## Deliverable

<deliverable>
A human-authorable format, and one CLI that checks it, compiles it, installs it reversibly and gates work against it, for every runtime that consumes intent: briefings for the coding agents (`AGENTS.md` for Codex, Cursor, Copilot and Claude Code; `CLAUDE.md` with hooks and skills; Windsurf; Aider) and definitions for the integrated stack (an aichat role, promptfoo replay against astrophage). Secondarily, the format and the target set stay extensible, so a new runtime is a new emitter rather than a new format.
</deliverable>

## Emit Targets

`brief emit <target>` compiles a `.brief.md` into one runtime's native artifact. The full reference is `docs/emit-targets-reference.md`.

- `claude` — a `CLAUDE.md` section in Claude Code conventions. The primary target.
- `agents-md` — an `AGENTS.md` section.
- `cursor`, `copilot`, `windsurf`, `aider`, `readme` — each ecosystem's native file, in its own register.
- `prompt` — raw system prompt text for API use.
- `anchor` — a ≤5-line NEVER/MUST constraint anchor for mid-session re-injection.
- `json` — the parsed briefing as structured JSON, with its content-hash `id` (`docs/schema/SPEC.md` §7).
- `jsonl` — one JSON line per brief: the `id` plus the goal, deliverable and constraints it hashes. Lines append into a stream that training data joins on `brief_id`.
- `xml` — an XML envelope with closing tags, for prompts that concatenate untrusted content.
- `promptfoo` — promptfoo providers that replay the brief's cassette sets through the astrophage gateway (base URL plus `x-aichat-cache-mode` and `x-aichat-cassette` headers). Every provider also sends `x-brief-id`. `--gateway <address>` overrides the default `127.0.0.1:8080`. `--session-id` switches to promptfoo `http` providers that send `X-Eridian-Session-Id` per test case. Stdout only; brief never runs astrophage or promptfoo. Mode selection is astrophage-side: aichat has no cassette flags, by decision.
</brief:generated>
