# `brief` — Linear Delivery Plan (5 Epics) — v2 (Emit Model)

**Tool:** Linear (Projects = Epics, Issues = Stories, Sub-issues = tasks)
**Repo:** `https://github.com/jikanter/brief`
**Design doc:** [new-surfaces-design-doc.md](new-surfaces-design-doc.md) (companion, v2)
**Author:** Jordan Kanter
**Last updated:** 2026-08-30

> **What changed from v1:** `brief` is an **emit** tool, not a dispatcher. Eridian + Hermes become two new **emit targets** (like the existing `emit claude`/`emit cursor`/`emit copilot`), each in its idiomatic register, installed **idempotently and non-destructively**. No runtime, no `/v1` dispatch. Epics are rebuilt around emit, Sacred path-scoping, budget parity, and CI.

> **Linear modeling convention**
> - **Epic → Linear Project** (name, goal, success metric).
> - **Story → Linear Issue** (title, description, acceptance criteria, estimate [Fibonacci], priority, labels).
> - **Labels:** `area:spec`, `area:emit-eridian`, `area:emit-hermes`, `area:validate`, `area:budget`, `area:sacred`, `area:ci`, `area:dx`, `type:feature`, `type:chore`, `type:spike`, `type:docs`.
> - **Milestones** map to design-doc rollout Phases 0–4.

---

## Epic overview

| Epic | Linear Project | Milestone | Goal | Success metric |
|---|---|---|---|---|
| E1 | **Brief Schema Formalization** | Phase 0 | Turn the real `.brief.md` into a formal, versioned schema | Every fixture validates; malformed fixtures rejected with correct codes |
| E2 | **Eridian Emit Target** | Phase 1 | `brief emit eridian` → idiomatic, idempotent Eridian role | UC produces a role Eridian composes; Sacred enforced |
| E3 | **Hermes Emit Target** | Phase 2 | `brief emit hermes` → hub-consumable registry/seed | Hermes loads the emission; #342 trust boundary documented |
| E4 | **Validation, Sacred & Budget Parity** | Phase 3 | New targets honor idempotency, Sacred, budget, compact, anchor | Golden idempotency + Sacred conformance + budget tests pass |
| E5 | **CI/CD, Observability & Docs** | Phase 4 | `brief` in pipelines; differentiator vs raw connector; docs | GitHub Action gates on validate/budget; README de-RAG'd |

---

## E1 — Brief Schema Formalization
**Linear Project.** *Goal:* the real `.brief.md` (frontmatter + body) becomes a formal, versioned schema that drives validation and every emit mapping. *Non-goal:* execution.
*Labels:* `area:spec`, `type:feature` · *Milestone:* Phase 0

### E1-S1 — Formalize `.brief.md` frontmatter + body schema
**Priority:** Urgent · **Estimate:** 5 · **Labels:** `area:spec`, `type:feature`
**Description:** From the informal schemas (to be supplied) + existing fixtures, produce a formal schema: frontmatter keys (`stack`, `context`, others), H1-as-Goal, `Constraints{Hard,Soft,Ask First}` tiers, `Sacred` globs, `Assumptions` checkboxes, `Deliverable`.
**Acceptance criteria:**
- [ ] Formal schema committed (`SPEC.md` + machine schema, e.g. JSON Schema).
- [ ] Every `tests/fixtures` well-formed example validates green.
- [ ] Every malformed fixture fails with the correct error band.

### E1-S2 — Schema versioning + reject-unknown-version
**Priority:** High · **Estimate:** 2 · **Labels:** `area:spec`, `type:feature`
**Description:** Add a schema-version marker and fail fast on unknown/newer versions.
**Acceptance criteria:**
- [ ] Version bump policy documented (additive vs breaking).
- [ ] Unknown version → dedicated non-zero exit code.

### E1-S3 — Sacred path-scoping formalization (P8)
**Priority:** High · **Estimate:** 3 · **Labels:** `area:spec`, `area:sacred`, `type:feature`
**Description:** Formally specify Sacred glob semantics so `brief check <path>` is deterministic and portable across every emit target.
**Acceptance criteria:**
- [ ] Glob semantics + precedence rules documented.
- [ ] `brief check src/auth/handler.rs` returns a stable verdict under test.

