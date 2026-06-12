# aichat agent format: gaps relative to SOTA, and what `brief` should do about them

**Status:** Synthesis. Authored 2026-04-27 by the brief team lead from three independent specialist reviews of the aichat agent specification (`index.yaml`, `config.yaml`, `agents.txt`, sidecar `functions.json`).

**Reviewers:**

- A senior ML researcher (30y, ex-NeurIPS/ICML area chair) — historical / cognitive-architectures lens. Full review: [analysis-ml-veteran.md](analysis-ml-veteran.md).
- An RL engineer specializing in online reward systems (RLHF/RLAIF/PRM) — reward-surface lens. Full review: [analysis-rl-online-rewards.md](analysis-rl-online-rewards.md).
- An ML engineer focused on transformers and supervised learning — chat-template / structured-decoding / reproducibility lens. Full review: [analysis-llm-supervised.md](analysis-llm-supervised.md).

This document compresses their independent findings into a single picture for the team. Where they agreed, the consensus is treated as load-bearing. Where they disagreed, the disagreement is named and adjudicated against `brief`'s hard constraints (single binary, ~60s authoring, format-first, interoperability not replacement).

## TL;DR

aichat's agent format is a respectable Unix-flavored runtime schema — file-based, diffable, no framework lock-in — but it is, as one reviewer put it, "roughly 2022-vintage as a transformer-runtime contract." The format compresses identity, capability, and policy into a single `instructions:` blob; conflates always-in-context reference material with retrieval corpus and few-shot examples in `documents:`; offers no slot for a reward signal, verifier, budget, or trajectory; carries a cosmetic `version:` and no calibration metadata; and locates the output JSON Schema in the user's per-machine override rather than in the agent definition.

`brief` should *not* try to fix most of these inside the aichat target. It should fix the ones that show up in the brief format itself, because every emit target inherits them — and surface the rest as honest, advertised lossy-projection warnings when the aichat emitter runs.

The recommendations land in three tiers (full table at the end). The headline four are: **split `context:` into `pin / retrieve / examples`**, **add an optional `verify:` slot**, **add `brief.id` plus meaningful `version` semantics**, and **make `brief emit aichat` warn loudly when it drops information.**

## What aichat specifies, in one paragraph

`index.yaml` carries name, description, version, a free-form `instructions:` system prompt, a `dynamic_instructions: bool` toggle, a flat `variables:` list (name + description + optional shell-evaluated default), `conversation_starters:`, and `documents:` (paths/globs/URLs fed to a per-agent RAG). `config.yaml` overlays per-machine overrides: model (`client:model`), temperature, top_p, `use_tools:`, `agent_prelude:` (warm-up session), an instruction override, variable values, optional `input_schema` / `output_schema` JSON Schemas, `pipe_to:` / `save_to:`. `agents.txt` is a one-line-per-slug discovery file. Tool definitions live in a sibling `functions.json` produced by `argc build` from comment-tagged shell scripts. There is a templating mini-language inside `instructions:` for built-in vars, env vars, user-defined vars, and `{{#if}}` blocks. That is the entire surface.

## State of the art for open-source agent specifications, briefly

The reviews converge on roughly the following picture of where the open-source field has actually settled in 2026:

