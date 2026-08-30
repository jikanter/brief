# Design Doc — `brief` as Intent Artifact with Eridian + Hermes Emit Targets

**Status:** Draft v2 (corrected to the real `brief` model)
**Author:** Jordan Kanter (Sr Mgr Data Eng, DCR/EDAA)
**Repo:** `https://github.com/jikanter/brief`
**Related systems:** `golden-ai`/Eridian (aichat fork), Hermes (Nous Research agent), `personal-llm-functions`, `pi`
**Last updated:** 2026-08-30

> **What changed from v1:** v1 modeled `brief` as a `parse → plan → dispatch` tool that called aichat `/v1`. That was wrong. The real `brief` is an **emit** tool: `.brief.md` → idiomatic config for each runtime (`CLAUDE.md`, `AGENTS.md`, `.cursor/rules/brief.mdc`, `.github/copilot-instructions.md`, `.windsurf/rules/brief.md`, `CONVENTIONS.md`), installed **non-destructively and idempotently**, with a token **budget** reporter and **Sacred** path-scoping. This doc replaces "dispatch" with "emit" everywhere.

---

## 1. Problem statement

`brief` is a structured briefing format for AI coding agents: a human-writable `.brief.md` (YAML frontmatter + Markdown body) plus a CLI that **validates, composes, and emits** the briefing into each agent runtime's native register — non-destructively and idempotently. It already emits to Claude Code, OpenAI Codex/`AGENTS.md`, Cursor, Copilot, Windsurf, and Aider.

The temptation was to build a `brief` **runtime** (read a brief, call a model, run a tool loop). That would clone Hermes and add a fourth executor to a stack that already has Eridian (composer), Hermes (hub), and `personal-llm-functions` (tool single-source). 

**The real gap is narrower and cleaner:** `brief` emits to six *external* ecosystems but not to *your own* stack. Eridian roles and Hermes registries are exactly the kind of idiomatic target `brief` already knows how to produce.

## 2. Decision

**`brief` stays a compiler-of-intent. We add two emit targets, not a runtime and not a dispatcher.**