### E1-S4 — Constraint-tier semantics (Hard / Soft / Ask First)
**Priority:** Medium · **Estimate:** 2 · **Labels:** `area:spec`, `type:docs`
**Description:** Nail down what each tier *means* to a consuming runtime so emit mappings are consistent.
**Acceptance criteria:**
- [ ] Each tier has a normative definition + expected agent behavior.
- [ ] Mapping guidance for emit targets referenced from E2/E3.

### E1-S5 — Non-goals as schema guardrails
**Priority:** Medium · **Estimate:** 2 · **Labels:** `area:spec`, `type:chore`
**Description:** Ensure the schema can't express self-execution (no model/session/tool-loop fields), enforcing design-doc §3.
**Acceptance criteria:**
- [ ] Schema rejects execution-implying keys.
- [ ] Rationale links design-doc §3.

---

## E2 — Eridian Emit Target
**Linear Project.** *Goal:* `brief emit eridian` produces a composable Eridian role in idiomatic register, installed idempotently. *Non-goal:* dispatching to Eridian.
*Labels:* `area:emit-eridian`, `type:feature` · *Milestone:* Phase 1

### E2-S1 — Define the Eridian role emission format + install path
**Priority:** Urgent · **Estimate:** 3 · **Labels:** `area:emit-eridian`, `type:spike`
**Description:** Determine the idiomatic Eridian role register and the file location Eridian loads from. Answers design-doc open question #2.
**Acceptance criteria:**
- [ ] Documented target format + install path.
- [ ] Sample hand-written role vs emitted role diff is negligible.

### E2-S2 — Implement `brief emit eridian --install`
**Priority:** Urgent · **Estimate:** 8 · **Labels:** `area:emit-eridian`, `type:feature`
**Description:** Map `.brief.md` → Eridian role: Goal→objective, Constraints→guardrails, Sacred→no-touch, stack/context→context block, Deliverable→output contract. Non-destructive, idempotent install.
**Acceptance criteria:**
- [ ] Emits a role Eridian's composer loads and runs.
- [ ] Re-running `--install` is a no-op (idempotent).
- [ ] `--uninstall` cleanly reverses.

### E2-S3 — Sacred → enforceable constraint in Eridian role
**Priority:** High · **Estimate:** 5 · **Labels:** `area:emit-eridian`, `area:sacred`, `type:feature`
**Description:** Ensure Sacred globs become enforceable no-touch constraints in the role, not descriptive prose. Answers open question #5 (Eridian side).
**Acceptance criteria:**
- [ ] Conformance test: role refuses to touch a Sacred path.
- [ ] `brief check` verdict matches role behavior.

### E2-S4 — `--compact` / `emit anchor` support for Eridian
**Priority:** Medium · **Estimate:** 3 · **Labels:** `area:emit-eridian`, `area:budget`, `type:feature`
**Description:** Support compact emission + a ≤5-line NEVER/MUST anchor re-injectable into an Eridian role mid-composition.
**Acceptance criteria:**
- [ ] `brief emit eridian --compact` strips reference prose.
- [ ] `brief emit anchor` produces a valid Eridian-consumable anchor.

### E2-S5 — Phase-0 proof: brief-as-Eridian-role round trip
**Priority:** High · **Estimate:** 3 · **Labels:** `area:emit-eridian`, `type:spike`
**Description:** Emit a real `.brief.md` to an Eridian role and compose it end-to-end; capture whether emit fidelity is sufficient before investing in Hermes.
**Acceptance criteria:**
- [ ] Working example in `examples/`.
- [ ] Go/no-go recorded for E3.

---

## E3 — Hermes Emit Target
**Linear Project.** *Goal:* `brief emit hermes` produces a hub-consumable registry/seed the Hermes hub loads. *Non-goal:* brief owning sessions or tools.
*Labels:* `area:emit-hermes`, `type:feature` · *Milestone:* Phase 2

### E3-S1 — Define the Hermes registry/seed emission format
**Priority:** Urgent · **Estimate:** 3 · **Labels:** `area:emit-hermes`, `type:spike`
**Description:** Determine the Hermes-consumable format (session seed / registry) and where the hub loads it. Answers open question #3.
**Acceptance criteria:**
- [ ] Documented target format + load path.
- [ ] Sample hand-written seed vs emitted seed diff is negligible.

