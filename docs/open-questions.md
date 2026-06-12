# Open Questions

Unresolved questions about how `brief` should evolve. **Not** roadmap items — questions the team still owes itself answers to before building.

Each question is held to the same bar: it is *not* answered by [analysis/phase2-synthesis.md](analysis/phase2-synthesis.md), [analysis/archive/obsolete-features.md](analysis/archive/obsolete-features.md), or [design-decisions.md](design-decisions.md). Items explicitly decided in those docs (composition/inheritance, `environment` frontmatter, Commands first-class parsing, subagent generation, MCP server, etc.) are settled and not reopened here.

## Tag conventions

Questions are tagged by scope:

- `[format]` — concerns the `.brief.md` format itself: schema, frontmatter, sections, parser model
- `[emit]` — concerns the emit layer (renderer, framing, ordering) but not per-backend
- `[deep-dive-needed]` — flagged for fuller investigation before answering; the team needs more information than the question itself contains

Backend-specific open questions live inline in [design/backends/](design/backends/) under each backend's README, tagged inline as `[open-question]`. They are not duplicated here.

---

## `[format]` Scoped Constraints

**Question:** Should constraints support per-path scoping?

**The problem.** The current model treats `Constraints { hard, soft, ask_first: Vec<String> }` as globally applicable to the whole project. In practice, constraints often apply to a subset: "WCAG 2.1 AA compliance" applies to UI code, not to migration scripts. "All public functions must return `Result<T, AppError>`" applies to the library surface, not to tests. Authors currently paper over this by either stating the scope in the constraint text (hoping the model picks up on it) or accepting that the constraint is imprecise.

**Why it's newly relevant.** Phase 2 synthesis P4 plans to emit to Cursor, whose `.cursor/rules/*.mdc` format natively carries per-rule `globs`. Copilot's `applyTo` frontmatter does the same. If brief has no scoping concept, the emit for these targets is forced to `alwaysApply: true` for every rule — which throws away one of the main features of those ecosystems. See [design/backends/cursor/README.md](design/backends/cursor/README.md) and [design/backends/copilot/README.md](design/backends/copilot/README.md) for the per-backend forcing functions.

**Two directions:**

1. *Format-level.* Add optional scope to the constraint model. A constraint becomes `(text, scope: Option<Vec<Glob>>)`. Authoring syntax could be `- [src/ui/**] WCAG 2.1 AA compliance` or a subsection `#### For src/ui/**` under `### Hard`. Either way it's a real parser change and a real model change, and the downstream emitters need to know what to do with an unscoped default.
2. *Emit-only via `emit:` hints.* Keep the format flat, use the proposed `emit:` frontmatter map (see [analysis/emit-quality-refinements.md](analysis/emit-quality-refinements.md) §5) to let users hint per-target scoping when they need it. Cheaper, preserves the "authoring stays simple" principle, but doesn't solve the general case — the scope exists only in emit metadata, not in the brief's semantic model.

**Connection to compiled-intent theory.** [analysis/compiled-intent-theory.md](analysis/compiled-intent-theory.md) level 3 ("constraint manifolds") assumes constraints have a domain of applicability as a first-class concept. Scoped constraints at the format level would be the text-layer foundation that a future embedding layer builds on. This is weak evidence for direction 1 over direction 2.

**Research update (2026-06-12).** Verified the current scoping mechanics of every target ecosystem (per-backend READMEs under [design/backends/](design/backends/), all re-verified 2026-06-12). Three findings reshape the decision:

1. **Two scoping *axes*, not one — the model must serve both:**
   - **Glob-frontmatter scoping** — Cursor `globs`, Copilot `applyTo`, Windsurf `trigger: glob`, **and Claude Code `.claude/rules/*.md` `paths:`**. A rule *file* carries a glob; it activates when editing matching files.
   - **Directory-hierarchy scoping** — nested `CLAUDE.md` / `AGENTS.md`: a config file's *location* scopes it to that subtree (loaded on-demand when Claude reads a file there).
2. **No format scopes multiple rule-sets within one file.** In every glob-frontmatter format the glob is **file-level** — one glob set per file. A scoped emit must therefore **fan out one rule file per distinct scope** (split-by-scope), not embed scope inside one file. The real cost lives in the *emitter*, not the authoring syntax.
3. **Claude Code is no longer prose-only for scopes.** `.claude/rules/*.md` with `paths:` glob frontmatter is a native per-glob instruction surface (context, not enforcement — hooks remain the only blocking layer). A scoped Hard constraint can emit to `.claude/rules/` natively for the *primary* target instead of degrading to "When editing X:" prose. This materially raises the payoff of format-level scoping.

