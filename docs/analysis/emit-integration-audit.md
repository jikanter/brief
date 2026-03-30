# Emit Target Integration Audit: Feature Parity and Cross-Ecosystem Analysis

**Date:** 2026-03-29
**Author:** Integration Engineer (Claude Opus 4.6)
**Scope:** Evaluate `brief emit` fidelity against real-world AI tooling formats, identify gaps, and recommend concrete changes.

---

## 1. CLAUDE.md Feature Parity Audit

### What a production CLAUDE.md actually contains

Based on Anthropic's official documentation, community best practices, and the CLAUDE.md already in this repository, a production CLAUDE.md typically includes these content categories:

| Content Type | Example | Present in brief model? |
|---|---|---|
| Project summary / one-liner orientation | "This is a Rust CLI tool for structured AI briefings" | Partial (goal serves this role, but goal is task-oriented, not project-oriented) |
| Stack and technology choices | Rust, clap, serde, pulldown-cmark | Yes (`frontmatter.stack`) |
| Build/run/test commands | `cargo build`, `cargo test`, `cargo clippy` | No |
| Code style rules | "Use thiserror for library errors, anyhow for CLI" | No (would need to be in unknown sections or constraints) |
| Architecture decisions | "Format-first architecture, tool not framework" | No |
| Project structure / file map | `src/parse/`, `src/emit/`, `src/model.rs` | No |
| Dependency policy | "No tokio, no reqwest" | Partial (expressible as hard constraint, but loses the "why") |
| Workflow rules | "Parse into Brief struct, then operate on that" | No |
| Sacred/protected regions | Auth modules, migrations | Yes (`sacred`) |
| Gotchas and non-obvious behaviors | "Frontmatter YAML errors lack line numbers" | No |
| Common commands Claude should use | `cargo run -- validate`, `cargo run -- emit claude` | No |
| Git/PR conventions | Branch naming, commit message format | No |
| Environment quirks | Required env vars, platform-specific setup | No |
| `@` imports referencing other docs | `@README.md`, `@docs/architecture.md` | Partial (context files listed, but not emitted as `@` imports) |

### What `brief emit claude` currently produces

The current Claude emitter (`src/emit/claude.rs`, 85 lines) outputs exactly:

1. `# Briefing: <goal>` -- H1 heading
2. `**Stack:** <comma-separated list>` -- inline bold
3. `## Reference Context` -- file list as code-formatted bullets
4. `## Constraints` -- with H3 subsections (Hard/Soft/Ask First)
5. `## Sacred Regions (Do Not Modify)` -- path + reason list
6. `## Assumptions` -- checkbox list
7. `## Deliverable` -- freeform text

It does not emit unknown sections. It does not emit commands, code style, architecture decisions, project structure, gotchas, workflow rules, git conventions, or `@` import syntax.

### Fidelity gap rating: 35-40%

The emitter produces a structurally valid CLAUDE.md, but it covers roughly a third of what teams put in production CLAUDE.md files. The critical missing categories are:

1. **Build/test/lint commands** -- the single most actionable content type in a CLAUDE.md. Claude Code uses these exact commands when you ask it to build or test. There is no field in the Brief model for this.
2. **Code style rules** -- the second most impactful content type. Brief has no dedicated section; these could theoretically go in hard constraints, but that conflates "must use ES modules" with "must not break API compatibility."
3. **Unknown sections not emitted** -- the parser preserves them in `unknown_sections: Vec<UnknownSection>`, but the Claude emitter ignores them entirely. This is the single largest bug: the format already supports extensibility, but the most important emitter throws it away.
4. **`@` import syntax** -- Claude Code supports `@path/to/file` references in CLAUDE.md that cause Claude to read those files. The `context` frontmatter field maps directly to this, but the emitter renders them as bullet list items rather than `@` references.

---

## 2. Cross-Ecosystem Emit Analysis

### Claude Code (CLAUDE.md)

