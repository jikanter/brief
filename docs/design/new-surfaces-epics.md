# `brief` — Delivery Plan — v3

**Tool:** Linear (Projects = Epics, Issues = Stories, Sub-issues = tasks)
**Repo:** `https://github.com/jikanter/brief`
**Design doc:** [new-surfaces-design-doc.md](new-surfaces-design-doc.md) (companion, v3)
**Author:** Jordan Kanter
**Last updated:** 2026-09-27

> **What changed from v2:** E3 (Hermes emit target) is **cut**: aichat's metadata refinery owns the aichat ↔ Hermes boundary. E2 becomes the **aichat role emitter**; the agent emitter is deferred to E6. E5-S3 (`eridian-trace` spans from brief) is **dropped**: aichat's trace is the only telemetry source. Shipped stories are marked. Story ids are kept stable, so a cut id is never reused.

> **Linear modeling convention**
> - **Epic → Linear Project** (name, goal, success metric).
> - **Story → Linear Issue** (title, description, acceptance criteria, estimate [Fibonacci], priority, labels).
> - **Labels:** `area:spec`, `area:emit-aichat`, `area:validate`, `area:budget`, `area:sacred`, `area:ci`, `area:dx`, `type:feature`, `type:chore`, `type:spike`, `type:docs`.

---

## Epic overview

| Epic | Goal | Status | Success metric |
|---|---|---|---|
| E1 | Formal, versioned schema | Mostly shipped; S2, S5, S6 open | Every fixture validates; `context:` accepts URIs |
| E2 | aichat role emitter | Next | An emitted role loads in aichat and keeps a stable `lock_hash` |
| E3 | ~~Hermes emit target~~ | **Cut** (2026-09-27) | — |
| E4 | Validation, Sacred & budget parity | Planned | One harness checks all six guarantees for every target |
| E5 | CI/CD & docs | Planned; S4 done | A CI workflow gates validate, format and the runtime guardrail |
| E6 | aichat agent emitter | **Deferred** | — |

---

## E1 — Brief Schema Formalization

### E1-S1 — Formalize frontmatter + body schema · **Shipped**
`docs/schema/SPEC.md`, `brief-v1.schema.json`, and the derived `brief-frontmatter-v1.schema.json`.

### E1-S2 — Schema versioning + reject-unknown-version · **Partly shipped**
**Priority:** Low · **Estimate:** 1
`brief_version` is gated. Open: an unknown version exits with a dedicated code instead of the generic 1.
- [ ] Dedicated non-zero exit code for an unknown `brief_version`, documented.

### E1-S3 — Sacred path-scoping formalization · **Shipped**
Scoped constraints and Sacred share one matcher (`pathmatch.rs`); `brief check` verdicts are tested.

### E1-S4 — Constraint-tier semantics · **Shipped**
`brief-format.md` §3.1 and the emit-time register in `design-decisions.md`.

### E1-S5 — Non-goals as schema guardrails
**Priority:** Medium · **Estimate:** 2 · **Labels:** `area:spec`, `type:chore`
The schema describes session, tool and decoding keys as outside the format, but `brief validate` accepts them silently.
- [ ] `brief validate` warns on execution-type frontmatter keys, naming the non-goal.
- [ ] Rationale links `design-decisions.md` "Non-goals".

### E1-S6 — URI-aware `context:` (new)
**Priority:** High · **Estimate:** 3 · **Labels:** `area:spec`, `area:validate`, `type:feature`
`context:` accepts "file paths or URLs", but every entry is treated as a local file: `validate` warns "Context file not found" on `https://…` and `ace://…`, and `emit claude` renders them as `@` imports.
- [ ] An entry with a URI scheme is not checked on disk.
- [ ] `emit claude` renders a URI as a link, not an `@` import; every other target renders it as plain text.
- [ ] A plain `ace://` pointer works end to end. A typed ACE field waits for the YAGNI bar.

---

## E2 — aichat Role Emitter

**Goal:** `brief emit aichat` produces a role aichat loads, in its terse register, byte-stable, installed and removed cleanly. *Non-goal:* running aichat.
*Labels:* `area:emit-aichat`, `type:feature`

### E2-S1 — Spike: pin the role surface
**Priority:** Urgent · **Estimate:** 3 · **Labels:** `type:spike`
Answer the design doc's §7.1 questions against aichat's current source: config-dir resolution per platform, whether role frontmatter tolerates an unknown key (for an ownership marker), and whether a hand-written and an emitted role of the same content give the same `lock_hash`.
- [ ] Documented role format, install path and ownership approach, in `backends/aichat/`.
- [ ] A hand-written role vs the emitted role for the same brief, diffed.

### E2-S2 — Implement `brief emit aichat [--install|--uninstall]`
**Priority:** Urgent · **Estimate:** 5
Map the brief to a role per the design doc's §7.1 table, in the skill emitter's voice; preserve aichat placeholders; name the role with `--name`, the file stem, or the goal.
- [ ] Emits a role aichat loads and runs (`aichat -r <name>`).
- [ ] Re-running `--install` is a no-op; `--uninstall` removes only a brief-owned role.
- [ ] Placeholders (`__INPUT__`, `{{…}}`, `{{#if}}`) round-trip unchanged.