**The directory-prefix distinction (the thing to get right).** A scope that is a clean **directory prefix** (`src/api/**`) has a native home in *both* axes — it's the glob for Cursor/Copilot/Windsurf/`.claude/rules`, *and* it can emit into `src/api/CLAUDE.md` via the hierarchy. An **arbitrary glob** (`**/*.test.ts`) only has a home in the glob-frontmatter axis. The model should expose `is_directory_prefix()` so the emitter can pick hierarchy vs. glob-file vs. prose. (AGENTS.md has *no* glob frontmatter — scoping only via nested placement; and Claude Code does **not** read AGENTS.md natively — it needs a `@AGENTS.md` import or a `CLAUDE.md` symlink.)

**Implication for the two directions.** Emit cost (file fan-out + directory-prefix detection) is identical whether scope is stored format-level (direction 1) or as `emit:` hints (direction 2) — so the deciding factor is authoring semantics, not emit difficulty. Direction 1 (`Constraint { text, scope: Vec<Glob> }`, canonical `Vec<Glob>` with each emitter serializing its own form) now looks stronger: Claude's `.claude/rules` gives the primary target a native scoped home, and the emitter-trait refactor wants to land alongside it (one model, N serializers).

**Status:** undecided, but better-informed. The Cursor emitter already shipped using the `alwaysApply: true` whole-file fallback (no `globs`), so the question is no longer blocking — but it is the highest-leverage format decision remaining. Proposed authoring syntax (flat bracket prefix) is captured in [brief-format.md](brief-format.md) §8.1.

---

## `[format]` Behavioral Instructions as a Fourth Semantic Primitive

**Question:** Is "how the agent should work" a semantic class brief should recognize, distinct from Constraints / Sacred / Goal?

**The distinction.** A constraint says *what* the output must look like: "do not break backward compatibility." A behavioral instruction says *how* the agent should conduct itself: "when you encounter an ambiguous requirement, ask before proceeding", "prefer small focused commits", "explain your reasoning when making non-obvious choices." These read naturally as Soft constraints today, but they're categorically different from "use `Result<T, AppError>`". One describes the work product; the other describes the work process.

**Why it matters for brief specifically.** Brief's three semantic primitives are Goal, Constraint, Sacred. These are all about *the thing being built*. Behavioral instructions are about *the agent's conduct* — the meta-layer. Skills are the closest existing home (they *are* about conduct), but skills aren't always loaded and aren't task-scoped. Stuffing behavioral instructions into Soft constraints works but loses the categorical distinction the author meant.

**A tentative answer.** A reserved unknown-section convention name — `## Workflow` or `## Behavior` — could carry behavioral instructions without promoting them to a parser-level primitive. Unknown section passthrough means the emit already handles them correctly. What's missing is:
1. A documented reserved name so authors know where to put them,
2. Emit-time treatment that prefixes behavioral items appropriately (not NEVER/MUST — something like "WHEN:" or "HOW:"),
3. An authoring guideline in [brief-format.md](brief-format.md) (currently a placeholder) explaining when something belongs in Workflow vs. Constraints.

This is the "reserved unknown-section names" idea from the format-expressiveness archive: treat it as convention, not schema. Capture it when `brief-format.md` is written.

**Status:** undecided, low urgency. Becomes actionable the moment someone sits down to write the format spec and needs to enumerate reserved section names.

---

## `[format]` CLAUDE.md ↔ .brief.md Drift Detection

**Question:** When `--install` injects a brief into CLAUDE.md and the `.brief.md` file subsequently evolves, how does a user know their installed section is stale?

**The setup.** `--install` is idempotent — running it again replaces the `<brief:generated>` block. But nothing reminds the user to re-run it. Over time, CLAUDE.md can carry a brief section that reflects an old version of the source `.brief.md`, and the agent reads stale constraints. Phase 2 synthesis P5 covers the *removal* side (`--uninstall`) and the *injection* position, but not the *drift detection* side.

**Proposed answer.** Extend `brief validate` (or introduce `brief validate --installed`) with a check: if a CLAUDE.md exists in the project and contains a `<brief:generated>` block, re-emit from the current `.brief.md` and compare. If the installed block differs from what would be emitted today, warn. Exit non-zero if `--strict`.

