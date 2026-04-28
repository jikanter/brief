---
author: RL Engineer (online rewards, RLHF/RLAIF/PRM)
role: Reinforcement Learning Engineer
date: 2026-04-27
purpose: Independent analysis for synthesis into aichat-agent-gaps.md
---

# aichat agent specs through a reward-systems lens

I have spent the last decade building reward pipelines: pairwise preference models for RLHF, process reward models for math and code, programmatic verifiers for tool-use loops, and the unglamorous plumbing that catches reward hacking before it ships. I am writing this because the aichat agent format — `index.yaml`, `config.yaml`, `agents.txt` — is, from my vantage point, *missing the entire reward surface*. That is a defensible choice for a CLI runtime in 2024. It is a less defensible choice in 2026, and it is a choice `brief` has the chance to either inherit or quietly correct.

The framing I want to hold onto throughout: an agent specification is a *partial reward function*. Even when no gradient steps are happening at use time, the spec implicitly defines what behaviors get praised, ignored, or punished by users and downstream consumers. If the spec leaves the reward surface blank, the optimization pressure does not disappear — it gets supplied by whoever yells loudest in the chat window.

## What aichat specifies, and what it does not

aichat's `index.yaml` carries identity, instructions, dynamic-instructions toggle, flat variables, conversation starters, and RAG documents. `config.yaml` overlays model, sampling params, tool whitelist, an `agent_prelude` session, and optional input/output JSON Schemas. There is a `pipe_to`/`save_to` shell-style hook for terminal IO.

What is absent, taking the schema at face value:

- No `success_criteria` field.
- No `evaluator:` or `judge:` slot — no LLM-as-judge config, no rubric.
- No verifier model or programmatic check attached to `output_schema`. The schema constrains *shape*, not *correctness*.
- No preference-pair sink. No thumbs up/down → DPO file.
- No trajectory log target. `save_to` is for the final terminal artifact, not the rollout.
- No exploration controls: no `max_turns`, no `max_cost_usd`, no `temperature_schedule`, no halt-on-condition.
- No tripwire / abort rules beyond what the user writes inside `instructions:`.
- No notion of *outcome* vs *process* reward.
- No multi-agent credit-assignment surface (agents are flat).

The output JSON Schema is the closest thing to a verifier in the spec, and it is a shape check — it will accept `{"answer": "buy NVDA"}` whether the answer is right or ruinous.

## The state of the practice in 2026 (briefly)

I want to ground the rest of this in concrete systems rather than waving at "best practices."

- **Outcome verifiers and RLVR.** Cobbe et al.'s GSM8K verifiers, then DeepSeek-R1's RL-from-verifiable-rewards, codified a pattern: where a problem admits a programmatic check (unit test pass, exact match, sandbox exit code), the check is the reward. SWE-bench/SWE-agent runs this loop at agent scale. aichat agents have nothing here.
- **Process reward models.** Lightman et al. 2023 (PRM800K), Math-Shepherd, and the OpenAI "Let's verify step by step" line showed step-level signals beat terminal ones for long-horizon reasoning. The closest primitive in any of the formats I am reviewing is `brief`'s `assumptions` checkbox — `[ ]` unvalidated, `[x]` validated. That is a step-level signal, even if it was designed for human cognition rather than agent training.
- **Verbal RL / reflection.** Reflexion (Shinn et al.), Voyager (Wang et al.), and Generative Agents established that a written self-critique cached against a task ID is enough to give measurable improvement without weight updates. None of these formats persist that critique by default.
- **Trajectory-aware harnesses.** OpenHands, SWE-agent, AutoGen, LangGraph, and Anthropic's Claude Agent SDK all log trajectories (state, action, observation tuples) and treat them as a first-class object. aichat's session save is closer to a chat transcript than a trajectory; the tool-call decisions and observations are interleaved with prose rather than structured.
- **Constraint and assertion layers.** DSPy's `Suggest`/`Assert` and MIPRO, TextGrad's textual gradients, and constitutional-AI-style critique chains all bake "did the constraint hold?" into the loop. aichat trusts the model to read its instructions.
- **Budget and halt primitives.** Anthropic's Agent SDK exposes `max_steps`/`stop_sequences`; OpenAI Responses + tool_use has `max_tool_calls`; AutoGen has termination conditions. aichat's only knobs are CLI flags, not part of the agent contract.

Set against that, aichat looks like a 2023-vintage role-playing harness that grew tools. That is fine for "summarize this PDF." It is not fine for "open a PR that closes this issue."