- **Tools as typed contracts.** JSON Schema in / JSON Schema out, side-effect / capability tags, transport identity. MCP, Anthropic `tool_use`, OpenAI Responses, DSPy Signatures, LangGraph nodes are all isomorphic to this.
- **Multi-message chat-template awareness.** Modern models attend to system / user / assistant / tool / developer roles distinctly; flattening to a single string drops measurable accuracy. Anthropic XML tagging, OpenAI's `developer` role, ChatML multi-system, Llama-3 / Qwen / Gemma template differences are all load-bearing.
- **Structured outputs with constrained decoding.** OpenAI Structured Outputs, Outlines, lm-format-enforcer, xgrammar, SGLang. JSON Schema as the machine contract; JSON-mode without a schema is a known footgun.
- **Reward signal in the spec.** `success_criteria`, programmatic verifiers (RLVR, GSM8K-verifier line, DeepSeek-R1), LLM-as-judge configs, preference sinks, trajectory logs. Even when no training is happening at use-time, the spec names where the reward lives.
- **Budget and halt primitives.** `max_steps`, `max_tool_calls`, `max_cost_usd`, `stop_sequences`, `halt_when:` predicates. Tool-loop incidents in production made these table stakes.
- **Reproducibility envelope.** Pinned model + decoding params + seed + content hash. DSPy compiled artifacts, LangGraph checkpoints, MCP protocol-version negotiation. Versioned eval suites colocated with the agent (HELM, lm-eval-harness, Inspect AI).
- **Composition by graph or by hand-off.** AutoGen, LangGraph, MetaGPT, CrewAI, Anthropic subagents. Even runtimes that stay flat (aichat) are scripted from outside into multi-agent loops.
- **Memory taxonomy.** Working / episodic / semantic / procedural, with eviction and provenance. MemGPT, Voyager, Generative Agents, A-MEM, Reflexion. Cognitive-architectures field had this in the 90s and the LLM-agent literature is re-deriving it paper by paper.
- **Process > outcome rewards for long horizons.** PRM800K, Math-Shepherd, "Let's verify step by step." Step-level signals beat terminal ones.

aichat addresses none of these directly. Some of that is fine — aichat is a CLI runtime, not an agent framework. The question is which gaps `brief` should help close at the format level.

## Where the three reviews converged

Every one of the following appeared independently in all three reviews. Treat these as load-bearing.

1. **Single-blob `instructions:` collapses three logically distinct things.** The veteran framed it as identity / capability / policy collapse (Soar / BDI / ACT-R lineage). The transformer engineer framed it as chat-template flattening (lost role-aware attention, no cache breakpoints, tool injection in the wrong slot). The RL engineer framed it as "policy without identity" (un-traceable constraint violations, un-composable rules). Same gap, three angles.