### E3-S2 — Implement `brief emit hermes --install`
**Priority:** Urgent · **Estimate:** 8 · **Labels:** `area:emit-hermes`, `type:feature`
**Description:** Map `.brief.md` → Hermes seed: intent projection framed for the hub; Sacred→enforced tool-action constraint; stack/context→seed context. Non-destructive, idempotent.
**Acceptance criteria:**
- [ ] Hermes hub loads the emission and seeds a session.
- [ ] `brief` holds no session state.
- [ ] Idempotent install + clean uninstall.

### E3-S3 — Sacred → enforceable constraint in Hermes
**Priority:** High · **Estimate:** 5 · **Labels:** `area:emit-hermes`, `area:sacred`, `type:feature`
**Description:** Sacred globs must constrain Hermes tool actions, not just seed prose. Answers open question #5 (Hermes side).
**Acceptance criteria:**
- [ ] Conformance test: hub blocks a tool action on a Sacred path.
- [ ] `brief check` verdict matches hub behavior.

### E3-S4 — Spike: Hermes server transitive tool re-export (#342)
**Priority:** High · **Estimate:** 3 · **Labels:** `area:emit-hermes`, `type:spike`
**Description:** Determine whether Hermes server mode transitively re-exports loaded tools; this governs whether an emitted registry widens the tool surface. Answers open question #4.
**Acceptance criteria:**
- [ ] Documented finding + trust-boundary note (design-doc §11).
- [ ] Go/no-go on relying on server-mode re-export.

### E3-S5 — `--budget` / `--compact` support for Hermes
**Priority:** Medium · **Estimate:** 3 · **Labels:** `area:emit-hermes`, `area:budget`, `type:feature`
**Description:** Hermes emission participates in budget reporting + compaction.
**Acceptance criteria:**
- [ ] `brief emit hermes --budget` reports tokens/chars to stderr.
- [ ] `--compact` produces a slimmer seed.

---

## E4 — Validation, Sacred & Budget Parity
**Linear Project.** *Goal:* the two new targets meet the same guarantees existing targets do — idempotency, Sacred integrity, budget, compact, anchor.
*Labels:* `area:validate`, `area:budget`, `area:sacred`, `type:feature` · *Milestone:* Phase 3

### E4-S1 — Cross-target emit conformance harness
**Priority:** Urgent · **Estimate:** 5 · **Labels:** `area:validate`, `type:feature`
**Description:** A shared test harness asserting all five emit guarantees (idiomatic, idempotent, reversible, non-destructive, budget-reportable) for every target including eridian/hermes.
**Acceptance criteria:**
- [ ] Harness runs against all emit targets.
- [ ] eridian + hermes pass all five guarantees.

### E4-S2 — Golden idempotency tests (install → re-install → uninstall)
**Priority:** High · **Estimate:** 3 · **Labels:** `area:validate`, `type:chore`
**Description:** Golden-file tests proving re-install is a no-op and uninstall restores original state for the new targets.
**Acceptance criteria:**
- [ ] Re-install produces zero diff.
- [ ] Uninstall restores pre-install state byte-for-byte.

### E4-S3 — Sacred conformance suite (portable verdicts)
**Priority:** High · **Estimate:** 3 · **Labels:** `area:sacred`, `area:validate`, `type:feature`
**Description:** Assert `brief check <path>` and each target's enforced behavior agree, across claude/eridian/hermes.
**Acceptance criteria:**
- [ ] Same Sacred verdict across all targets for a fixture set.
- [ ] Divergence fails the build.

### E4-S4 — Budget parity + `--budget` regression gate
**Priority:** Medium · **Estimate:** 3 · **Labels:** `area:budget`, `type:feature`
**Description:** Ensure eridian/hermes emissions report budget consistently; add a regression test that flags budget blow-ups.
**Acceptance criteria:**
- [ ] Budget numbers reported for new targets.
- [ ] Test fails if an emission exceeds a configured ceiling.

