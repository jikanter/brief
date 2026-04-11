# Claude Code Backend

**Status:** Shipped. `brief emit claude`, `brief emit claude --install`, and `brief emit skill --install` are implemented. Phase 2 work on hooks integration (P2) and `--install` enhancements (P5) is captured in [phase2-synthesis.md](../../../analysis/phase2-synthesis.md). This document captures Claude-specific integration surfaces *beyond* what's in the live roadmap.

## Integration surfaces brief uses today

| Surface | File / location | Status |
|---|---|---|
| Project instructions | `CLAUDE.md` (repo root) | Shipped — `--install` injects between `<brief:generated>` markers |
| Skills | `.claude/skills/<name>/SKILL.md` | Shipped — `brief emit skill --install` |
| Hooks | `.claude/settings.json` `hooks.PreToolUse` | Planned (P2) — `--install --hooks` |

## Integration surfaces brief does **not** use today

### Hierarchical / monorepo CLAUDE.md discovery

Claude Code composes multiple CLAUDE.md files at runtime:
- The closest `.claude/CLAUDE.md` to the current working directory
- Parent-directory `CLAUDE.md` files up the tree (monorepo support)
- `~/.claude/CLAUDE.md` for user-global instructions

`brief emit claude --install` currently assumes a single CLAUDE.md at the repo root. In a monorepo, this may not be the correct injection target — a sub-project may have its own CLAUDE.md that the agent reads in preference.

[open-question] In a monorepo, where should `--install` write — the repo root, the closest CLAUDE.md to `pwd`, the closest CLAUDE.md to the `.brief.md` file, or all of the above? Phase 2 synthesis P5 doesn't address this.

[open-question] Should `--install` walk up looking for an existing CLAUDE.md and inject there, or always create one at the repo root? Either choice has failure modes.

### PreToolUse hook handler types

Phase 2 synthesis P2 documents `type: "command"` hooks (deterministic shell-out). The integration audit asserts Claude Code also supports two other handler types:

- `type: "prompt"` — judgment-based check (model evaluates whether the tool call complies with a constraint)
- `type: "agent"` — deep verification (run tests, validate assumptions, fuller analysis before allowing the tool call)

If real, these unlock an enforcement ladder specifically aligned with brief's three-tier taxonomy:
- `command` for Hard constraints and sacred regions (deterministic suppression)
- `prompt` for Soft conventions (judgment-based compliance review)
- `agent` for Ask First flows (deep verification before proceeding)

[open-question] Do `type: "prompt"` and `type: "agent"` PreToolUse hook handlers actually exist in current Claude Code? The audit cited them but did not verify. Phase 2 synthesis P2's worked example only uses `command`. Resolving this is a prerequisite to designing the full enforcement ladder.

### Skill frontmatter — full schema and recommendations

