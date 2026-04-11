# Emit Quality Refinements

**Status:** Forward-looking analysis extending [phase2-synthesis.md](phase2-synthesis.md) §P0 and §P6.
**Source:** Extracted 2026-04-11 from [archive/flexibility-gap.md](archive/flexibility-gap.md) and [archive/format-expressiveness.md](archive/format-expressiveness.md) — refinements the original archives surfaced that are not yet captured in the live roadmap.

Phase 2 synthesis established the core emit-quality thesis: the emitter is a prompt engineering problem, not a template rendering problem. NEVER/MUST/PREFER/STOP framing, primacy/recency section ordering, and sacred region reframing are in the live roadmap. This document captures the *refinements* to that thesis that came out of the archived reports and deserve to outlive them.

---

## 1. Constraint Polarity: NEVER for Prohibitions, MUST for Requirements

Phase 2 synthesis P0 proposes `NEVER: <constraint>` for Hard constraints uniformly. The flexibility-gap report's sharper observation: Hard constraints are not semantically uniform. Some are prohibitions ("do not break backward compatibility", "no lodash", "never use `unsafe`"); others are positive requirements ("use `Result<T, AppError>`", "all public functions must have doc comments").

Prohibitions map naturally to `NEVER:`. Requirements map naturally to `MUST:`. Rendering a requirement as `NEVER:` is actively misleading — it implies an adversary being suppressed when there isn't one.

**Proposed refinement:** the emitter auto-detects polarity via keyword scan of the constraint text (leading "do not", "never", "avoid", "no ", "don't" → prohibition; everything else → requirement) and routes to the appropriate imperative verb. ~20 lines of emitter code, measurable compliance impact, no format change.

**Why this matters beyond the trivial fix:** it's a concrete instance of the broader "framing is prompt engineering" thesis. The same underlying constraint text lands differently depending on whether the imperative verb matches the constraint's polarity, and the emitter has enough information to get this right automatically.

---

## 2. Convention vs. Prohibition Framing

The format-expressiveness report made an even sharper point, which is load-bearing for the P0 work currently in flight.

A Hard constraint like "Use `thiserror` for library errors" is semantically **different** from a Hard constraint like "Never use `unsafe`". The first is a standing convention — it describes how the project has decided to do something, with no implied adversary. The second is an active prohibition — it implies there *is* an alternative the author wants suppressed.

Rendering every Hard constraint as `NEVER:` collapses this distinction and:
- Inflates the apparent constraint weight (everything reads as equally adversarial)
- Confuses the model when there is no obvious thing being prohibited
- Trains readers to skim past `NEVER:` as boilerplate

**Proposed refinement:** extend the polarity auto-detection with a third bucket. Conventions (positive, no prohibition keywords, uses declarative verbs like "use", "prefer", "follow") could render as simple imperative statements without the NEVER/MUST prefix, reserving the strong imperatives for constraints that actually need them.

This is the counter-argument to "apply NEVER/MUST everywhere" that the P0 roadmap work will encounter as soon as it's tested against real briefs. Capturing it here so the trade-off is visible before the code is written.

**Connection to format evolution:** if the emitter treats "convention" as a distinct semantic class, it becomes natural to also recognize a `## Conventions` section alongside `## Constraints`. This is tracked as an open question in [../open-questions.md](../open-questions.md) `[format]` Behavioral Instructions rather than an actionable item, because it implies format growth.

---

## 3. Per-Target Tone Adaptation

Phase 2 synthesis P0 applies NEVER/MUST/PREFER/STOP framing uniformly. The emit-integration audit's observation: different target ecosystems have different idiomatic registers. Claude Code measurably responds to imperative/RFC-2119 language. Aider's native conventions file is more conversational. Cursor rules tend toward descriptive statements, not imperative commands. Copilot's instructions are typically descriptive as well.