### E4-S5 — `validate --install` conflict warnings for new targets
**Priority:** Medium · **Estimate:** 2 · **Labels:** `area:validate`, `type:chore`
**Description:** Extend lint/validate to warn on conflicts when installing eridian/hermes emissions (vague constraints, path collisions).
**Acceptance criteria:**
- [ ] Conflicts surfaced with actionable messages.
- [ ] `--strict` promotes warnings to failures.

---

## E5 — CI/CD, Observability & Docs
**Linear Project.** *Goal:* make `brief`-emitted artifacts first-class in pipelines and prove the differentiator vs a raw connector; keep the runtime temptation dead in the docs.
*Labels:* `area:ci`, `area:dx`, `type:docs` · *Milestone:* Phase 4

### E5-S1 — GitHub Action: `brief validate` + `emit` + budget gate
**Priority:** Urgent · **Estimate:** 5 · **Labels:** `area:ci`, `type:feature`
**Description:** Composite action that validates the brief, emits selected targets, and gates the job on validate/Sacred/budget outcomes.
**Acceptance criteria:**
- [ ] Sample workflow validates + emits eridian/claude and gates on exit code.
- [ ] Budget overflow fails the job.

### E5-S2 — Differentiator demo: `brief` vs raw connector
**Priority:** High · **Estimate:** 3 · **Labels:** `area:ci`, `type:docs`
**Description:** Show one `.brief.md` emitting consistent constraints into multiple runtimes vs six divergent hand-edited configs / an invisible connector prompt.
**Acceptance criteria:**
- [ ] Side-by-side doc quantifying drift/consistency + review surface.
- [ ] Explicit "use raw connector when… / use brief-emit when…" boundary.

### E5-S3 — `eridian-trace` integration for emit
**Priority:** Medium · **Estimate:** 3 · **Labels:** `area:dx`, `type:feature`
**Description:** Emit trace spans for validate/compose/emit through `crates/eridian-trace`.
**Acceptance criteria:**
- [ ] Each phase is a span; off by default in CI unless requested.

### E5-S4 — README + AGENTS.md update (de-RAG, emit-target framing)
**Priority:** High · **Estimate:** 2 · **Labels:** `area:dx`, `type:docs`
**Description:** Document eridian/hermes as first-class emit targets; remove stale RAG mentions; align with "compiled intent theory" note.
**Acceptance criteria:**
- [ ] Quick Start lists `emit eridian` / `emit hermes`.
- [ ] No stale RAG references remain.

### E5-S5 — Anti-runtime guardrail (CI check)
**Priority:** Medium · **Estimate:** 3 · **Labels:** `area:dx`, `type:chore`
**Description:** CI test failing the build if `brief` gains a model-call HTTP client, dispatcher, or session store (enforces §3).
**Acceptance criteria:**
- [ ] Forbidden-import check fails the build with a message pointing to design-doc §3.

### E5-S6 — Example gallery (eridian + hermes)
**Priority:** Medium · **Estimate:** 2 · **Labels:** `area:dx`, `type:docs`
**Description:** Add runnable `examples/` for emitting to Eridian and Hermes from a shared `.brief.md`.
**Acceptance criteria:**
- [ ] One example per new target, each linking its enabling story.

---

## Suggested cycle sequencing

| Cycle | Focus | Stories |
|---|---|---|
| Cycle 1 | Formal schema + Sacred spec | E1-S1, E1-S2, E1-S3, E1-S4, E1-S5 |
| Cycle 2 | Eridian emit | E2-S1, E2-S2, E2-S3, E2-S5 |
| Cycle 3 | Hermes emit + #342 spike | E3-S1, E3-S2, E3-S3, E3-S4 |
| Cycle 4 | Parity harness | E4-S1, E4-S2, E4-S3, E2-S4, E3-S5 |
| Cycle 5 | CI + docs | E5-S1, E5-S2, E5-S4, E5-S5, E4-S4, E4-S5, E5-S3, E5-S6 |

**Critical path:** E1-S1 → E2-S1 → E2-S2 → E3-S1 → E3-S2 → E4-S1 → E5-S1.
Sacred (E1-S3 → E2-S3 → E3-S3 → E4-S3) is a parallel spine that must land before the CI gate.