- **`brief emit eridian`** → an Eridian **role artifact** in its idiomatic register (composable role that Eridian's composer consumes).
- **`brief emit hermes`** → a Hermes-consumable **registry/seed config** (the hub loads it; tools flow *into* Hermes).

`brief` never calls a model, never owns a session, never runs a tool loop. It produces the artifact; Eridian composes, Hermes orchestrates. This is the same "one declarative source → idiomatic outputs for many consumers" pattern `brief` already embodies and that `personal-llm-functions` uses (one header → `functions.json` **and** Hermes registry).

## 3. Non-goals (explicit YAGNI list)

- ❌ No `brief` runtime / inference loop.
- ❌ No `brief` dispatcher calling aichat `/v1` (that was the v1 mistake).
- ❌ No `brief`-owned session/state store (Hermes owns sessions).
- ❌ No MCP server inside `brief`.
- ❌ No RAG (consistent with Eridian's "contextual engineering").

Emit produces **static artifacts**; execution stays entirely in Eridian/Hermes.

## 4. Design principles

1. **Emit, don't execute.** `brief`'s only job is `validate → compose → emit`.
2. **Idiomatic register per target.** An Eridian role must look like a hand-written Eridian role; a Hermes registry like a hand-written Hermes registry. Same discipline as `emit cursor` producing `.cursor/rules/brief.mdc`.
3. **Non-destructive + idempotent.** Re-emitting reconciles in place; `--uninstall` reverses cleanly. Matches existing `emit claude --install/--uninstall`.
4. **Sacred is honored, not translated away.** Path-scoped Sacred regions (P8 format-level path scoping) must survive into Eridian/Hermes emissions as enforceable constraints, not prose.
5. **Budget-aware.** Eridian/Hermes emissions must participate in `--budget` and `--compact`.
6. **Single source of intent.** The `.brief.md` is canonical; every target is a projection of it.

## 5. Architecture

```
                         .brief.md  (YAML frontmatter + Markdown body)
                         canonical intent · Sacred regions · constraints
                                        │
                                 brief  (validate → compose → emit)
                                        │
        ┌───────────────┬───────────────┼───────────────┬───────────────┐
        ▼               ▼               ▼               ▼               ▼
   emit claude     emit cursor     emit copilot     emit eridian     emit hermes
   CLAUDE.md    .cursor/rules/… .github/copilot… Eridian role      Hermes registry
   (existing)     (existing)      (existing)      (NEW)             (NEW)
                                                      │                 │
                                                      ▼                 ▼
                                              Eridian composer     Hermes hub
                                              (roles-call-roles)   (sessions, MCP,
                                                                    tools flow in)
```

**Boundary contract (unchanged from your adopted plan):**
- Eridian = pure composer/tool. `brief` writes a role file; nothing reaches back into `brief`.
- Hermes = the hub. `brief` writes a registry/seed; Hermes owns the loop.
- `brief` = the compiler. Owns the `.brief.md` format + emit projections only.

## 6. The `.brief.md` format (as it actually exists)

Markdown with YAML frontmatter. Frontmatter carries machine facts; the body is the reviewable intent. Observed structure:

```markdown
---
stack: [Python 3.12, PostgreSQL 16]
context: [./docs/architecture.md]
---

# Redesign event pipeline for 10M events/day   # H1 = Goal

## Constraints
### Hard
- v2 API backward compatibility
### Soft
- Prefer async patterns
### Ask First
- Database schema changes

## Sacred
- `src/auth/**` — Proprietary tenant resolution   # P8 path-scoped, enforceable

## Assumptions
- [ ] Bottleneck is synchronous DB writes

## Deliverable
Architecture doc + implementation plan + working code
```

**Formalizing this into a schema is E1** (you'll supply the informal schemas; I'll produce the formal one). Known elements to capture: frontmatter keys (`stack`, `context`, and any others in your fixtures), the H1-as-goal convention, the `Constraints{Hard,Soft,Ask First}` tiers, `Sacred` path globs, `Assumptions` checkboxes, `Deliverable`.

## 7. The emit contract (per target)

Every emit target — including the two new ones — must satisfy the same five guarantees the existing targets already meet:

1. **Idiomatic register.** Output looks native to the target ecosystem.
2. **Idempotent install.** `--install` reconciles in place; repeated runs are no-ops.
3. **Reversible.** `--uninstall` removes exactly what was installed (section/hooks/skill).
4. **Non-destructive.** Never clobbers hand-authored content outside `brief`'s managed region.
5. **Budget-reportable.** Participates in `--budget` (stderr token/char estimate) and `--compact`.

### 7.1 `brief emit eridian` (NEW)
- **Output:** a composable Eridian **role** in its idiomatic register.
- **Mapping:** Goal (H1) → role objective; `Constraints.Hard/Soft/Ask First` → role guardrails; `Sacred` globs → enforceable no-touch constraints in the role; `stack`/`context` → role context block (contextual engineering, not RAG); `Deliverable` → role output contract.
- **Idempotent install target:** the role file location Eridian loads from (reconcile in E2).

### 7.2 `brief emit hermes` (NEW)
- **Output:** a Hermes-consumable **registry/seed config** the hub loads.
- **Mapping:** same intent projection, but framed for a session seed; `Sacred` → constraint the hub enforces on tool actions; `stack`/`context` → seed context.
- **Trust boundary caveat:** if Hermes server mode transitively re-exports loaded tools (issue #342), an emitted registry may widen the tool surface — track and document (E3 spike).

## 8. Sacred / path-scoping across targets (P8)

Sacred regions are the highest-value, hardest-to-fake constraint. In Claude/Cursor/Copilot emissions they're framed in the target's rule register; for Eridian/Hermes they must become **enforceable** constraints (no-touch globs the composer/hub respects), not just descriptive prose. `brief check <path>` must return consistent Sacred verdicts regardless of which target consumed the brief.

## 9. Budget & compaction

Eridian/Hermes emissions must:
- Report estimated tokens/chars under `--budget` (to stderr), like existing targets.
- Honor `--compact` (strip reference prose, keep essentials) and `emit anchor` (≤5-line NEVER/MUST constraint anchor) so a role/registry can be re-injected cheaply mid-session.

## 10. CI/CD positioning — differentiator vs raw connector

A raw Claude/Codex connector step embeds an **imperative, invisible** prompt. A `.brief.md` is a **declarative, checked-in, diffable** intent artifact that emits identical constraints into *every* runtime your CI touches. The differentiator:

- **One source, many runtimes.** CI can `brief emit claude`, `brief emit eridian`, `brief emit copilot` from the same file — no per-tool prompt drift.
- **Reviewable intent.** A PR shows the `.brief.md` diff, not six divergent config edits.
- **Enforceable Sacred.** `brief validate` + `brief check` gate the pipeline on constraint integrity before any agent runs.
- **Budget gate.** `--budget` can fail CI if an emission blows the context window.

You'd reach for a `brief`-emitted Eridian role in CI (vs raw connector) **when the same intent must stay consistent across multiple agent runtimes and honor Sacred path scoping** — that consistency is the thing a raw connector can't give you.

## 11. Failure modes & mitigations

| Failure | Mitigation |
|---|---|
| Emit drifts into dispatch/runtime | Non-goals §3 lint-enforced; no model-call HTTP client in `brief` |
| Eridian/Hermes emission not idempotent | Reuse existing install/reconcile/uninstall harness; golden idempotency tests |
| Sacred degrades to prose in new targets | Sacred→enforceable-constraint conformance test per target |
| Hermes transitively re-exports tools (#342) | Spike + documented trust boundary before relying on it |
| Emission blows context budget | `--budget` participation required for new targets |
| Format churn breaks emissions | Formal schema (E1) + schema-version gate |

## 12. Rollout plan

1. **Phase 0 — Formal schema.** Turn the real `.brief.md` into a formal, versioned schema (you supply informal schemas). Unblocks validation + emit mapping.
2. **Phase 1 — `emit eridian`.** Produce an idiomatic, idempotent Eridian role; prove Sacred survives as enforceable constraint.
3. **Phase 2 — `emit hermes`.** Produce a hub-consumable registry/seed; run the #342 spike.
4. **Phase 3 — Budget/compact/anchor parity** for both new targets.
5. **Phase 4 — CI/CD + docs.** GitHub Action, differentiator demo, `eridian-trace`, README/AGENTS.md updates.

## 13. Open questions

1. Full frontmatter key set beyond `stack`/`context` (from your fixtures) → E1 schema.
2. Eridian role file format/location `brief` should write to (idiomatic register + install path).
3. Hermes registry/seed format the hub expects.
4. Does Hermes server mode transitively re-export loaded tools (#342)? → trust boundary for `emit hermes`.
5. How should Sacred globs be expressed as enforceable constraints in Eridian vs Hermes?
