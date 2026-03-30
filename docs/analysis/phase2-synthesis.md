# Phase 2 Synthesis: Bridging the CLAUDE.md Flexibility Gap

**Date:** 2026-03-29
**Analysis by:** Four parallel Claude Opus 4.6 agents operating as specialized roles:
- **Context Engineer** — data model, parser, emitters, technical pipeline ([full report](./flexibility-gap.md))
- **Context Architect** — format expressiveness, section design, purpose ([full report](./format-expressiveness.md))
- **Integration Engineer** — ecosystem emit targets, Claude Code deep integration ([full report](./emit-integration-audit.md))
- **DevOps Engineer** — CI/CD enforcement, operational context, team-scale operations ([full report](./devops-engineer.md))

---

## The Problem

Brief's `.brief.md` format captures ~50% of what makes a real CLAUDE.md useful. The missing 50% -- build commands, code style rules, architecture context, dependency policies, project structure, behavioral instructions -- is the substance of what agents actually reference most frequently during execution.

The question: should brief expand its sections, improve its documentation, or both?

---

## Universal Convergence (All Four Agents Agree)

### 1. Unknown sections must be emitted

Every agent independently identified the same critical bug: the parser preserves `unknown_sections` in the data model, but 4 of 5 emitters silently drop them. This means the format's built-in extensibility mechanism is a dead end. A user who adds `## Commands` to their `.brief.md` today will find it vanish from `brief emit claude`.

**Verdict:** Fix this first. It is ~30 lines of code across 4 emitter files and immediately closes 50-60% of the CLAUDE.md fidelity gap by allowing any freeform section to flow through.

### 2. A `## Commands` section is the single most impactful addition

All four agents ranked build/test/lint/format commands as the #1 missing content type. The Context Engineer called it "the single highest-value first-class addition." The Context Architect said it is "the most universally present section in real CLAUDE.md files." The Integration Engineer rated it the #1 missing CLAUDE.md content type. The DevOps Engineer called it "the most operationally impactful context an AI agent receives."

**Verdict:** Add `## Commands` as a first-class parsed section. Auto-detect from Cargo.toml, package.json, Makefile, pyproject.toml in `brief init`. Put structured command data in frontmatter (`commands: BTreeMap<String, String>`) or parse as `## Commands` with `Name: \`command\`` pairs.

### 3. Brief should augment CLAUDE.md, not replace it

The Context Architect explicitly argued this case: brief should remain a task-level intent specification that slots into a broader CLAUDE.md, not a complete replacement. Standing project context (architecture, project structure, full style guides) belongs in the CLAUDE.md itself or in a base `.brief.md` with composition/inheritance. Task-specific context (goal, constraints, sacred regions, assumptions) is brief's core.

The 60-second authoring promise is incompatible with capturing the full breadth of a real CLAUDE.md. The format should grow enough to cover the most common gaps while staying lean.

### 4. `brief validate-diff` is the enforcement breakthrough

The DevOps Engineer ranked it #1. The Integration Engineer identified CI enforcement as a critical gap. Currently, brief is entirely advisory -- there is no way to block a PR that touches sacred regions. A `brief validate-diff <ref>` command that checks a git diff against sacred regions would transform brief from documentation into a CI gate.

---

## The Recommended Architecture

### Tier 0: Fix the Bug (effort: hours)

| Change | Files | Impact |
|--------|-------|--------|
| Emit unknown sections in all text emitters | `emit/claude.rs`, `emit/prompt.rs`, `emit/agents_md.rs`, `emit/skill.rs` | Unlocks all convention-based sections immediately |
| Preserve raw Markdown in unknown sections | `parse/body.rs` | Makes passthrough lossless (code blocks, sub-headings, emphasis) |
| Emit context files as `@` references in Claude target | `emit/claude.rs` | Makes context files functional, not decorative |
| Add emphasis to hard constraints in Claude target | `emit/claude.rs` | `**IMPORTANT:**` prefix measurably improves adherence |