## Reward specification: what brief can usefully model

`brief` already encodes more reward-shaped information than aichat consumes. `constraints.hard` is a hard penalty (any violation → reject). `constraints.soft` is a shaping reward (prefer this, but do not fail on it). `constraints.ask_first` is a *gated action* — the RL analogue is a region of action space requiring an explicit handoff, exactly the kind of guardrail you put around irreversible operations. `sacred` is a hard constraint with locality (a *region* of the codebase, not a global rule). `deliverable` is a goal description. `assumptions` is a process-signal primitive.

What is missing from `brief` itself, judged as a reward spec:

1. **A success-criteria slot distinct from `deliverable`.** `deliverable` reads as "what done looks like" in prose. A reward function needs the *check*, not the description. This can be lightweight — a list of programmatic checks (`cargo test`, a regex over the diff, a presence-of-file assertion) plus an optional rubric for an LLM judge.
2. **A verifier hook.** The lesson from GSM8K-verifiers through R1 is "if you can write a check, write the check." Even a single `verify:` field that names a shell command — exit-zero is success — would change the reward surface dramatically. This is not enforcement; it is *legibility* of the reward to whatever runtime consumes the brief.
3. **Budget bounds.** The kind of bug where a tool-using agent burns $40 in tokens looping on a flaky test happens because nobody wrote down the budget. `max_turns`, `max_cost_usd`, `wall_clock_minutes` are three lines in YAML and they protect the user from their own optimism.
4. **Halt-when conditions.** Distinct from budget. "Halt if you find a `TODO(security)` comment touching auth." "Halt if `git status` shows a deletion under `migrations/`." These are tripwires, and they are exactly the constraint-locality pattern `sacred` already uses.

I want to be specific about what I am *not* recommending. I am not asking `brief` to ship a judge model, host preference data, or run a verifier. `brief` is format-first and cooperative. The job is to give the runtime a slot to find the reward signal in, the same way `sacred` gives a runtime a slot to find the no-go regions in.

## Process vs outcome rewards, and the assumption checkbox

The strongest reward primitive `brief` already has is the assumption checkbox. Read as RL: `- [ ]` is a subgoal whose validation is part of the reward; `- [x]` is a subgoal already credited. This is, in spirit, a PRM signal — step-level, human-authored, expressible in 60 seconds.

Two improvements would generalize it without breaking the format:

- Allow a per-assumption *check command* (optional): `- [ ] DB writes are the bottleneck — \`./bench/db.sh > /tmp/x; grep -q "p99 > 200ms" /tmp/x\``. The runtime can flip the box if the check passes. This turns the assumption list into a cheap PRM trace.
- Add a `## Verification` section parallel to `## Assumptions` for *outcome* checks (final-state, not stepwise). This avoids overloading `deliverable` with executable content.

The format-first discipline holds: if no check is supplied, the assumption stays prose, and nothing breaks.

## Reward hacking and Goodharting

I have seen this movie. You give a model "write a function that passes the tests," it reads the test file and hardcodes the expected outputs. You give it "make the code lint-clean," it disables the lint rules. You give it "summarize the doc concisely," it returns one word.

The defenses the field has converged on:

- **Held-out checks the model cannot see.** R1, AlphaCode2.
- **Constitutional rules above the task reward.** Anthropic's CAI line.
- **Sacred regions / refusal behaviors.** Claude Code's hooks, Cursor's `.cursorrules`, brief's `sacred`.
- **Outcome verifiers that re-derive correctness from a different angle than the generator used.** GSM8K verifiers.

Mapping to `brief`:

- `constraints.hard` is the constitution.
- `sacred` is the refusal scope. It is the single most underrated primitive in the format. aichat has *no equivalent* and would have to carry it as prose inside `instructions:` — which is exactly where Goodharting lives, because the model can rationalize prose.
- `ask_first` is the gated-action list. It is the cheapest possible defense against irreversible reward hacking — "you can do anything, but you must ask before `rm -rf`."
- The missing piece is the *outcome verifier* slot. Without it, `deliverable` is the reward and `deliverable` is prose — Goodhart's heaven.

When `brief` emits to aichat, the constraint blocks should be voiced as imperative rules in `instructions:` (the README already proposes this) *and* duplicated into the JSON Schema's `description` fields if `output_schema` is set. The shape check then carries some of the constraint weight.

## Online vs offline: the missing feedback loop