2. **`documents:` is not one thing.** Reference material that must be in context every turn, a retrieval corpus, and few-shot exemplars are three distinct primitives that aichat (and currently `brief`'s `context:`) collapse into one slot. The transformer engineer was loudest here; the veteran independently framed it as the "reference vs working corpus" distinction; the RL engineer is silent on it but consistent. Conflation forces every emitter to make the wrong call.

3. **No reproducibility or calibration story.** Cosmetic `version:`. No model-pin, no behavioral snapshot, no seed in decoding, no content hash on the agent. The veteran called for `brief.id` + meaningful `version` + `calibrated_against:`. The transformer engineer called for `tested_against:` + `decoding:` (with `seed`). The RL engineer framed it as "anchor the spec to checks, not model quirks." All three want the same thing through different vocabularies.

4. **Tools are not first-class typed contracts inside the agent definition.** Tool schemas live in a sidecar `functions.json` derived from shell-script comments; `use_tools:` lives in the per-user `config.yaml`, not the agent definition; the model never sees the tool surface in the same artifact that defines the agent. All three reviewers flagged this. They differ on what `brief` should do (see disagreements below) but agree on the gap.

5. **Constraint structure must survive at the source, even when emitters flatten it.** `Hard / Soft / Ask-First / Sacred` is the most valuable typed information `brief` carries; aichat will inline it as prose, and that is fine — provided the structured form remains the source of truth. None of the three reviewers wants `brief` to soften this on the way in.

6. **Output schema is in the wrong file.** The transformer engineer made this point sharply: aichat puts `output_schema` in `config.yaml` (the per-user override), not `index.yaml` (the agent definition). That is upside-down — the schema is part of the contract, not a user preference. The veteran and RL engineer reach a similar conclusion via different paths (the schema is part of identity / part of the reward surface).

7. **No reward, verifier, or budget surface at all.** The RL engineer made this central; the other two endorsed it as a sub-case of governance / non-stationarity. There is no `success_criteria`, no `verify:`, no `judge:`, no `max_turns`, no `halt_when:`. The output JSON Schema is a shape check, not a correctness check.

## Where the three reviews diverged

Disagreements were narrower than the convergence and mostly about scope discipline.

- **How much should `brief` model about tools?**
  - *Veteran:* nothing structured. Add a `capabilities:` controlled vocabulary (`fs.read`, `fs.write`, `net.read`, `net.write`, `exec`, `secrets`); leave tool I/O schemas to the runtime. This is the right altitude for a 60-second-to-author file.
  - *Transformer engineer:* reserve a `tools:` slot that accepts MCP server URLs, OpenAI-shape inline schemas, or aichat tool slugs. Without this, brief cannot ever represent agentic capability faithfully.
  - *RL engineer:* silent on the form, but compatible with either.
  - **Adjudication:** these are closer than they read. Both reviewers oppose modeling tool I/O JSON Schemas in `brief`. The disagreement is whether the slot is a *taxonomy* (capabilities) or a *reference* (tool ids / MCP URLs). Recommendation: ship `capabilities:` first (Tier 2), reserve a `tools:` reference slot for v2 (Tier 3) with a documented stub. Two thin fields, no schema modeling.

- **How aggressive should `brief` be about adding feedback-loop primitives?**
  - *RL engineer:* most expansive — `verify:`, `budget:`, `halt_when:`, `trajectory_log:`, `preference_sink:`, generalize assumption checkboxes with optional shell checks.
  - *Veteran:* most conservative — name only what survives a model bump. `verify:` would qualify; the rest can stay as runtime concerns.
  - *Transformer engineer:* aligned with veteran here.
  - **Adjudication:** `verify:` clears the bar with all three reviewers. `budget:` is cheap and protects users from real incidents; it clears the bar. `trajectory_log:` and `preference_sink:` are documentary stubs only — name them, do not implement them. Generalizing the assumption checkbox is the most format-native of the lot and clears the bar.

- **Should `brief` model composition?**
  - *Veteran:* yes, by name, not by graph. `delegates_to: [name, ...]`. Modeling topology is wrong scope.
  - *RL engineer:* yes, eventually. `role: planner | executor | critic` plus `delegates_to:`. P2-or-later.
  - *Transformer engineer:* silent.
  - **Adjudication:** Tier 3, name-only. Reserve the namespace. Do not implement multi-agent semantics in v1.

- **LLM-as-judge.**
  - *RL engineer:* explicitly out of scope. "Judging is a runtime concern, the right rubric is task-specific, and a brief field that names a judge model would either be ignored or abused."
  - *Others:* silent.
  - **Adjudication:** out of scope, document the omission.

## What `brief` should fix at the format level vs. what stays runtime

The reviews converge sharply on this discipline. The principle: **`brief` expresses intent and constraint; runtimes express execution and enforcement; the emitter is the contract between them.** Apply that to the gap list.

**Fix at the format level (modify `.brief.md`):**

- The `context:` conflation. Split into `pin / retrieve / examples`. The aichat backend cannot recover the distinction otherwise; neither can any other emitter.
- A reward-signal slot — `verify:` (and optionally a `## Verification` body section). One field, optional, names a shell command whose exit-zero is success. Format-first: if the runtime ignores it, it is documentation.
- Reproducibility metadata — `brief.id` (sha256 of canonical parse tree), real `version` semantics (additive-only within a major), `tested_against:` (or `calibrated_against:`) model list, a thin `decoding:` block with `seed / temperature / top_p / max_tokens / stop / reasoning_effort`.
- A `capabilities:` controlled vocabulary — `fs.read`, `fs.write`, `net.read`, `net.write`, `exec`, `secrets`. Each emitter maps to its runtime's tool selector. Right altitude for a 60-second file.
- A `budget:` block — `max_turns`, `max_cost_usd`, `wall_clock_minutes`. Cheap, protective.
- Generalize the assumption checkbox to optionally carry a check command. Backwards-compatible parser tweak; turns `## Assumptions` into a lightweight PRM trace.
- Reserve namespaces for v2: `delegates_to: [name]`, `trajectory_log:`, `preference_sink:`, `tools:` (MCP refs). Documentary in v1; emit nothing for them yet.

**Leave runtime-side, do not model:**

- Tool I/O JSON Schemas. (Veteran emphatic; transformer engineer agrees on the modeling discipline.)
- Memory architecture (working / episodic / semantic / procedural; eviction). Deeply runtime-specific.
- Trace formats (OpenTelemetry GenAI, LangSmith spans, Phoenix). Runtime concern.
- Eval suite formats (HELM, lm-eval-harness, Inspect AI). Runtime concern.
- Sandbox policy / permission tiers. Runtime concern.
- LLM-as-judge configs.
- Multi-agent graph topology (LangGraph state machines).
- Constrained-decoding implementation choice (Outlines, xgrammar, etc).

**Fix at the emitter level (modify `src/emit/aichat.rs` only):**

- Voice constraint blocks as imperative rules in `instructions:` (skill-emitter register, not claude-emitter `**IMPORTANT:**` style). The README already calls this out.
- When `output_schema` is set in the brief, mirror constraint text into the schema's `description` fields so shape-check carries some constraint weight.
- Push `output_schema` to `index.yaml`, not `config.yaml` — the schema is part of the agent contract. PR upstream if aichat does not yet allow it there; otherwise document the constraint.
- Place rules in the high-recency slot of `instructions:` (immediately before the user-input marker `__INPUT__`) regardless of authoring order.
- Round-trip aichat templating tokens unchanged: `__INPUT__`, `{{__tools__}}`, `{{__os__}}`, `{{var}}`, `{{$AICHAT_FOO}}`, `{{#if}} … {{/if}}`. YAML serializer must not fold or escape `{{` / `}}`. Test with explicit fixtures.
- Print stderr warnings on every lossy projection: dropped multi-message structure, dropped `pin/examples` distinction, dropped `verify:`, dropped `budget:`, dropped `decoding:` fields aichat does not consume, dropped `tested_against:`. Authors learn what their brief actually contracts for.

## Tiered recommendations

| Tier | Item | Format change? | Cost | Why now |
|---|---|---|---|---|
| **1** | Split `context:` → `pin / retrieve / examples` | Frontmatter | Parser + emitters | Highest single information-loss gap; cannot be recovered downstream |
| **1** | `verify:` slot (and/or `## Verification` H2) | Both | Small parser change | Single largest reward-signal gain; survives model bumps |
| **1** | `brief.id` content hash + meaningful `version` | Frontmatter | One parser change | Cheap provenance every emitter inherits; pay later otherwise |
| **1** | `brief emit aichat` warns on lossy projection | Emitter only | Emitter logic | Honesty about what the artifact contracts for |
| **2** | `tested_against:` model list | Frontmatter | Trivial | Reproducibility floor; surfaces in `validate` |
| **2** | `decoding:` block (seed / temp / top_p / max_tokens / stop / reasoning_effort) | Frontmatter | Trivial | Reproducible evals impossible without seed |
| **2** | `capabilities:` controlled vocabulary | Frontmatter | Schema + emitter mapping | Right altitude for tools without modeling JSON Schema |
| **2** | `budget:` block (max_turns, max_cost_usd, wall_clock_minutes) | Frontmatter | Trivial | Real protection against tool-loop incidents |
| **2** | Constraint structure preserved as source-of-truth (no concession to aichat) | Discipline | None — already so | Reaffirm; do not let aichat target soften the model |
| **3** | Generalize assumption checkbox with optional shell check | Body parser | Backwards-compatible tweak | Lightweight PRM trace that extends an existing primitive |
| **3** | `deliverable_schema:` (inline JSON Schema or path) | Frontmatter | Schema + emitter routing | Compiles to OpenAI Structured Outputs / aichat output_schema / Claude tool |
| **3** | Reserve `delegates_to: [name, ...]` | Frontmatter | Field only | Composition by name, not by graph |
| **3** | Reserve `trajectory_log:` and `preference_sink:` | Frontmatter | Two optional strings | Stub-name the contract surface |
| **3** | Reserve `tools:` (MCP refs / aichat slugs) | Frontmatter | Field only | Future capability without modeling tool I/O |
| **out** | Tool I/O JSON Schemas, memory architecture, trace formats, eval suites, sandbox policy, LLM-as-judge configs, graph topology | — | — | Runtime concerns by review consensus |

## What this means for the aichat backend roadmap

The existing aichat README ([./README.md](../../design/backends/aichat/README.md)) lists four scaffolding items: multi-file emit return type, `--output <dir>`, widening `--install`, idempotent file-append for `agents.txt`. None of those change. They remain prerequisites.

What the gap analysis adds:

1. Land **Tier 1** in the brief format itself *before* the aichat backend ships. Specifically: `pin / retrieve / examples` split, `verify:` slot, `brief.id`, lossy-emit warnings. Without these, the aichat backend will silently lose information that future emit targets (Claude Code, OpenAI Agents SDK, MCP) will need.

2. Treat the aichat target as **explicitly lossy**, and say so. The prompt and claude emitters remain the high-fidelity targets. The aichat target is *useful*, not *faithful*. Documentation should state this; emit-time warnings should reinforce it.

3. Settle the open question in the README about `output_schema` placement: the aichat emitter should target `index.yaml`, not `config.yaml`, on the principle that the schema is part of the agent contract. If upstream aichat does not yet support `output_schema` in `index.yaml`, file the issue and emit to `config.yaml` with a stderr warning naming the workaround.

4. Test fixtures for the aichat emitter must include the templating-token round-trip cases (`__INPUT__`, `{{__tools__}}`, `{{$AICHAT_FOO}}`, `{{#if}}`). The transformer-engineer review flagged this as a YAML-serializer hazard.

5. **Out of scope for the aichat backend**, even after these changes: tool definition emit, memory architecture emit, trajectory logging, judge configuration, multi-agent wiring. These are runtime concerns; the brief format may eventually reserve namespaces for them, but the aichat emitter will not consume them.

## Closing read

The single best framing came from the veteran review: the aichat target will be `brief`'s first non-trivial backend, and "it will surface, by what it cannot represent, exactly which fields brief itself is missing. Use it that way."

The three independent reviews — historical, reward-systems, transformer — converged on a small, well-defined list of those missing fields, and a smaller list of things `brief` should *not* model. Honoring both lists is the work.

## Appendix: source reviews

- [analysis-ml-veteran.md](analysis-ml-veteran.md) — Senior ML researcher, 30y. Historical / cognitive-architectures lens. Strongest on identity-vs-capability-vs-policy separation, governance, lifecycle, and the `calibrated_against:` non-stationarity argument.
- [analysis-rl-online-rewards.md](analysis-rl-online-rewards.md) — RL engineer, online rewards (RLHF/RLAIF/PRM). Strongest on the missing reward surface, `verify:`, `budget:`, the assumption checkbox as a process-reward primitive, and reward-hacking defenses mapped onto brief's existing `sacred` / `ask_first` / `constraints.hard`.
- [analysis-llm-supervised.md](analysis-llm-supervised.md) — Transformer / structured-decoding engineer. Strongest on chat-template ignorance, the three-way `documents:` conflation, structured outputs and constrained decoding, decoding-controls / `seed`, and lossy-emit warnings.
