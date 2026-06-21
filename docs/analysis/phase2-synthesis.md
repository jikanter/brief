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

### P0: Emit Quality — Constraint Framing and Section Ordering (effort: 1-2 days) — **SHIPPED**

**Status (2026-06-12): shipped.** The `claude` and `prompt` emitters now adopt the polarity classifier shipped with P6 (`src/framing.rs`), closing the item P6 explicitly deferred. Zero format changes; emit-time only.

- **Register.** `claude` and `prompt` hard constraints are framed by polarity (`NEVER:` prohibitions / `MUST:` requirements / plain conventions — the refinement over uniform NEVER), soft constraints as `PREFER:`, ask-first as `STOP and confirm with the user before:`. The `**IMPORTANT:**` prefix is gone. The descriptive targets (`cursor`/`copilot`/`windsurf`/`aider`) and the conversational `aider` register are deliberately untouched — imperative framing is wrong for those ecosystems (see emit-targets-reference divergence matrix).
- **Sacred framing.** Both targets now lead the sacred block with the preamble "must not be modified under any circumstances … STOP and report the conflict" — removing the exception ambiguity and giving the model an action instead of pure suppression.
- **Section ordering.** `prompt` is aggressively reordered for attention dynamics: hard constraints + sacred regions take the **primacy** position, goal/stack/context and the softer tiers sit in the middle, and the deliverable takes the **recency** position. `claude` uses the compromise order (goal first, then constraints ahead of reference material) since CLAUDE.md is read by humans too.

Framing helpers (`frame_hard`/`frame_soft`/`frame_ask_first`) are unit-tested in `src/framing.rs`; emit output is pinned in `src/emit/claude.rs`, `src/emit/prompt.rs`, and `tests/emit_tests.rs`.

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

### P1: `validate-diff` — CI-Enforceable Sacred Regions (effort: 2-3 days) — **SHIPPED**

**Status (2026-06-08): shipped.** `brief validate-diff [--base <ref>] [--stdin] [--json]` is implemented in `src/validate_diff.rs` (pure, unit-tested core) and wired in `src/main.rs`. Default base is `HEAD`; `--stdin` reads newline-separated paths for hook/CI use; `--json` emits a machine-readable report. Exits non-zero when any changed file matches a sacred region. Integration tests in `tests/validate_diff_cli.rs`.

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

### P2: Hooks Integration — Deterministic Sacred Region Enforcement (effort: 1-2 days) — **SHIPPED**

**Status (2026-06-08): shipped.** Two pieces, both in `src/hooks.rs` (pure, unit-tested) + wired in `src/main.rs`:
- `brief check --hook` reads a PreToolUse event on stdin (`tool_input.file_path`), relativizes it against the brief's base dir, and on a sacred match prints the deny decision `{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":…}}`. Always exits 0 so the JSON decision governs and a hook crash never blocks unrelated edits; non-file events and non-sacred paths emit nothing.
- `brief emit claude --install --hooks` injects the CLAUDE.md section *and* idempotently registers the `Edit|Write` PreToolUse hook in `.claude/settings.json`, preserving any existing settings. `--hooks` implies `--install` and is claude-only.

Integration tests in `tests/hooks_cli.rs`. The original sketch below (passing the path as an argument) was superseded by the real stdin protocol confirmed against current Claude Code.

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

### P3: Context Window Budget Awareness (effort: 1 day) — **SHIPPED**

**Status (2026-06-10): shipped.** Implemented in `src/budget.rs` (pure, unit-tested) and wired into `brief emit` in `src/main.rs`. The module treats the emitter like a model compiler's codegen: `estimate_tokens` is the size report (tokenizer-free ~4-chars-per-token heuristic — no network, no model files), `compact` is an optimization pass over the Brief IR (drops stack/context/assumptions/identity/unknown sections, keeps goal + constraints + sacred + deliverable; emitters then render the reduced IR through their normal path so each target's register survives), and per-`Target` thresholds are the linker budget. New `emit` flags:

- `--compact` — emit only the load-bearing essentials.
- `--budget` — print `budget: ~N tokens, M chars (budget T)` to **stderr** (stdout stays clean for piping). Opt-in so existing pipelines are unaffected.
- `--max-tokens <N>` — override the warn threshold.

Over-budget always warns to stderr (even without `--budget`): default token budgets are 500 for `prompt`, 2000 for the file targets, none for `json`. Windsurf additionally carries a 12,000-char ecosystem cap; brief warns on overrun and never silently truncates. Integration tests in `tests/budget_cli.rs`.