**What Claude Code actually parses or benefits from:**
- Freeform Markdown; no required schema. Claude reads it as natural language context.
- `@path` references trigger file reads (e.g., `@README.md`, `@docs/architecture.md`).
- Emphasis markers like "IMPORTANT" or "YOU MUST" measurably improve adherence.
- Brevity matters: Anthropic recommends under 200 lines. Bloated files cause instructions to be ignored.
- CLAUDE.md is loaded every session, so only broadly-applicable content belongs here.
- Claude Code also supports `.claude/CLAUDE.md`, parent directory CLAUDE.md files (monorepo support), and `~/.claude/CLAUDE.md` (user-global).

**What brief needs to express but currently cannot:**
- Build/test/lint commands (highest priority)
- Code style conventions
- Architecture decision records (even abbreviated)
- Gotchas / non-obvious behaviors
- Git workflow conventions
- `@` import references for context files

**Ideal CLAUDE.md structure for brief to target:**
```markdown
# Project Name

Brief project description.

**Stack:** Rust, clap, serde, pulldown-cmark

## Commands
- Build: `cargo build`
- Test: `cargo test`
- Lint: `cargo clippy`
- Format: `cargo fmt`

## Code Style
- Use thiserror for library errors, anyhow for CLI
- Emitters take &Brief and return String

## Constraints
### Non-negotiable
- ...

### Preferred
- ...

### Ask First
- ...

## Sacred Regions (Do Not Modify)
- `src/auth/**` -- Authentication logic

## References
See @README.md for project overview.
See @docs/architecture.md for system design.

## Gotchas
- Frontmatter YAML errors lack line numbers
```

### GitHub Copilot (.github/copilot-instructions.md and .instructions.md)

**Format:** Plain Markdown, no frontmatter required for repository-wide instructions. Natural language directives.

**Path-specific instructions:** `.github/instructions/NAME.instructions.md` files with YAML frontmatter containing an `applyTo` glob field:
```yaml
---
applyTo: "**/*.ts"
---
```

**What Copilot benefits from:**
- Short, imperative directives ("Use kebab-case for URL paths")
- Distinct headings separating topics
- Bullet points for scanning
- File-scoped rules via `applyTo` globs

**What Copilot also reads:**
- `AGENTS.md` files (nearest in directory tree)
- `.github/CLAUDE.md` and `.github/GEMINI.md` (agent-specific instructions)

**What brief needs to express but cannot:**
- Per-file-glob scoped rules (constraints currently have no `scope` field)
- The Copilot format is simpler than CLAUDE.md; brief's current output is actually closer to what Copilot needs. The main gap is that constraints need file-scope metadata to generate `applyTo` frontmatter for `.instructions.md` files.

**Emit complexity:** Low. Flatten constraints into imperative bullets, emit sacred regions as "Do not modify" directives. A `brief emit copilot` target is straightforward with the current model, though per-file scoping would require model changes.

### Cursor (.cursor/rules/*.mdc)

**Legacy format:** `.cursorrules` in project root. Plain text/Markdown, 12,000 character limit.

**Modern format:** `.cursor/rules/*.mdc` files with frontmatter:
```yaml
---
description: API conventions for this project
globs: ["src/api/**/*.ts"]
alwaysApply: false
---
```

**Activation modes:**
- `alwaysApply: true` -- always in system prompt (like CLAUDE.md)
- `alwaysApply: false` with `globs` -- applied when editing matching files
- `alwaysApply: false` without `globs` -- model decides based on `description`
- Manual activation via `@rule-name` in chat

**What Cursor benefits from:**
- Focused, composable rules (one file per concern)
- Concrete naming conventions and architectural patterns
- Framework-specific guidelines
- Rules under 500 lines each

**What brief needs to express but cannot:**
- Multiple rule files from a single brief (Cursor's model is one-rule-per-file, not one-document)
- File-glob scoping for constraints
- Rule activation mode (always vs. conditional vs. manual)
- Description metadata per rule group

**Emit complexity:** Medium-high. A single `.brief.md` maps poorly to Cursor's multi-file model. A `brief emit cursor` would need to either: (a) emit a single `.cursorrules` file (legacy, simpler), or (b) split constraints into multiple `.mdc` files by scope/category. Option (b) requires constraint scoping metadata that doesn't exist in the model.

### Windsurf (.windsurf/rules/*.md)

**Format:** Markdown files in `.windsurf/rules/` with YAML frontmatter:
```yaml
---
trigger: always_on | model_decision | glob | manual
globs: ["**/*.py"]  # required for trigger: glob
---
```

**Trigger modes:**
- `always_on` -- included in every message's system prompt
- `model_decision` -- model sees description, reads full content when relevant
- `glob` -- applied when editing matching files
- `manual` -- activated via `@rule-name`

**Character limits:** 12,000 per workspace rule file, 6,000 for global rules.

**What brief needs to express but cannot:**
- Trigger mode metadata (same gap as Cursor)
- File-glob scoping (same gap)
- Multiple rule files from one brief

**Emit complexity:** Medium. Very similar to Cursor's model. Windsurf's frontmatter is simpler (just `trigger` and optional `globs`). A `brief emit windsurf` could emit a single `always_on` rule from the brief contents -- functional but not idiomatic. Idiomatic Windsurf usage splits rules by concern, requiring the same scoping metadata brief lacks.

### Aider (.aider.conf.yml + CONVENTIONS.md)

**Format:** Two-part system:
1. `.aider.conf.yml` -- YAML config for model settings, file handling, API keys
2. `CONVENTIONS.md` -- plain Markdown with coding conventions, loaded via `read: CONVENTIONS.md` in the config or `/read CONVENTIONS.md` per-session

**Convention file format:** Simple bullet lists of natural language preferences:
```markdown
- Prefer httpx over requests for making HTTP requests.
- Use types everywhere possible.
```

**What brief needs to express but cannot:**
- Aider's config is primarily operational (model selection, API keys, file lists), not instructional. The `model` frontmatter field maps to Aider's `model:` config key.
- Convention content maps well to brief's constraints. The gap is small.

**Emit complexity:** Low. `brief emit aider` would produce a `CONVENTIONS.md` from constraints and a minimal `.aider.conf.yml` from frontmatter (model, context files as `read:` entries). This is achievable with the current model.

---

## 3. The "Write Once, Emit Everywhere" Reality Check

### Where convergence works

All five ecosystems share a common core that maps well to brief's model:

- **Technology context** (stack) -- universally useful, expressed similarly everywhere
- **Hard constraints** -- every system benefits from "do not do X" directives
- **Sacred/protected regions** -- expressible as "do not modify" in all formats
- **General coding conventions** -- bullet-list format works everywhere

For a team that wants basic constraint propagation across tools, a single `.brief.md` can produce usable output for all five targets today (with new emitters). The output would be 60-70% as good as hand-crafted per-tool configuration.

### Where targets diverge irreducibly

| Divergence | Affected targets | Why it can't be unified |
|---|---|---|
| **File-scoped rules** | Copilot (.instructions.md), Cursor (.mdc), Windsurf (glob trigger) | Claude Code and Aider don't scope rules to files; Copilot/Cursor/Windsurf do. This is a fundamental structural difference in how rules are applied. |
| **Multi-file vs. single-file** | Cursor (one .mdc per concern), Windsurf (one .md per rule) | Claude Code uses one CLAUDE.md; Cursor/Windsurf use many small files. A brief can't be both without splitting logic. |
| **Activation modes** | Cursor (alwaysApply), Windsurf (trigger modes) | Some rules should be always-on, others conditional. Brief has no concept of rule activation. |
| **Operational config** | Aider (.aider.conf.yml), Claude Code (settings.json) | Model selection, API keys, permission rules are operational, not instructional. Brief's frontmatter partially overlaps but can't express hook configurations or sandbox rules. |
| **Commands and workflows** | Claude Code (CLAUDE.md commands section) | Build/test/lint commands are critical for Claude Code but largely irrelevant for Copilot or Cursor. |
| **Emphasis and enforcement tone** | Claude Code ("IMPORTANT", "YOU MUST") | Claude Code measurably responds to emphasis markers. Other tools may not. Per-target tone adaptation requires either templates or target-aware emitters. |

### Should brief support target-specific overrides?

Yes, but minimally. The recommended approach:

1. **Emit unknown sections in all text-based targets.** This is the lowest-cost, highest-impact change. Authors already use unknown sections for commands, code style, and architecture. Emitting them closes 60% of the CLAUDE.md gap immediately and provides extensibility for other targets.

2. **Add an optional `emit` frontmatter map for per-target metadata:**
   ```yaml
   ---
   stack: [Rust, clap, serde]
   emit:
     cursor:
       split_by: scope  # or "section"
     claude:
       emphasis: true   # add "IMPORTANT" markers to hard constraints
   ---
   ```
   This keeps the brief format clean while allowing authors to tune per-target behavior when needed. Start with 2-3 keys per target, not a comprehensive schema.

3. **Do NOT create per-target content sections.** Something like `## Claude-Only Constraints` defeats the single-source purpose. If content is target-specific, it belongs in the target's native config, not in brief.