**Verified 2026-04-11** against [code.claude.com/docs/en/skills](https://code.claude.com/docs/en/skills). Brief's skill emitter currently writes only `name` and `description` (see [src/emit/skill.rs](../../../../src/emit/skill.rs)). The current Claude Code skill schema is considerably richer.

#### Full SKILL.md frontmatter reference

| Field | Purpose | Brief should consider |
|---|---|---|
| `name` | Optional; defaults to directory name. Lowercase + hyphens, max 64 chars. | Already emitted |
| `description` | Recommended. Truncated at 250 chars in listings — front-load keywords. | Already emitted |
| `argument-hint` | Autocomplete hint, e.g. `[issue-number]`. | No natural source in brief today |
| `disable-model-invocation` | Bool. Removes the skill from Claude's context entirely so the model never sees it. User can still invoke via `/skill-name`. See semantics table below. | Strong candidate — natural fit for skills with side effects |
| `user-invocable` | Bool, default `true`. If false, hides the skill from the `/` menu but keeps it in model context (model can still invoke it). | Niche; only useful for "background" skills |
| `allowed-tools` | Space-separated string or YAML list. Pre-approves tools when this skill is active. | Strong candidate — maps to brief's notion of tool constraints |
| `model` | Per-skill model override. | Brief already has a `model` frontmatter field — can pass through |
| `effort` | `low`/`medium`/`high`/`max`. Overrides session effort. | No natural source in brief today |
| `context` | Set to `fork` to run in a subagent. | Niche |
| `agent` | Subagent type when `context: fork` (e.g. `Explore`, `Plan`). | Niche, paired with `context: fork` |
| `hooks` | Skill-scoped hooks. | Connects to phase2-synthesis P2 hooks work — could share an emit path |
| `paths` | Glob patterns that gate auto-activation. | **Strongest candidate** — maps directly to brief's sacred/glob vocabulary, gives Cursor-like scoping for free on Claude Code |
| `shell` | `bash` or `powershell` for inline shell-injection blocks. | Niche |

There is no `version` field in the current schema. Brief does not currently emit one — no action needed, but worth knowing if anyone proposes adding one.

#### `disable-model-invocation` semantics (verified)

Setting `disable-model-invocation: true`:

- **Removes the skill's description from Claude's context entirely.** The model has no awareness the skill exists. Contrast with default behavior, where skill descriptions are loaded into context so Claude knows what's available, but full skill content only loads when invoked.
- **Does not prevent user invocation.** The skill is still runnable via `/skill-name`. When invoked by the user, the full skill content is loaded into context at that point.
- **Is not the same as `user-invocable: false`**, which is the inverse: Claude can invoke, user cannot (hides from `/` menu but keeps the skill in model context).
- **Permission-layer note:** `user-invocable: false` only hides menu visibility; it does not block the Skill tool. To actually block programmatic model invocation, use `disable-model-invocation: true` (or a `Skill(name)` deny rule in `/permissions`).

Context-loading truth table:

| Frontmatter | User invokes | Model invokes | In context |
|---|---|---|---|
| default | yes | yes | description always; body on invoke |
| `disable-model-invocation: true` | yes | no | nothing until user invokes |
| `user-invocable: false` | no | yes | description always; body on invoke |

#### When to set `disable-model-invocation`

Canonical use case from Anthropic's docs: **workflows with side effects or where timing matters** — "like `/commit`, `/deploy`, or `/send-slack-message`. You don't want Claude deciding to deploy because your code looks ready."

The docs categorize this as the **"task content"** pattern (as opposed to "reference content"): step-by-step action playbooks the user wants to trigger explicitly. Examples shown in the docs: `deploy`, `commit`, `fix-issue`, and any skill with `allowed-tools` that grants write/mutation permissions (`Bash(git commit *)` etc.).

**Rule of thumb:** if running the skill unexpectedly would be bad (mutates state, costs money, notifies humans, takes a long time), set `disable-model-invocation: true`.

#### Recommendations for brief

The deep-dive turned up four concrete proposals, in priority order:

1. **`skill_manual: bool` → `disable-model-invocation: true`.** Cheap, well-founded, directly answers the original audit question. Authors who set `skill_manual: true` get a skill that won't be auto-invoked by the model. Default false (preserves current behavior).
2. **`paths: [...]` passthrough.** Brief already has glob vocabulary (sacred regions). Exposing `paths` in skill emit gives users Cursor-like activation scoping on Claude Code without any format growth. This is the highest-leverage addition because it overlaps with the [open-questions.md](../../../open-questions.md) `[format]` scoped constraints discussion — `paths` may be a partial answer for the Claude backend specifically.
3. **`allowed-tools` from a new frontmatter field.** Brief's three-tier constraint taxonomy doesn't include "tools allowed" today. A new optional `allowed_tools: [Bash, Edit, Read]` field on `.brief.md` frontmatter could pass through to the skill emitter and (eventually) the hooks emitter. Adds a frontmatter field, so flag for the YAGNI question.
4. **`model:` passthrough.** Brief already has `model` in frontmatter. The skill emitter should pass it through when present. Trivial.

These are recommendations only — none are implemented. They should be considered alongside the broader skill emitter design when phase2-synthesis P5's "unified install" work picks up.

**Sources:**
- [Extend Claude with skills — Claude Code docs](https://code.claude.com/docs/en/skills)
- [Agent Skills open standard](https://agentskills.io) (referenced by the Claude Code docs as the base spec that Claude Code extends)

### Other settings.json keys

Beyond `hooks`, `.claude/settings.json` carries `permissions.allow` / `permissions.deny` (tool-call patterns, **not** file-path globs — see [obsolete-features.md](../../../analysis/archive/obsolete-features.md) item 1) and other configuration. Brief deliberately does not touch `permissions` for sacred regions because the patterns there match tool invocations, not file paths. Hooks are the correct surface.

[open-question] Are there other settings.json keys brief should consider configuring as part of `--install --full` (phase2-synthesis P5)? The current plan covers CLAUDE.md, skills, hooks. Anything else worth touching?

## Idiomatic register

Claude Code measurably responds to imperative / RFC-2119 language (NEVER, MUST, IMPORTANT). This is the basis for the phase2-synthesis P0 NEVER/MUST/PREFER/STOP reframing. The Claude backend should use the most aggressive version of this register; other backends should not (see [emit-quality-refinements.md](../../../analysis/emit-quality-refinements.md) §3).

## Marker format

`brief emit claude --install` uses XML-style markers:

```
<brief:generated>
...injected content...
</brief:generated>
```

Earlier versions used HTML-comment markers (`<!-- brief:start --> / <!-- brief:end -->`), but Claude Code strips HTML comments from CLAUDE.md before the model reads it, which rendered briefings invisible to the agent. Legacy markers are still recognized on read and migrated to the new format on the next `--install`. See [design-decisions.md](../../../design-decisions.md) "Augment, Not Replace" for the full reasoning.