Every token of brief output displaces conversation history, code context, or tool output. This prevents the slow degradation of agent performance as briefs grow. A bloated brief consuming 3,000 tokens in the system prompt gets re-injected every turn.

### P4: Cross-Ecosystem Emit Targets (effort: 2-4 days total) — **SHIPPED**

**Status (2026-06-10): shipped.** All four targets land. `cursor` shipped earlier (`src/emit/cursor.rs`). This phase adds the remaining three, each tuned to its ecosystem's idiomatic register rather than reusing the Claude imperative voice (see the per-backend docs under `docs/design/backends/`):

| Target | Emitter | Install destination | Register | Idempotency |
|--------|---------|---------------------|----------|-------------|
| `copilot` | `src/emit/copilot.rs` | `.github/copilot-instructions.md` | descriptive (Requirements/Preferences) | `<brief:generated>` marker injection (file is hand-edited) |
| `windsurf` | `src/emit/windsurf.rs` | `.windsurf/rules/brief.md` | descriptive; `trigger: always_on` frontmatter | brief-owned file, overwritten (mirrors cursor) |
| `aider` | `src/emit/aider.rs` | `CONVENTIONS.md` **+** `.aider.conf.yml` | conversational ("Prefer:" / "Ask before:") | CONVENTIONS.md via markers; conf via idempotent YAML merge |