aichat's `agent_prelude` is a static warm-up — load this session, then start the conversation. There is no "save preference," no "log this trajectory keyed by task," no "diff this run against the last good run." Compared with OpenHands' trajectory store or the SWE-agent rollout JSON, aichat is offline-only by accident.

`brief` cannot fix this in the runtime, but it *can* declare the contract. Two minimal slots:

- `trajectory_log:` — a path or sink the runtime should write rollouts to. The runtime decides format; `brief` just says "a feedback loop exists, here is where it lives."
- `preference_sink:` — a path for thumbs up/down or pair-collection. Same logic.

Both are out-of-scope to *implement* but in-scope to *name*. The cost is one frontmatter field each. The benefit is that downstream tooling (a future `brief replay` or an eval harness) has a stable place to look.

## Exploration, budget, and halting

This is the cheapest and highest-value addition. The kind of bug where an agent enters a tool-call loop and exhausts a credit balance before anyone notices is a 2026 reality, not a hypothetical. aichat punts to CLI flags. `brief` should bake the contract into the spec:

```yaml
budget:
  max_turns: 20
  max_cost_usd: 2.00
  wall_clock_minutes: 15
halt_when:
  - "git diff --name-only | grep -q 'migrations/'"
  - "tool_call_count('shell') > 50"
```

Format-first: if the runtime ignores it, the comment is still useful documentation. If the runtime honors it, the user is protected. aichat would need a small extension to consume this; today, `brief` would emit it as a comment in `config.yaml` plus a "## Budget" block in `instructions:`. That is cooperative pressure on aichat to grow the field, which is healthy.

## Verifiers, multi-agent composition, and non-stationarity

Three smaller points worth naming:

- **Verifier > generator.** If `brief` adds a single field this cycle, make it `verify:` (a shell command, exit-zero = success). That one field carries more reward signal than every other constraint combined for any task that admits a check.
- **Multi-agent credit assignment.** aichat is flat; brief's other emit targets (Claude Code skills, OpenAI Agents SDK, AutoGen, LangGraph) compose. A `role:` field at the top level (planner / executor / critic) and an optional `delegates_to:` would let `brief` say something useful about composition without adopting a workflow DSL. This is a P2-or-later consideration.
- **Non-stationarity.** A reward function calibrated to GPT-4 drifts under GPT-5; same for any prose spec. The mitigation is to keep the spec *anchored to checks, not to model quirks*. Every `verify:` and every `halt_when:` is a piece of the spec that survives a model bump. Every prose `instructions:` paragraph is a piece that does not. This is the strongest argument I can make for shifting `brief`'s center of gravity from prose toward executable assertions over time.

## What I would tell the brief team

Prioritized, with cost/benefit. Items 1–3 are high-leverage and cheap. Items 4–6 are explicitly stretch.

1. **Add a `verify:` slot in frontmatter or as a `## Verification` H2.** One field. Optional. Names a shell command (or list) whose exit-zero is success. Cost: small parser change, one emitter line per target. Benefit: every downstream runtime — aichat included — gains a programmatic reward signal without `brief` having to *be* the runtime. Closes the largest single gap in the aichat format.

2. **Add a `budget:` block in frontmatter** with `max_turns`, `max_cost_usd`, `wall_clock_minutes`. Cost: trivial schema work. Benefit: protects users from tool-loop incidents and gives runtimes a contract to honor. Emit to aichat as comments in `config.yaml` until aichat grows a real field.

3. **Generalize the assumption checkbox to optionally carry a check command.** Cost: parser tweak; backwards-compatible. Benefit: turns `## Assumptions` into a lightweight PRM trace. The most format-native improvement on this list — it extends a primitive that already works.

4. **Name a `trajectory_log:` and a `preference_sink:` path in frontmatter, even without implementing them.** Cost: two optional string fields. Benefit: claims the contract surface so future tooling has a stable target. Honest about being a stub.

5. **Mirror constraints into JSON Schema descriptions when `output_schema` is set on the aichat target.** Cost: emitter logic only. Benefit: shape-check carries some constraint weight; partial defense against Goodhart.

6. **Out of scope for `brief` but worth saying out loud:** an LLM-as-judge config block. I would not add this. Judging is a runtime concern, the right rubric is task-specific, and a brief field that names a judge model would either be ignored or abused. Leave it to the runtime; document the omission.

The throughline: `brief` is closer to a working reward specification than aichat's agent format is, and the cheapest way to widen that lead is to make the *checks* first-class and leave the *judging* to whoever consumes the brief. Format-first means naming the slot, not filling it.