**Proposed refinement:** target-specific tone presets. The `claude` target uses the aggressive NEVER/MUST register (already in P0). The `aider` and `copilot` targets render constraints as plain bulleted preferences, closer to the original authoring format. The `prompt` target — which occupies the highest-privilege position as a system prompt — uses the most aggressive register.

This is a small addition to each emitter (a format function per tier, parameterized by target) but it prevents brief from imposing Claude's idioms on ecosystems where they read as overwrought.

**Related:** the [emit-targets-reference](../emit-targets-reference.md) divergence matrix and [../design/backends/](../design/backends/) per-backend docs flag "idiomatic register" as one of the axes along which no single unified emit format is possible. This refinement is how the emit layer accommodates that axis.

---

## 4. Fragmentation vs. Coherence: When Not to Listify

The format-expressiveness report made a philosophical point that generalizes well. Some constraints read most effectively when preserved as a paragraph, not split into bullet-per-rule:

> "Tests for every parser edge case. Integration tests that round-trip: parse a fixture, emit, verify output contains expected content."

This is a testing *worldview*, not a list of individual constraints. Fragmenting it into three bullets loses the framing — "here is how we think about testing" — and trades coherence for structure.

**Implication for emit:** the emitter should not add visual bullet markers (`-`, `*`) to content that was authored as prose. More subtly: the emitter should not fragment multi-sentence Soft constraints into multiple lines under the assumption that every newline is a list boundary.

**Implication for authoring:** brief's heading taxonomy encourages fragmentation by default. Authors should be told explicitly that prose-form constraints are legitimate, and the format should not reject them. This is authoring guidance, not a feature — worth a sentence in [brief-format.md](../brief-format.md) when that file is written.

**Connection to compiled-intent-theory:** the level-2 "intent embedding" framing in [compiled-intent-theory.md](compiled-intent-theory.md) implicitly requires holding chunks of intent together rather than shattering them into atomic tokens. Fragmentation works *against* the embeddings having useful structure. This is a small piece of evidence that the compiled-intent direction has emit-layer implications even before any embedding work starts.

---

## 5. Optional `emit:` Per-Target Frontmatter Map

From the emit-integration audit, applicable once P4's cross-ecosystem work begins: a bounded per-target hint mechanism in frontmatter.

```yaml
---
stack: [Rust]
emit:
  cursor:
    split_by: scope       # emit one .mdc per sacred/constraint cluster
  claude:
    emphasis: strong      # use the aggressive register
  aider:
    tone: conversational
---
```

**Design constraints:**
- Bounded — 2-3 keys per target max. This is a hint channel, not a DSL.
- Per-target blocks are optional. Absence means "use the target's default rendering rules."
- Never used to carry content — hints only. If a user wants per-target *content*, that's an argument for scoped constraints, not a frontmatter map.

**Why bounded:** the opposite extreme (per-target content sections, `## Cursor`, `## Claude`) creates a combinatorial explosion and fragments authorial intent across sections. A hint map preserves the single-source authoring model while acknowledging that different targets benefit from different rendering choices.

**When to build:** not now. Phase 2 synthesis P4 can ship the trivial wrappers without this. It becomes necessary once (a) two or more targets are shipped and (b) a real brief demonstrates a concrete case where the same authoring content wants different emit behavior per target. Track as demand-pull.

---

## 6. Command-Executable Validation

Minor refinement to `brief validate`. Once `brief init` auto-scaffolds a `## Commands` section from `Cargo.toml` / `package.json`, the commands become a silent failure mode: the underlying binary could be renamed, removed, or not on PATH, and nothing flags it.

**Proposal:** lightweight validator pass that scans the unknown section for `## Commands` and runs `which` (or equivalent) on backtick-wrapped tokens. Warns on missing executables without failing validation.

This does not require first-class parsing of `## Commands` — it operates on the raw unknown-section Markdown. It pairs naturally with the "constraint specificity validation" item already in phase2-synthesis P6, covering a different class of vague-or-stale content.

**Priority:** nice-to-have. Build alongside or after `brief init` auto-scaffolding of commands, whenever that happens.