### Tier 1: New First-Class Sections (effort: days)

Two sections deserve first-class status because they benefit from structured parsing, target-specific reformatting, and auto-detection:

**`## Commands`** — Build/test/lint/format commands as name-command pairs.

```markdown
## Commands
- Build: `cargo build`
- Test: `cargo test`
- Lint: `cargo clippy`
- Format: `cargo fmt`
```

Model: `commands: Vec<CommandEntry>` or frontmatter `commands: BTreeMap<String, String>`.
Auto-detection: `brief init` reads Cargo.toml, package.json scripts, Makefile targets, pyproject.toml tool configs.
Emit: Per-target formatting (CLAUDE.md heading, prompt label block, JSON structured).

**`## Style`** — Code conventions as a flat list.

```markdown
## Style
- Use `thiserror` for library errors, `anyhow` for CLI
- Emitters take `&Brief` and return `String`
- Tests for every parser edge case
```

Model: `style: Vec<String>`.
Why first-class: Semantically distinct from constraints. Constraints are negotiable boundaries; style rules are standing conventions. Emitters should render them differently (no "NON-NEGOTIABLE" / "PREFERRED" annotations, just direct instructions).

### Tier 2: Documented Conventions for Unknown Sections (effort: documentation only)

Three categories are freeform prose that does not benefit from structured parsing. They should be documented conventions that flow through the now-functional unknown section passthrough:

| Convention Name | Purpose | Example Content |
|----------------|---------|-----------------|
| `## Context` | Architecture overview, project structure, how things work | "This is a CLI tool that reads `.brief.md` files..." + directory tree |
| `## Workflow` | Agent behavioral instructions, scope, approach | "Make small commits. Ask before refactoring. Prefer surgical fixes." |
| `## Diagnosis` | Problem hypothesis, what's been tried, known failures | "Login returns 500 after cache flush. Tried middleware warmup, didn't help." |

These require zero parser or model changes. They work immediately once unknown sections are emitted.

### Tier 3: New CLI Commands (effort: days to weeks)