**Why this belongs in validate, not in a new command.** `brief validate` already verifies that sacred paths exist and context files are readable. Checking that the installed block matches the source is the same class of check: is the artifact on disk consistent with the declared state? Piggybacking on `validate` also means CI pipelines that already run `brief validate` pick up the drift check for free.

**Variant:** a pre-commit hook could run the drift check and either auto-reinstall or block the commit. This pairs naturally with the hook work planned under phase2-synthesis P2.

**Status:** undecided. Low urgency until multiple projects have `<brief:generated>` blocks checked in and staleness is observed in practice.

---

## `[format]` Frontmatter-Only Commands for JSON Consumers

**Question:** Does `commands: BTreeMap<String, String>` in frontmatter have value even though `## Commands` first-class parsing was downgraded?

**The setup.** [analysis/archive/obsolete-features.md](analysis/archive/obsolete-features.md) downgraded `## Commands` first-class parsing to optional. The remaining question was whether a frontmatter-only variant — purely for structured JSON access by downstream tools — still has value.

**Deep-dive result (2026-04-11).** The deep-dive produced a reusable YAGNI bar for frontmatter additions, captured both as a reference doc ([design/frontmatter-additions.md](design/frontmatter-additions.md)) and as an invokable skill ([.claude/skills/frontmatter-yagni/SKILL.md](../.claude/skills/frontmatter-yagni/SKILL.md)) that walks future proposers through the checks. Applying the bar to `commands`:

| Check | Result |
|---|---|
| 1. Task-specific | ✗ Build/test commands don't change task-to-task; standing project context belongs in CLAUDE.md |
| 2. No existing carrier | ✗ `## Commands` unknown sections already pass through to all emitters and JSON serializes their content as strings |
| 3. Concrete consumer exists | ✗ No JSON consumer, no MCP server (downgraded), no IDE plugin in flight |
| 4. Shape is obvious | ✗ At least three plausible shapes (flat map, map of objects, list of objects); no consumer to pin down |
| 5. Stays inside mission | ~ Single field is fine; "commands" is a wedge for `environments`, `services`, etc. |
| 6. Current user flow fails | ✗ No user is currently blocked |

**0 of 6 checks pass. Recommendation: defer indefinitely.**

**Trigger to revisit.** All three of:
1. A concrete downstream consumer exists or is being actively built
2. That consumer specifies the shape it needs (resolving the shape question)
3. Auto-detection of commands by `brief init` is judged insufficient because users have a reason to author commands directly in `.brief.md` rather than in their existing tooling (`Cargo.toml`, `Justfile`, `package.json` `scripts`, CLAUDE.md)

**Important finding from the deep-dive.** Brief's JSON emit already serializes `unknown_sections[].content` as strings (see `src/emit/json.rs`). A downstream JSON consumer that wants commands data can read it from the `## Commands` unknown section today — no schema growth required. This significantly weakens the original "structured access requires a typed field" argument and is the load-bearing reason check 2 fails.

**Status:** deep-dive complete; recommendation is defer; final decision still rests with the team. Not yet acted upon — left here for the team to confirm or override before being closed.

---

## `[format]` `[deep-dive-needed]` Raw Markdown Preservation for Known Sections

**Question:** Should Constraints, Assumptions, and Deliverable preserve raw Markdown formatting the way unknown sections now do?

**The current state.** Unknown sections use byte-range raw capture (see `src/parse/body.rs`) and flow through to emitters without formatting loss. Known sections — Constraints, Assumptions, Deliverable — still go through `segments_to_plain_text`, which flattens code spans, links, and emphasis. A constraint like "Use `` `tokio::select!` `` for [this pattern](link)" loses the code span and link.

**Is this a bug or a feature?** Two readings:

- *Feature.* Constraints should be terse imperatives. If a constraint wants a code span or a link, that's probably a sign it's too long and should be rewritten. The flattening enforces terseness.
- *Bug.* Authors will want to reference specific functions, types, or docs in constraints, and having those references rendered as plain text in the emit loses precision. Code spans in particular carry semantic weight that LLMs respond to.

**Why this needs a deep dive.** The right answer depends on whether code-span preservation in constraint text actually moves the needle on agent compliance. There's no a priori way to know — it's an empirical question about LLM tokenization of backticked identifiers. Before extending the byte-range capture to known sections (a small refactor), the team should decide whether the extra precision earns its complexity. Possible deep-dive output: a small experiment emitting the same brief with and without code-span preservation and observing whether Claude's behavior on a constraint like "use `Result<T, AppError>`" changes.