---

## 4. MCP Server Integration

### How brief-as-MCP-server changes the integration story

An MCP server would transform brief from a build-time document generator into a runtime query layer. Any MCP-capable agent (Claude Code, Copilot with MCP support, custom agents) could query constraints and sacred regions during execution, not just at session start.

### Tools it would expose

| Tool | Input | Output | Use case |
|---|---|---|---|
| `check_sacred_path` | `{filepath: string}` | `{is_sacred: bool, pattern: string, reason: string}` | Pre-write validation by any agent |
| `get_constraints` | `{type?: "hard"\|"soft"\|"ask_first"}` | `Constraint[]` | Agent queries applicable constraints before acting |
| `get_briefing` | `{}` | Full Brief as JSON | Agent reads complete context on demand |
| `validate_brief` | `{}` | `Diagnostic[]` | CI/automation validates brief health |
| `check_assumption` | `{index: number}` | Assumption details + validation state | Agent checks if assumptions still hold |

### How this relates to the flexibility problem

The MCP server sidesteps the emit fidelity problem entirely for Claude Code. Instead of emitting a perfect CLAUDE.md that captures everything, you configure Claude Code to use the MCP server:

```json
// .mcp.json
{
  "brief": {
    "command": "brief",
    "args": ["mcp-serve"],
    "env": {}
  }
}
```