| Command | Purpose | Agent Recommending |
|---------|---------|-------------------|
| `brief validate-diff <ref>` | Check git diff against sacred regions; exit 1 on violations | DevOps (ranked #1) |
| `brief verify` | Run verification commands from structured deliverables | DevOps (ranked #5), Context Architect |
| `brief emit copilot` | Emit `.github/copilot-instructions.md` | Integration (ranked #4) |
| `brief emit cursor` | Emit `.cursorrules` (legacy single-file) | Integration (ranked #5) |
| `brief emit windsurf` | Emit `.windsurfrules` | Integration (ranked #6) |
| `brief emit aider` | Emit `CONVENTIONS.md` + `.aider.conf.yml` snippet | Integration (ranked #7) |
| `brief sync` | Emit all detected targets at once | Integration (ranked #12) |

### Tier 4: Claude Code Deep Integration (effort: weeks)

The Integration Engineer identified five deep integration surfaces beyond CLAUDE.md text:

1. **Hooks** — `brief emit claude-hooks` generates a `PreToolUse` hook that runs `brief check` on every Edit/Write, deterministically blocking writes to sacred regions.

2. **Permissions** — `brief emit claude-permissions` generates `permissions.deny` rules from sacred entries. Hardest possible enforcement: Claude cannot even attempt to write to sacred paths.

3. **MCP server** — `brief mcp-serve` exposes `check_sacred_path`, `get_constraints`, `get_briefing`, `validate_brief` as runtime-queryable tools. Sidesteps the emit fidelity problem entirely for MCP-capable agents.

4. **Subagents** — `brief emit claude-agent` generates a constraint-reviewer subagent definition.

5. **Skills** — Already partially implemented. Gap: `skill_manual: bool` frontmatter for `disable-model-invocation`.

### Tier 5: Architecture Improvements (effort: weeks)

| Change | Rationale |
|--------|-----------|
| Emitter trait abstraction | `trait Emitter` with per-section methods + default orchestrator. Pays for itself at the 6th emitter. |
| Constraint scope metadata | Optional `scope: Vec<String>` (glob patterns) per constraint. Required for idiomatic Cursor .mdc and Copilot .instructions.md multi-file emit. |
| Brief composition/inheritance | `extends:` frontmatter field. Union semantics for hard constraints and sacred; override for soft constraints and commands. Enables monorepo and org-wide policies. |
| `environment` frontmatter field | Structured infrastructure data: services, config files, deployment topology. |

---

## What NOT To Do

1. **Do not bump the format version.** All changes are additive. Version "1" remains correct. `serde(default)` handles new fields gracefully. Unknown sections already work for new headings.

2. **Do not try to replace CLAUDE.md entirely.** Brief augments; it does not replace. Standing project context belongs in the CLAUDE.md. Brief handles task-specific structured intent.

3. **Do not add Phases/Subtasks as a first-class section.** Task decomposition is better addressed through brief composition (multiple brief files) in Phase 3.

4. **Do not build a template engine for emitters yet.** The trait abstraction is the right level of extensibility for Phase 2. A Tera/Handlebars template system is premature until community contributors need to add targets without writing Rust.

5. **Do not add infrastructure details inline.** Connection strings, env var catalogs, deployment topology belong in dedicated files referenced via `context`. Brief is not an infrastructure manifest.

---

## Implementation Sequence

```
Week 1:  Tier 0 (bug fixes) + unknown section passthrough
         → Immediately unblocks convention-based sections
         → CLAUDE.md fidelity jumps from ~35% to ~70%

Week 2:  Tier 1 (Commands + Style sections)
         → CLAUDE.md fidelity reaches ~85%
         → brief init auto-detects commands

Week 3:  validate-diff + machine-readable output (--json)
         → CI enforcement becomes possible
         → GitHub Action and pre-commit hook support

Week 4:  New emit targets (copilot, cursor, windsurf, aider)
         → Addressable user base multiplies 5-10x
         → "Write once, emit everywhere" becomes real

Month 2: Claude Code deep integration (hooks, permissions, MCP)
         → Sacred regions become enforced, not advisory
         → Brief becomes runtime infrastructure

Month 3: Composition/inheritance + emitter trait refactor
         → Monorepo and org-wide policy support
         → Extension cost drops for new targets
```

---

## The Core Insight

All four agents converged on the same conclusion from different angles:

> **Brief's format is sound. The bug is that the extensibility mechanism (unknown sections) is broken at the emitter layer, and the two most universally needed content types (commands, style) lack first-class support.**

The fix is surgical, not architectural. Brief does not need a redesign. It needs:
1. Unknown sections to actually emit (30 lines of code)
2. A Commands section (most impactful format addition)
3. A Style section (second most impactful)
4. `validate-diff` (transforms brief from advisory to enforceable)
5. Cross-ecosystem emit targets (multiplies addressable users)

The format stays lean. The 60-second authoring target survives. The gap closes from ~50% missing to ~15% missing. The remaining 15% (deep architecture context, organizational conventions, full project structure) belongs in the CLAUDE.md itself, not in brief -- and that is by design.

---

## Agent Team

| Agent | Focus | Key Contribution |
|-------|-------|------------------|
| Context Engineer | Data model, parser, emitters | Identified the hybrid approach: promote only what benefits from structured parsing; everything else flows through unknown sections |
| Context Architect | Format expressiveness, section design | Established that brief should augment not replace CLAUDE.md; proposed Commands + Style as the two first-class additions |
| Integration Engineer | Ecosystem emit targets, Claude Code | Audited 5 ecosystems; identified hooks/permissions as deterministic enforcement; rated CLAUDE.md fidelity at 35-40% |
| DevOps Engineer | CI/CD, operational context | Designed validate-diff, GitHub Action, pre-commit integration; argued Commands section resolves the operational context gap |

Full analyses from each agent are in this directory.