### E2-S3 — Determinism and `lock_hash` conformance
**Priority:** High · **Estimate:** 2
- [ ] Two emits of the same brief are byte-identical (test).
- [ ] aichat's `--lock` over the emitted role gives the same hash on every emit; a fixture pins it.

### E2-S4 — Sacred → `tool_approval` deny rules · **Blocked on aichat (gap A1)**
**Priority:** High · **Estimate:** 3 · **Labels:** `area:sacred`
Once aichat can match a named tool argument, emit Sacred globs as deny rules.
- [ ] Companion issue filed in aichat (documented here, applied there).
- [ ] Conformance test: the role's rules deny a write that `brief check` flags.

### E2-S5 — `--budget` / `--compact` for the role
**Priority:** Medium · **Estimate:** 2 · **Labels:** `area:budget`
- [ ] Reported against the `prompt` threshold.
- [ ] `--compact` gives a shorter role.

### E2-S6 — Round-trip example
**Priority:** Medium · **Estimate:** 2
- [ ] `examples/` holds a brief, its emitted role, and the aichat command that runs it.

---

## E3 — ~~Hermes Emit Target~~ · **Cut (2026-09-27)**

aichat's metadata refinery moves a role's description into Hermes' `profile.yaml` and deliberately nothing else. brief reaches Hermes only through a role it emits for aichat, so E3-S1 through E3-S5, including the tool re-export spike (#342), are not brief work.

---

## E4 — Validation, Sacred & Budget Parity

### E4-S1 — Cross-target emit conformance harness
**Priority:** High · **Estimate:** 5 · **Labels:** `area:validate`
- [ ] One harness asserts all six guarantees (idiomatic, idempotent, reversible, non-destructive, budget-reportable, deterministic) for every target.

### E4-S2 — Golden idempotency tests
**Priority:** High · **Estimate:** 3
Install, re-install and uninstall against golden host files, including host files that mention the marker in prose or code (the bug fixed 2026-09-27).
- [ ] Re-install produces zero diff.
- [ ] Uninstall restores the pre-install file byte for byte.

### E4-S3 — Sacred conformance suite
**Priority:** High · **Estimate:** 3 · **Labels:** `area:sacred`
- [ ] The same Sacred verdict from `brief check`, the Claude and Cursor hooks, `validate-diff`, and (after E2-S4) the aichat role's rules.

### E4-S4 — Budget regression gate
**Priority:** Medium · **Estimate:** 2 · **Labels:** `area:budget`
- [ ] A test fails when an emission exceeds a configured ceiling.

### E4-S5 — Conflict warnings for new targets
**Priority:** Low · **Estimate:** 2
- [ ] `--install` conflict detection covers the role target.

---

## E5 — CI/CD & Docs

### E5-S1 — CI workflow: validate + test + format + guardrail
**Priority:** Urgent · **Estimate:** 3 · **Labels:** `area:ci`
The repo has no CI.
- [ ] A workflow runs `cargo test`, `cargo clippy`, `cargo fmt --check` and `brief validate` on this repo's own `.brief.md`.
- [ ] A reusable action or recipe for other repos: validate, emit, and fail on a budget overflow.

### E5-S2 — Differentiator demo, including the 48C round trip
**Priority:** High · **Estimate:** 3 · **Labels:** `type:docs`
- [ ] One `.brief.md` emitting consistent constraints into `AGENTS.md`, `CLAUDE.md` and an aichat role.
- [ ] The cassette round trip: brief → `astrophage serve --mode cassette --record` → commit → promptfoo replay → a prompt edit fails as a cassette miss.

### E5-S3 — ~~`eridian-trace` spans from brief~~ · **Dropped**
aichat's trace is the only telemetry source in the stack; brief emits no telemetry.

### E5-S4 — README + standing-file framing · **Done (2026-09-27)**
`AGENTS.md` is the cross-vendor default in the Quick Start; the intent statement is in the README, `CLAUDE.md` and `design-decisions.md`.

### E5-S5 — Anti-runtime guardrail
**Priority:** Medium · **Estimate:** 2 · **Labels:** `area:dx`
Today a test checks `Cargo.toml` for four crate names.
- [ ] Extend it to model-client crates and to source imports; the failure message cites `design-decisions.md` "Non-goals".

### E5-S6 — Example gallery
**Priority:** Low · **Estimate:** 2 · **Labels:** `type:docs`
- [ ] One example per integrated-stack target (role, promptfoo), from a shared brief.

---

## E6 — aichat Agent Emitter · **Deferred**

An agent is a directory whose definition lives in llm-functions, and it needs multi-file emit support. It waits until agent formats settle across the industry. Reference: [backends/aichat](backends/aichat/README.md).

---

## Suggested sequencing

| Cycle | Focus | Stories |
|---|---|---|
| 1 | Foundations | E1-S6, E5-S1, E5-S5, E1-S2 |
| 2 | Role emitter | E2-S1 → E2-S2 → E2-S3, E2-S5 |
| 3 | Parity | E4-S1, E4-S2, E4-S3, E2-S6 |
| 4 | Docs and demo | E5-S2, E5-S6, E1-S5, E4-S4, E4-S5 |
| When aichat is ready | Sacred in aichat | E2-S4 (after aichat gap A1) |

**Critical path:** E5-S1 → E2-S1 → E2-S2 → E2-S3 → E4-S1.