The original synthesis budgeted all three as "trivial wrappers." The per-backend audit corrected `aider`: a complete integration is two files — `CONVENTIONS.md` plus an `.aider.conf.yml` carrying `read: CONVENTIONS.md` (and `model:` from frontmatter when present, never overwriting a user's choice) so the conventions auto-load each session. `merge_aider_conf` performs the idempotent merge (promotes an existing scalar `read` to a list, extends an existing list, leaves a satisfied config untouched). Unit tests in each emitter module; CLI integration tests in `tests/p4_cli.rs`.

Path-scoped output (`copilot`'s `.github/instructions/*.instructions.md`, `windsurf`/`cursor` per-glob rules) remains deferred until brief grows a format-level scoped-constraints concept — see [open-questions.md](../open-questions.md).

### P5: `--install` Enhancements (effort: 2-3 days) — **SHIPPED**

**Status (2026-06-12): shipped.** Three claude-only additions to the `emit` command, all built on the marker plumbing in `src/emit/markers.rs`:

- **`--position top|bottom|after:<heading>`** — controls where a *first* install lands. `bottom` (default) appends; `top` prepends with the reconciliation preamble "The following task-specific constraints supplement the project instructions below."; `after:<heading>` inserts inside the named `## <heading>` section. On a re-install (markers already present) the section is replaced in place and position is ignored — idempotency is preserved. New marker helpers `inject_section_at` / `remove_sections`.
- **`--full`** — unified install (implies `--install`): the CLAUDE.md section, plus the skill if the brief declares a `skill_name` (via the same source-stamping path as `skill emit --install`, so it carries the `metadata.brief.source` ownership marker), plus the sacred-region PreToolUse hook, plus `permissions.allow` entries derived from the project's `## Commands` (`src/commands.rs` → `Bash(<cmd>:*)`, a starting allowlist the user can prune).
- **`--uninstall`** — reverses what brief installed: strips the CLAUDE.md section (current + legacy markers), removes the sacred-region hook from `.claude/settings.json` (`hooks::remove_pretooluse_hook`), and removes the installed skill **via P7's ownership-checked `skill uninstall`** — never a blind `rm`, so a hand-edited skill is protected.

`permissions.allow` and hook-removal merges are in `src/hooks.rs` (pure, unit-tested). Integration tests in `tests/p5_cli.rs`.

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

### P6: Emit Quality Refinements (effort: ongoing) — **SHIPPED**

**Status (2026-06-11): shipped.** Four refinements land, all additive. Their shared primitive is `src/framing.rs` — the prompt-engineering canonicalization pass — and each refinement is framed against the model-compiler analogy from P3/P4.

**Constraint framing / polarity** (`src/framing.rs`). Rather than the roadmap's blunt "prefix every hard constraint with MUST + 'do not use alternatives'", the shipped version implements the sharper refinement from [emit-quality-refinements.md](emit-quality-refinements.md) §1–§2: hard constraints are classified by polarity and routed to the matching verb — **prohibition** → `NEVER:` (folding the leading negation in), **requirement** → `MUST:`, **convention** ("use thiserror") → left plain so a standing decision is not dressed as adversarial. This is the codegen canonicalization step.

*Scope decision (deliberate):* framing is applied to the **`prompt`** target (highest-privilege register — straight into an API system prompt) and the new **`anchor`** output. It is **not** wired into the `claude`/`agents-md` emitters (their human-readable `**IMPORTANT:**` register and tests stand) nor the descriptive targets (`cursor`/`copilot`/`windsurf`/`aider`, whose idiom is explicitly non-imperative). Reframing the `claude` target is the unshipped **P0** "constraint language reframing" item; `framing.rs` is the reusable mechanism it will adopt when taken up.

**Constraint conflict detection** (`src/conflict.rs`). On `--install` into a host file with existing content (`CLAUDE.md`/`AGENTS.md`/`.github/copilot-instructions.md`/`CONVENTIONS.md`), `detect_conflicts` warns when a brief constraint contradicts an existing instruction. This is the linker's symbol-redefinition warning: it fires only on **same subject** (shared content token, polarity words excluded) **and opposite polarity** (exactly one side is a prohibition), which keeps false positives low. brief's own `<brief:generated>` region is excluded so re-installs never flag the brief against itself. Warns; never blocks. (cursor/windsurf are brief-owned overwrites — no host content to conflict with — so they are skipped.)

**Constraint specificity validation** (`src/validate.rs`). `brief validate` flags vague constraints as a lint-pass **Warning** (never an error): vague = under 8 words **and** no concrete reference (backtick span, path, `::`, version/digit, or `snake_case`/`CamelCase` identifier). A short-but-concrete constraint ("no `unsafe`") is not flagged; a long sentence is not flagged.

**Attention anchoring** (`src/emit/anchor.rs`, `brief emit anchor`). Emits a compact, deliberately lossy `<brief:anchor>` block (≤5 lines: polarity-framed hard constraints, then ask-first items, then a one-line sacred summary) suitable for periodic re-injection by an agent framework to counter attention decay over long sessions. Soft constraints, assumptions, and prose are dropped on purpose.

Tests: unit suites in each new module + `tests/p6_cli.rs`.

*Deferred (tracked, not in this slice):* per-target tone presets ([emit-quality-refinements.md](emit-quality-refinements.md) §3) and command-executable `which` validation (§6 — pairs with `brief init` command auto-scaffolding, which does not exist yet).

### P7: Skill discovery, scaffold, and install/uninstall — hand-editable across the boundary (effort: 3-5 days) — **SHIPPED**

**Status (2026-06-11): shipped.** The forced primitive — the brief-owned / user-owned region boundary inside a SKILL.md — is encoded in the library module `src/skill.rs`, and all four commands ride on it:

- `brief skill search <query>` — local-only, no network. Ranks `name` + `description` substring matches across `.claude/skills/`, `.agents/skills/`, and `~/.claude/skills/` (name match outweighs description). Exit 0 on a hit, 1 on none.
- `brief skill scaffold [--description "<text>"] [--from-brief <file>] [--name <slug>]` — spec-compliant skeleton with `metadata.brief.source` stamped, plus empty `scripts/` + `references/`. Source precedence: `--from-brief` → `--description` → the active `.brief.md`. **The skill-first `--description` path is included** (the deliberate step the spec flagged — taken, per the broader "writes to multiple surfaces" framing in design-decisions.md).
- `brief skill install <path>` — syncs into `.claude/skills/<name>/`. Idempotent across the boundary: a re-install updates only `metadata.brief.source` and the `<brief:generated>` body fence; all other destination fields and body content are preserved byte-for-byte. The frontmatter edit is **surgical** (line-level, not a YAML re-serialize) so user formatting/comments/key-order survive.
- `brief skill uninstall <name> [--force]` — the canonical removal surface. Refuses to remove a skill lacking brief's ownership marker (`metadata.brief.source`) without `--force`.

**v1 scope decisions (deliberate):** `init`/`validate`/`emit` are unchanged (scaffold does not replace `init`); spec validation moved into `src/skill.rs` so the lib and CLI share one implementation (the bin `skill_validate.rs` re-exports). `uninstall`'s "no edits outside brief-owned regions" check is gated on the ownership marker only — finer edited-body detection would need a stored baseline hash, which did not pass the YAGNI bar. **P5's future `brief emit claude --uninstall` must call `brief skill uninstall` for skill cleanup** rather than `rm`-ing the directory, per the boundary this work protects.

Unit suite in `src/skill.rs` (boundary, scaffold, install/uninstall, search) + CLI integration tests in `tests/skill_p7_cli.rs`.

**The use case.** A user has a skill they can describe in plain English but does not know two things:

1. Whether the skill has already been built somewhere they can reuse — most realistically, somewhere in the local repo or under `~/.claude/skills/`.
2. How to structure the resulting `SKILL.md` so it conforms to the [agentskills.io spec](../reference/standards/agentskills/specification.md) (cached locally on 2026-05-04).

The existing `brief skill emit [--install]` workflow is *brief-first*: it derives a SKILL.md from a `.brief.md`. The user's authoring need is broader: they may want to start from a description string with no brief at all, or from a brief whose `skill_name` / `skill_description` are already set (per [design/frontmatter-additions.md](../design/frontmatter-additions.md)). P7 makes scaffold accept either, and — more importantly — makes the resulting SKILL.md hand-editable, the same way `brief emit claude --install` lets users hand-edit CLAUDE.md around its fenced region.

**The metadata boundary (read this before designing).** The cached spec defines exactly six recognized frontmatter fields: `name`, `description`, `license`, `compatibility`, `allowed-tools`, and `metadata`. The `metadata:` map is explicitly carved out as the extension point for clients: *"Clients can use this to store additional properties not defined by the Agent Skills spec."* That sentence is brief's seam.

| Region | Owner | brief's behavior |
|---|---|---|
| `name`, `description`, `license`, `compatibility`, `allowed-tools` | Spec / user | Write initial values during scaffold. Never overwrite on re-install. |
| Markdown body, `scripts/`, `references/`, `assets/` | User | Write skeleton during scaffold. Never overwrite on re-install. |
| `metadata.brief.source` | brief | A pointer back to the originating `.brief.md` (or the literal `--description` argument). The single load-bearing key — it is what re-install reads to find the skill again. Other `metadata.brief.*` keys are out of scope until a concrete consumer demands them, per the YAGNI discipline in [design/frontmatter-additions.md](../design/frontmatter-additions.md). |
| Optional fenced body region (`<brief:generated>` / `</brief:generated>`) | brief | If brief injects body content, fence it with the same XML-style markers that `brief emit claude --install` migrated to in 2026-04-11 (HTML comments are stripped from CLAUDE.md/SKILL.md before the agent sees them — see [design-decisions.md](../design-decisions.md) "Augment, Not Replace"). Legacy `<!-- brief:start -->` / `<!-- brief:end -->` markers are recognized on read for migration. Content outside the fence is the user's. |

This makes the SKILL.md a true co-edit surface: the user can rename, rewrite, or restructure anything outside `metadata.brief.source` and the optional brief fence, and `brief skill install` re-runs are non-destructive.

**Proposed commands.**

| Command | Purpose |
|---|---|
| `brief skill search <query>` | **Discovery, local-only in v1.** Match `query` against `name` + `description` across configured local skill roots (`.claude/skills/`, `.agents/skills/`, `~/.claude/skills/`). Print ranked hits with paths. Exit 0 if any hit, 1 if none. No network access — consistent with the project's "no network calls" rule in CLAUDE.md. A future entry can add public-registry search if and when that constraint is relaxed deliberately. |
| `brief skill scaffold [--description "<text>"] [--from-brief <file>] [--name <slug>]` | **Structuring.** Generate a spec-compliant skeleton. Source precedence: `--from-brief` (uses `skill_name` / `skill_description` from `.brief.md` frontmatter) → `--description` literal → the active `.brief.md` if one is in scope. Derive a kebab-case `name`, validate it against the spec rules already in `src/skill_validate.rs`, write `SKILL.md` with `metadata.brief.source` stamped, and create empty `scripts/` + `references/`. This subsumes `brief skill init` for new skills. |
| `brief skill install <path>` | **Install.** Sync a skill directory into the active skills root (default `.claude/skills/<name>/`). Idempotent: re-running replaces only `metadata.brief.source` and any fenced body region. Hand edits to other fields and to body content outside the fence are preserved byte-for-byte. |
| `brief skill uninstall <name>` | **Uninstall, the canonical surface for skill removal.** Remove the installed skill if it carries the `metadata.brief.source` ownership marker and has no edits outside brief-owned regions; otherwise warn and refuse without `--force`. **P5's planned `brief emit claude --uninstall` must call into this command for skill cleanup rather than removing the skill directory directly** — P5 as currently drafted does an unconditional `rm` of the skill dir, which would clobber hand edits that this boundary is meant to protect. |

**Why this is one work item.** Discovery, scaffold, and install/uninstall share the same load-bearing primitive: a clean separation of brief-owned regions from user-owned regions inside a SKILL.md. Until that separation is encoded, none of the three operations can be re-run safely on a hand-edited skill. Forcing the metadata-boundary decision once — rather than three times — is the whole point of bundling them.

**Out of scope.**

- Generating body *instructions* from anything other than a literal description string or an existing `.brief.md`. Body authorship is the user's job; brief only owns the metadata boundary and the optional fenced region.
- Public-registry search (e.g., `anthropics/skills`). The agentskills.io quickstart calls that repo "Example skills," not a curated registry, and pulling it would require network access — see the discovery row above. Track separately if/when needed.
- Semantic / LLM-powered search. v1 is substring + keyword match.
- Cross-product install targets beyond `.claude/skills/`. The existing `brief skill emit --install` target is the one to mirror first.
- Additional `metadata.brief.*` keys (e.g., `brief.version`, `brief.installed-at`). They were considered, did not pass the YAGNI bar — `brief.source` alone is what re-install needs to locate the skill again. Add later only when a concrete consumer demands one.

**Connection to existing work.** Builds directly on `src/skill_init.rs` (slug derivation — likely folded into `scaffold`), `src/skill_validate.rs` (the spec checks already in place), `src/emit/skill.rs` (body templating), and the marker-replacement logic in `src/emit/claude.rs` lines 5-6, 113-154 (active markers + legacy migration). The `--from-brief` source mode lets P7 reuse `brief skill emit`'s body generator wholesale; the `--description` mode is the genuinely new path.

**Mission framing (worth flagging).** P7 introduces a *skill-first* authoring path (`scaffold --description`) that does not require a `.brief.md` at all. This is a step beyond brief's stated mission ("CLI that reads `.brief.md` files and emits to targets") and toward the broader framing in [design-decisions.md](../design-decisions.md) ("a tool that writes to multiple integration surfaces"). The decision to take that step should be made deliberately, not absorbed silently as part of P7. If the team prefers to keep brief strictly brief-first, drop `--description` and require `--from-brief`; the rest of P7 still stands.

**Status.** Proposed. The boundary itself is forced — any of the three commands needs it — so it is the first sub-task regardless of which command ships first. P5 must be revised to call P7's `brief skill uninstall` rather than unconditionally removing the skill directory.

### P8: Scoped constraints — format-level path scoping (effort: 3-5 days) — **SHIPPED**

**Status (2026-06-21): shipped.** Resolves the highest-leverage open format question ([open-questions.md](../open-questions.md) `[format]` Scoped Constraints) in favor of **Direction 1** (format-level scope, not emit-only hints). A constraint is now `Constraint { text, scope: Vec<Glob> }` in `src/model.rs`; an empty scope is project-wide, preserving the historical model. The type carries `From<&str>`/`Deref<Target=str>`/`Display`/`AsRef<str>`/`PartialEq<&str>` so the eleven existing emitters compiled against the new type with near-zero churn, and `is_directory_prefix()` exposes the directory-prefix vs. arbitrary-glob distinction the emitter needs.

- **Authoring syntax** (`src/parse/body.rs`). A constraint list item may carry a leading bracket of comma-separated globs: `` - [`src/ui/**`, `src/api/**`] text ``. Globs are backtick-wrapped (the sacred-path convention) so Markdown does not fold `**` into bold emphasis; `[]` with no globs is literal text.
- **Native glob fan-out — `cursor`** (`src/emit/cursor.rs`). `emit cursor --install` writes the always-apply `brief.mdc` bundle (project-wide constraints only) **plus** one `brief-<slug>.mdc` per distinct scope, each with native `globs:` frontmatter and `alwaysApply: false` (the split-by-scope pattern the research note identified — one rule file per scope, since every glob-frontmatter format is file-level). brief owns the `brief*.mdc` namespace; re-install sweeps orphaned scoped files. stdout stays a single lossless bundle.
- **Non-destructive prose — all other emitters** (`src/framing.rs::with_scope`). Targets with no native per-rule glob home prefix the rendered line with `When working in `glob`, `glob`: …` so scope is never silently dropped. `xml` instead emits a structured `scope="…"` attribute per `<rule>`. JSON serializes the `scope` array (omitted when empty).
- **Validation** (`src/validate.rs`). `brief validate` warns (never errors) when a scope glob matches no files; directory-prefix scopes fall back to a directory-existence check because the `glob` crate does not expand a trailing `/**` to a directory's direct children.

Unit suites in `src/model.rs`, `src/parse/body.rs`, `src/framing.rs`, `src/emit/cursor.rs`, `src/emit/json.rs`, `src/validate.rs` + CLI integration tests in `tests/p8_cli.rs`.

*Deferred (tracked, not in this slice):* native fan-out for `copilot` (`applyTo`), `windsurf` (`trigger: glob`), and Claude `.claude/rules/` `paths:`. The cursor emitter proves the split-by-scope mechanism; the remaining glob-frontmatter targets reuse it when taken up. The emitter-trait refactor (one model, N serializers) is the natural home for that work — see the Long-term table.

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