Claude Code would then query constraints at runtime, check sacred paths before writes, and validate assumptions on demand. The emitted CLAUDE.md becomes a summary for human orientation, not the sole machine interface.

For non-MCP targets (Cursor, Windsurf, Aider), static emit remains necessary. The MCP server is complementary to better emitters, not a replacement.

---

## 5. Claude Code Deep Integration

### Beyond emitting CLAUDE.md text

Claude Code's 2026 feature set provides five integration surfaces beyond CLAUDE.md content:

#### 5.1 Hooks for sacred region enforcement

This is the highest-value deep integration. A `PreToolUse` hook on `Edit|Write` that runs `brief check` would deterministically block writes to sacred regions:

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Edit|Write",
        "hooks": [
          {
            "type": "command",
            "command": "jq -r '.tool_input.file_path' | xargs brief check --file .brief.md 2>/dev/null || exit 0; brief check $(jq -r '.tool_input.file_path') --file .brief.md"
          }
        ]
      }
    ]
  }
}
```

A `brief emit hooks` target (or `brief emit claude-hooks`) could generate this settings.json fragment. This transforms sacred regions from advisory ("Do Not Modify") to enforced (blocked at write time).

Claude Code supports three hook handler types that map to brief's constraint model:
- `type: "command"` -- deterministic checks (sacred path validation)
- `type: "prompt"` -- judgment-based checks (constraint compliance review)
- `type: "agent"` -- deep verification (run tests, check assumptions against codebase)

#### 5.2 Permission rules

Claude Code's `permissions.deny` patterns can enforce sacred regions at the filesystem level:

```json
{
  "permissions": {
    "deny": [
      "Edit(src/auth/**)",
      "Edit(migrations/**)",
      "Write(src/auth/**)",
      "Write(migrations/**)"
    ]
  }
}
```

A `brief emit claude-permissions` target could generate deny rules from sacred entries. This is simpler than hooks and provides the hardest possible enforcement -- Claude cannot even attempt to write to these paths.

#### 5.3 MCP server configuration

As discussed in section 4, brief could emit a `.mcp.json` entry for itself:

```json
{
  "brief": {
    "command": "brief",
    "args": ["mcp-serve"]
  }
}
```

This could be generated by `brief emit claude-mcp` or included in a broader `brief init --claude-code` scaffolding command.

#### 5.4 Skills

The `brief emit skill --install` command already generates and installs `.claude/skills/<name>/SKILL.md` files. This is the most mature deep integration. Skills are loaded on demand (not every session), which makes them appropriate for task-specific briefs while CLAUDE.md carries project-wide context.

One gap: skills support a `disable-model-invocation: true` frontmatter field for workflows with side effects that should only be triggered manually. Brief's skill emitter does not emit this field. Adding a `skill_manual: bool` frontmatter option would close this gap.

#### 5.5 Subagent definitions

Claude Code supports `.claude/agents/*.md` files that define specialized subagents with their own system prompts, tool restrictions, and model preferences. A brief could emit a subagent definition:

```markdown
---
name: brief-reviewer
description: Reviews changes against project constraints
tools: Read, Grep, Glob, Bash
model: haiku
---
You are a constraint compliance reviewer. Check all changes against:
[emitted constraints from brief]

[emitted sacred regions from brief]
```

This is a niche but valuable integration: teams could use `brief emit claude-agent` to generate a reviewer subagent that knows the project's constraints.

#### 5.6 Settings.json directives (not recommended)

Brief could theoretically emit full `settings.json` fragments covering permissions, hooks, sandbox rules, and model preferences. However, this creates a maintenance hazard: teams already configure settings.json for many purposes, and brief-generated fragments would conflict with manually-managed settings. The hooks and permissions integrations described above are better delivered as standalone snippets that teams paste into their existing settings, not as complete settings files.

---

## 6. Concrete Recommendations (Ranked by User Impact)

### Tier 1: Fix what's broken (high impact, low effort)

**R1. Emit unknown sections in the Claude emitter.**
The parser already preserves `unknown_sections`. The Claude emitter ignores them. This is a bug, not a feature request. Adding ~10 lines to `emit_claude()` to render unknown sections as `## <heading>\n<content>\n\n` immediately makes the emitter extensible. Authors can add Commands, Code Style, Architecture, Gotchas, or any other section to their `.brief.md` and have it appear in the emitted CLAUDE.md.

Effort: ~30 minutes. Impact: Closes 50-60% of the CLAUDE.md fidelity gap.

This same fix should be applied to all text-based emitters (prompt, agents_md, skill).

**R2. Emit context files as `@` references in the Claude target.**
Change `- \`./docs/architecture.md\`` to `See @docs/architecture.md for reference.` in the Claude emitter. This activates Claude Code's file-read behavior, making context files actually functional rather than decorative.

Effort: ~15 minutes. Impact: Context files become actionable.

**R3. Add emphasis markers to hard constraints in the Claude target.**
Change hard constraint rendering from `- No breaking changes` to `- **IMPORTANT:** No breaking changes`. Anthropic's own documentation confirms emphasis markers measurably improve adherence.

Effort: ~10 minutes. Impact: Better constraint compliance.

### Tier 2: New emit targets (high impact, medium effort)

**R4. Add `brief emit copilot` target.**
Emit a `.github/copilot-instructions.md` with constraints as imperative directives, sacred regions as "Do not modify" rules, and stack/context as orientation. This is the simplest new emitter because Copilot's format is plain Markdown with no special structure.

Effort: ~2 hours (including tests). Impact: Covers the largest single user base of AI coding tools.

**R5. Add `brief emit cursor` target (legacy format).**
Emit a `.cursorrules` file. Use the legacy single-file format, not the multi-file `.mdc` format, because brief's model does not support per-rule scoping. This gets Cursor users 70% of the value with minimal complexity. The `.mdc` multi-file format can follow later when the constraint model gains scope metadata.

Effort: ~2 hours. Impact: Covers millions of Cursor DAUs.

**R6. Add `brief emit windsurf` target.**
Emit a single `.windsurfrules` file (legacy format) or a single `always_on` rule in `.windsurf/rules/`. Very similar to the Cursor emitter.

Effort: ~1.5 hours. Impact: Covers growing Windsurf user base.

**R7. Add `brief emit aider` target.**
Emit a `CONVENTIONS.md` file from constraints, plus optionally a `.aider.conf.yml` snippet with `read: CONVENTIONS.md` and `model:` from frontmatter.

Effort: ~1.5 hours. Impact: Covers the CLI-first AI coding audience.

### Tier 3: Deep Claude Code integration (high impact, medium-high effort)

**R8. Add `brief emit claude-hooks` to generate sacred region enforcement hooks.**
Generate a settings.json `hooks` fragment with a `PreToolUse` hook that runs `brief check` on every `Edit|Write` tool call. This transforms sacred regions from advisory to enforced.

Effort: ~3 hours. Impact: Deterministic sacred region enforcement.

**R9. Add `brief emit claude-permissions` to generate deny rules.**
Generate a `permissions.deny` array from sacred entries. Simpler and harder-to-bypass than hooks.

Effort: ~1 hour. Impact: Hardest possible sacred enforcement.

**R10. Add a `## Commands` known section to the brief format.**
Add `Commands` as a recognized H2 section (alongside Constraints, Sacred, Assumptions, Deliverable). Parse command entries as name-command pairs. This is the single most impactful format addition for CLAUDE.md parity.

```markdown
## Commands
- Build: `cargo build`
- Test: `cargo test`
- Lint: `cargo clippy`
- Format: `cargo fmt`
```

Effort: ~3 hours (parser + model + all emitters). Impact: Fills the #1 missing CLAUDE.md content type.

### Tier 4: Format evolution (medium impact, higher effort)

**R11. Add optional `scope` field to constraints for per-file targeting.**
This unlocks idiomatic Cursor .mdc and Copilot .instructions.md generation, plus Windsurf glob-triggered rules. Without it, those targets are limited to single-file legacy formats.

Effort: ~1 day (model change, parser change, all emitters). Impact: Required for idiomatic multi-file target emission.

**R12. Add `brief sync` command to emit all detected targets at once.**
Detect which AI tools are configured in the project (look for `.cursor/`, `.windsurf/`, `.github/`, `.claude/`) and emit to all applicable targets from a single command.

Effort: ~4 hours. Impact: The "write once, emit everywhere" promise made concrete.

---

## Summary

The brief tool's core format and parser are sound. The immediate problems are:

1. The Claude emitter discards unknown sections, throwing away the format's built-in extensibility.
2. Context files are rendered as passive lists instead of functional `@` references.
3. Four major AI tooling ecosystems (Copilot, Cursor, Windsurf, Aider) have no emit targets.
4. Claude Code's hooks and permissions systems could enforce sacred regions deterministically, but brief does not generate these configurations.

Recommendations R1-R3 can be implemented in under an hour and close the majority of the CLAUDE.md fidelity gap. Recommendations R4-R7 each take 1-2 hours and multiply brief's addressable user base by 5-10x. Recommendations R8-R10 add Claude Code-specific deep integrations that transform brief from advisory documentation into enforced project policy.

The "write once, emit everywhere" vision is realistic for 70-80% of content. The remaining 20-30% (file-scoped rules, activation modes, operational config) will always require per-target tuning. The right design keeps the brief format simple and pushes target-specific logic into the emitters, with optional `emit:` frontmatter hints for cases where authors need per-target control.
