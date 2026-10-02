# Design Doc — brief's integrated-stack targets

**Status:** Draft v3
**Author:** Jordan Kanter
**Repo:** `https://github.com/jikanter/brief`
**Related systems:** aichat ([jikanter/aichat-private](https://github.com/jikanter/aichat-private)), astrophage, llm-functions, rocky, pi; Hermes through aichat only
**Last updated:** 2026-10-02

> **What changed from v2 (2026-08-30).** v2 planned two new targets, `emit eridian` and `emit hermes`. Since then aichat has named brief its primary composition language, shipped a metadata refinery that owns the aichat ↔ Hermes boundary, versioned every Entity with a `lock_hash`, and added `tool_approval` rules to role frontmatter. v3 follows:
> - **`emit hermes` is cut.** aichat's refinery moves a role's description into Hermes' `profile.yaml` and deliberately moves nothing else. A brief target would bypass that design.
> - **`emit eridian` becomes the aichat role emitter.** Roles first; the agent-directory emitter is deferred until agent formats settle ([backends/aichat](backends/aichat/README.md)).
> - **`emit promptfoo` shipped** (aichat Phase 48A/B, brief PRs #1–#3).
> - **brief's own trace spans are dropped.** aichat's trace is the only telemetry source in the stack.
> - **brief's intent is broader than a compiler:** it compiles, installs and gates, and a brief can be the source of a reusable definition, not only a task ([../design-decisions.md](../design-decisions.md) "Intent").
> - **2026-10-02: brief mints an id.** `emit json` carries `id`, `emit jsonl` ships, and `emit promptfoo` forwards the id (§7.5, aichat Phase 77F). New rule: every emitter has a JSONL output for training (§4).

---

## 1. Problem statement

brief emits to every major coding agent, but the integrated stack consumes only one of its outputs. aichat's own architecture lists the brief seam as "emitted role files and fixtures"; today brief supplies the fixtures half (promptfoo providers for astrophage cassettes) and no role. aichat's integration idea #7, "one brief → three artifacts" (a role, an eval suite and cassette bindings), has one of its three parts.

There are also two gaps where brief's contracts don't yet reach the stack: Sacred regions cannot become enforced rules in aichat, and `context:` cannot point at anything but a local file.

## 2. Decision

brief stays emit-only. For the integrated stack it has, in order:

| Target | Consumer | Status |
|---|---|---|
| `emit promptfoo` | promptfoo, then astrophage | Shipped |
| `brief.id`, `emit jsonl` (§7.5) | aichat's label table, rocky | Shipped |
| aichat role (§7.1) | aichat | Next |
| Eval suite (§7.3) | aichat role evaluation or promptfoo | Waiting on aichat's #7 decision |
| aichat agent (§7.4) | aichat, llm-functions | Deferred |

brief never calls a model, owns a session, runs a tool loop, or calls the network. It writes files; aichat and astrophage do the work.

## 3. Non-goals

The full list is in [../design-decisions.md](../design-decisions.md) "Non-goals". The ones this plan could drift into:

- ❌ A `brief` runtime, or dispatch to aichat's `/v1`.
- ❌ A Hermes target of any kind.
- ❌ ACE tag filing or curation.
- ❌ brief-owned telemetry.
- ❌ Test-case authoring as a new brief grammar before aichat picks an eval format.

## 4. Design principles

1. **Emit, don't execute.** brief's jobs are compile, install and gate.
2. **Idiomatic register per target.** An emitted role must look like a hand-written aichat role: short and conversational, not a CLAUDE.md rules block.
3. **Non-destructive, idempotent, reversible.** Re-emitting reconciles in place; `--uninstall` removes only what brief wrote.
4. **Deterministic.** The same brief gives byte-identical output. aichat hashes a role into its `lock_hash`, so any drift reads as a new Entity version.
5. **Sacred is honored.** Where a consumer can enforce a path rule, brief emits one; where it cannot yet, brief says so rather than pretending prose is enforcement.
6. **Single source of intent.** The `.brief.md` is canonical; every target is a projection of it.
7. **Every emitter has a JSONL output for training.** Owner rule, 2026-10-02, relayed with aichat's Phase 77F request. `emit jsonl` is the first instance. What the rule means for a target whose output is not the brief's objective (the role emitter, §7.1) is settled when that target is built (G9).

## 5. Architecture

See [../architecture/README.md](../architecture/README.md): where brief sits, every seam, the internal pipeline, and the invariants.

## 6. The `.brief.md` format

Unchanged by this plan: [../brief-format.md](../brief-format.md), with the machine contract in [../schema/SPEC.md](../schema/SPEC.md). The role emitter needs no new frontmatter field (§7.1 names the role without one).

## 7. The emit contract

Every target meets six guarantees: idiomatic register, idempotent install, reversible, non-destructive, budget-reportable, and deterministic.

### 7.1 The aichat role emitter (next)

**Output:** one file, `roles/<name>.md`: YAML frontmatter plus a prompt body, the Role preset of aichat's Entity model.

**Proposed CLI:** `brief emit aichat` prints the role; `--install` writes it into aichat's `roles/` directory; `--uninstall` removes it. A future agent emitter would be a flag on the same target, not a new one.

**Naming the role without a new field:** `--name <slug>`, defaulting to the brief's file stem (`review.brief.md` → `review`), then to a slug of the goal. `skill_name` is not reused: it names a skill, not a role.

**Mapping** (adapted from the [backend audit](backends/aichat/README.md) mapping table):

| Brief | Role | Notes |
|---|---|---|
| H1 goal | `description:` and the body's opening line | `description` is what aichat's refinery carries to Hermes, so it should be contrastive, not a summary |
| `model` | `model:` | Only when the value is `client:model`; otherwise omitted so aichat's default applies |
| Hard / Soft / Ask First | Body, in the terse register (the skill emitter's voice) | Not the `NEVER:`/`MUST:` rules block |
| Sacred | Body "Do not modify" lines; `tool_approval.deny` once aichat can match a path (§8) | |
| `stack`, `context` | A short context preamble in the body | Roles have no `documents:` field |
| Deliverable | The body's closing paragraph | |
| Unknown sections | Passed through as body sections | Same as every target |
| `cassettes` / `## Fixtures` | Not in the role | They belong to `emit promptfoo` |

**Must preserve** aichat's placeholders untouched: `__INPUT__`, `{{__tools__}}`, `{{__os__}}` and the other built-ins, `{{<var>}}`, `{{$AICHAT_*}}`, and `{{#if}}` / `{{#unless}}` blocks.

**Must not** read the clock. `skill::now_utc` stamps skill install metadata and must never reach this emitter.

**To settle in the first spike (E2-S1), against aichat's current source:**
- How aichat resolves its config directory on each platform, and whether `$AICHAT_CONFIG_DIR` is still honored.
- Whether role frontmatter tolerates an unknown key, so brief can mark the file as its own (as `metadata.brief.source` does for skills). If not, ownership is tracked by the file matching brief's output, and `--uninstall` refuses a file that has been edited.
- Whether an emitted role and a hand-written one of the same content give the same `lock_hash`.

### 7.2 `emit promptfoo` (shipped)

Cassette bindings (`cassettes:`, `## Fixtures`) compile to promptfoo providers aimed at astrophage, with `--gateway` and `--session-id`. Spec: [../brief-format.md](../brief-format.md) §8.3.

### 7.3 Eval suite (waiting)

The third artifact of aichat's idea #7. aichat has not chosen the format: its own role evaluation or promptfoo tests. brief should not grow a test-case grammar before that choice, and any new field still has to clear the [YAGNI bar](frontmatter-additions.md). Unknown `##` sections already carry draft test cases to any consumer through `emit json`.

### 7.4 aichat agent (deferred)

An agent is a directory whose definition lives in llm-functions, and it needs multi-file emit support brief does not have. It waits until agent formats settle across the industry. The audit in [backends/aichat](backends/aichat/README.md) stays as the reference.

### 7.5 `brief.id` and `emit jsonl` (shipped)

aichat exports agent trajectories for training (`aichat --trace-train atif`) and a label table keyed by id. brief supplies the objective those turns were working toward, so brief mints the key: a content hash of `goal`, `deliverable` and `constraints`. The contract, with a golden pair for aichat to mirror, is [../schema/SPEC.md](../schema/SPEC.md) §7.

| Piece | What it does |
|---|---|
| `emit json` | Carries `id`. |
| `emit jsonl` | One line per brief: `id` plus the objective. The stream aichat's label table and rocky join on `brief_id`. |
| `emit promptfoo` | Sends `x-brief-id` on every provider, and names `AICHAT_BRIEF_ID` for CLI providers in its header comment. brief emits no CLI provider, so there is no provider to set the variable on. |

aichat records the id (`session.start.brief_id`, ATIF `extra.brief.id`) and does not carry the goal; the goal is joined back by id. That half is aichat's (A6).

## 8. Sacred across targets

`brief check <path>` is the one verdict every target must agree with. Enforcement today:

| Target | Enforcement |
|---|---|
| Claude Code | `PreToolUse` hook running `brief check` (deterministic) |
| Cursor | `preToolUse` hook |
| CI | `brief validate-diff` |
| Other coding agents | Text only |
| aichat role | **Text only, until aichat changes.** A `tool_approval` rule's argument glob matches a tool's `command` argument or its only string argument. A file-writing tool takes a path *and* content, so no rule can say "deny writes under `src/auth/**`". aichat needs a way to name the argument, such as `fs_write(path=src/auth/**)`. |

## 9. Budget & compaction

The role emitter reports under `--budget` and honors `--compact`. A role is a system prompt, so it takes the tight `prompt` threshold (500 tokens), not the 2,000 of file-based targets.

## 10. CI/CD positioning

A raw connector step embeds an invisible prompt; a `.brief.md` is a checked-in, diffable source that emits the same constraints into every runtime CI touches. With the role emitter, the same file also becomes a versioned aichat Entity: a PR shows the brief diff, and the Entity's `lock_hash` changes exactly when the emitted role does.

## 11. Gaps

**In brief:**

| # | Gap | Where it's tracked |
|---|---|---|
| G1 | No aichat role target | E2 |
| G2 | `context:` treats every entry as a local file: `validate` warns on URLs and `ace://` pointers, and `emit claude` renders them as `@` imports | E1-S6 |
| G3 | No CI workflow in the repo: the schema drift test, format check and runtime guardrail run only locally | E5-S1 |
| G4 | The runtime guardrail checks four crate names in `Cargo.toml`, not imports or a session store | E5-S5 |
| G5 | Execution-type frontmatter keys are described as out of format but not rejected | E1-S5 |
| G6 | An unknown `brief_version` fails with the generic exit code 1 | E1-S2 |
| G7 | No shared conformance or golden idempotency harness across targets | E4 |
| G8 | The Phase 48C round-trip demo (brief → record → commit → replay → drift) is unbuilt | E5-S2 |
| G9 | The JSONL rule (§4.7) has one instance, `emit jsonl`. The role emitter's JSONL form is undefined | E2 |
| G10 | Providers carrying `x-brief-id` have not been run through a real `promptfoo eval`; the two shapes without it were | E5-S2 |

Fixed during this review: an install matched a `<brief:generated>` marker anywhere in the host file, including inside prose or a code block, and could delete hand-written content. A marker now counts only on its own line.

**In sibling repos (documented here, applied there):**

| # | Gap | Repo |
|---|---|---|
| A1 | `tool_approval` cannot match a named argument, so Sacred cannot become a path rule (§8) | aichat |
| A2 | Idea #7's eval-suite format is undecided (§7.3) | aichat |
| A3 | Phase 62's seam conformance kits do not include the brief → aichat role seam | aichat |
| A4 | aichat's docs still describe brief's cassette field as deferred (integrated-architecture index) and PR #1 as unmerged (Phase 48, roadmap ledger) | aichat |
| A5 | ACE / ClaudeKb is not in aichat's ownership table, although brief points at it | aichat |
| A6 | Read `X-Brief-Id` (`--serve`) and `AICHAT_BRIEF_ID` (CLI), record `session.start.brief_id`, export ATIF `extra.brief.id`; mirror the golden pair in SPEC.md §7.4 (Phase 77F) | aichat |
| A7 | Confirm the gateway ignores `x-brief-id` when matching a cassette, and decide whether it copies the header into its trace or forwards it upstream | astrophage |

## 12. Failure modes & mitigations

| Failure | Mitigation |
|---|---|
| Emit drifts into a runtime | Non-goals, plus the guardrail test (E5-S5) |
| Emitted role is not byte-stable | Determinism test; a `lock_hash` conformance check against aichat (E2-S3) |
| Role reads like a CLAUDE.md rules block | Terse register from the skill emitter; review against hand-written roles (E2-S1) |
| Sacred presented as enforced when it is prose | §8 table stated in the emitted role's docs; `tool_approval` only after A1 |
| aichat changes its role format | The E2-S1 spike pins the surface; a conformance fixture fails when it moves |
| An install deletes hand-written content | Line-anchored markers (fixed); golden idempotency tests (E4-S2) |

## 13. Rollout

1. **Now:** URI-aware `context:` (G2), a CI workflow (G3), the wider guardrail (G4).
2. **Role emitter:** the E2-S1 spike, then `emit aichat` with install, uninstall and a `lock_hash` check.
3. **Sacred in aichat:** after A1 lands in aichat.
4. **Eval suite:** after A2 is decided.
5. **Deferred:** the agent emitter.

## 14. Open questions

1. The target's name: `emit aichat` (recommended, with an agent flag later) or `emit aichat-role`.
2. How brief marks an installed role as its own, if role frontmatter rejects unknown keys (§7.1).
3. The eval-suite format (A2).
4. Whether ACE pointers need a typed form (`pin`, `retrieve`) or a plain `ace://` URI in `context:` is enough. Plain URIs first; a typed field has to clear the YAGNI bar.