**Status:** undecided, deep-dive pending. Tracked as an explicit follow-up in the working todo list.

---

## `[format]` Cassette / `## Fixtures` Field for Eridian Replay

**Question:** Should `.brief.md` grow a `cassettes: [name]` frontmatter field (or a first-class `## Fixtures` section) binding a role's intent to a pinned eval-replay set?

**The setup.** Eridian's caching sub-track is extracting its wire-level record/replay/cache/mock substrate into a standalone tool, **astrophage** ([github.com/jikanter/astrophage](https://github.com/jikanter/astrophage)), forked out of the aichat runtime. A *cassette* is a pinned, content-addressed, committed set of model interactions that astrophage replays deterministically — offline, token-free — for regression/eval. The cross-repo design that names brief as a possible declaration surface is aichat's [`SPEC-astrophage.md`](https://github.com/jikanter/aichat-private/blob/main/docs/architecture/integrated-architecture/SPEC-astrophage.md) §4 (and its in-repo basis [`SPEC-004-ecosystem-surfaces.md`](https://github.com/jikanter/aichat-private/blob/main/docs/analysis/caching/SPEC-004-ecosystem-surfaces.md) §brief).

**The proposed seam (F6 pattern — brief never runs anything).** An intent author declares, next to the role's constraints/deliverable, that the role's eval replays a named set:

```markdown
## Fixtures
- cassette: rust-reviewer-v1 — pinned eval-replay set for the rust-reviewer role
```

or in frontmatter:

```yaml
cassettes: [rust-reviewer-v1]
```

`brief emit` would compile the binding into the eval harness's replay config — a promptfoo `providerconfig` snippet selecting `--cache-mode cassette --cassette rust-reviewer-v1 --cache-replay`. Brief only **parses and emits a string**; astrophage (invoked by the harness, with aichat as the client) records and replays. This preserves design decision 4 (format-first, no runtime) and the hard "no `tokio`, no `reqwest`" constraint — brief never gains network code.

**Applying the YAGNI bar ([design/frontmatter-additions.md](design/frontmatter-additions.md)):**

| Check | Result |
|---|---|
| 1. Task-specific | ~ A cassette name is role/eval-specific, not standing repo context — leans pass. But the binding may be just as naturally owned by `promptfooconfig.yaml`. |
| 2. No existing carrier | ✗ A `## Fixtures` **unknown section** already passes through every emitter and serializes to `unknown_sections[].content` as a string; an emit target can parse the cassette name from there with no schema growth. |
| 3. Concrete consumer exists | ✗ astrophage is specced, not yet shipped; until its `brief emit` target is built there is no consumer reading a typed `cassettes:` field. |
| 4. Shape is obvious | ~ `cassettes: [string]` is plausible, but record/replay options (record-extend vs hard-fail, per-cassette namespace) may force a map-of-objects; not pinned until astrophage's CLI stabilizes. |
| 5. Stays inside mission | ~ A cassette ref is task intent, but it wedges toward `fixtures`, `eval_config`, `replay_mode` — eval-harness infrastructure that drifts away from brief's mission. |
| 6. Current user flow fails | ✗ No user is blocked: the cassette set can be named directly in `promptfooconfig.yaml` (aichat `SPEC-004` §promptfoo) with zero brief involvement. |

**2 of 6 firmly fail, 3 are soft. Recommendation: defer (optional, do not build now).** Identical disposition to aichat's own `SPEC-004` verdict ("spec a minimal, optional, deferred emit target; apply nothing now"). The `## Fixtures` field is a *convenience* for teams already authoring intent in `.brief.md` who want the cassette reference to live next to the role it pins — not a requirement for the eval-replay story.

**Trigger to revisit.** All three of:
1. astrophage has shipped and exposes a stable cassette-selection CLI (resolves check 4's shape).
2. A `brief emit astrophage` / `brief emit promptfoo` target is being actively built and string-parsing the `## Fixtures` unknown section is judged insufficient (flips check 2/3).
3. A user is authoring the cassette binding in `.brief.md` and finds the unknown-section round-trip lossy or error-prone (flips check 6).

**Cross-repo note.** This is a **brief-repo task** owned here; aichat documents it as a companion change but must never edit `.brief.md` parsing from its own repo. The reverse rule also holds — brief links to aichat via GitHub URL, never a local path.

**Status:** deferred per the YAGNI bar; logged as a companion to aichat `SPEC-astrophage.md`. Not acted upon — awaiting the revisit trigger.
