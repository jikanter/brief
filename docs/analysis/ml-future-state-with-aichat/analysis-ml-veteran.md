---
author: ML Veteran (30y, ex-area-chair)
role: Senior Machine Learning Researcher
date: 2026-04-27
purpose: Independent analysis for synthesis into aichat-agent-gaps.md
---

# A veteran's read on the aichat agent format

I have been around long enough to remember when "agent" meant a Soar production system and a graduate student arguing for three hours about whether a chunking event constituted learning. I have also been around long enough to know that every generation of agent tooling re-discovers, with great fanfare, ideas that were settled in 1987. That is the lens I am bringing to aichat.

What follows is a critical evaluation of aichat's on-disk agent specification — `index.yaml`, `config.yaml`, `agents.txt`, and the `functions.json` produced by `argc build` — measured against where the open-source agent-spec field has actually converged. The audience is the brief team. The goal is to figure out which gaps brief should help close at the *format* level, and which it should leave to the runtime.

## What aichat got right

Before I take it apart, let me say plainly what I would not change.

- **File-based, plain-text, diffable.** No registry server, no SaaS, no proprietary container. The whole agent is a directory you can `tar` and email. After thirty years of watching ML tooling oscillate between "everything is a service" and "everything is a file," I have a very strong prior that the file camp wins on longevity. aichat is in the right camp.
- **Separation of definition and per-user config.** `index.yaml` is the upstream artifact; `config.yaml` is the user's override. This is the same pattern as `kubeconfig`, `~/.gitconfig`, and `tsconfig.json` extends-chains. It is correct and underrated. Most "agent platforms" of the last two years did not figure this out.
- **A registry that is just a text file.** `agents.txt` is one line per agent, `#` comments allowed. Compare this to Hugging Face Hub-style metadata sprawl. It is not glamorous, but it is the kind of thing you can still read in 2040.
- **YAML frontmatter plus prose.** The `instructions:` block is human-authored Markdown, not a templating DSL. This is the right default for a tool whose purpose is to load a system prompt, and it is the same instinct that makes `.brief.md` viable.
- **No framework lock-in.** aichat does not impose a planner, a memory system, or a multi-agent orchestrator. Whatever you put in `instructions:` is what runs. That is a feature.

If aichat were trying to be the *one* agent format, those virtues would still leave it short. But it is not — it is a CLI runtime's local schema. Held to that standard, the criticisms below are mostly about what *brief* should not lose by emitting to it.

## The conceptual gaps

### Identity, capability, and policy are all one blob

Classical agent architectures kept these separate for good reason. Soar had problem spaces, operators, and preferences. BDI agents had Beliefs, Desires, and Intentions, with deliberation rules over them. ACT-R drew a hard line between declarative and procedural memory. Even the more pragmatic 2000s work — Jason, JADE, GOAL — kept "who I am" distinct from "what I can do" and from "what I am allowed to do."

aichat collapses all three into one `instructions:` field. The agent's *identity* ("you are a senior reviewer of Rust code"), its *capability surface* (which is actually defined elsewhere in `functions.json` and selected by `use_tools:`), and its *policy* ("never modify files under `src/auth/**`") are all prose, all in the same string, with no schema saying which is which.

Why does this matter? Three reasons. First, it makes constraints unenforceable: there is no way for the runtime to know that the line "never touch `src/auth/`" is a hard policy as opposed to a stylistic preference. Second, it makes constraints unobservable: you cannot trace which policy a generation violated, because policies have no identity. Third, it makes them un-composable: if I want to mix two agents' constraints, I am doing string surgery on Markdown.

Brief's `Constraints { hard, soft, ask_first }` and `Sacred` types already encode this separation. That is the right move. The aichat emitter has to flatten it on the way out, but the source of truth keeps the structure.

### Tools as typed contracts, not comment-tagged shell scripts

aichat's tool story is `argc build` running over shell scripts whose comments declare argument types. The output is `functions.json`, an OpenAI-shaped tool-call schema. This works, and it is charmingly Unix. It is also where aichat is furthest from the state of the art.

The field has converged on tools as *typed I/O contracts with side-effect declarations*. MCP makes this explicit: a tool has a JSON-Schema input, a JSON-Schema output, and a transport-level identity. Anthropic's `tool_use` blocks and OpenAI Responses tools are isomorphic to this. DSPy goes further — its Signatures are typed I/O at the *step* level, allowing the optimizer to reason about the program's data flow. LangGraph nodes are typed in the same spirit.

What aichat is missing — and what brief should *not* try to fix at the format level — is:

- Output schemas on tools (aichat has input shapes but the return is whatever the script printed).
- Side-effect / capability tags (read-only? network? destructive? requires-approval?).
- Provenance: which agent versions were tested against which tool versions.
- A way to declare "this agent requires tools X, Y, Z" inside `index.yaml`. Right now `use_tools:` lives only in the per-user `config.yaml`, which means an agent definition cannot state its own capability needs. That is the kind of inversion-of-dependency mistake we used to catch in code review.

The brief team should resist the temptation to model tools in `.brief.md`. Tools are a runtime concern with an established schema (JSON Schema for I/O, plus capability tags). What brief *can* do is express **required capabilities at a coarse grain** — "this brief needs filesystem write, network read" — and let each emitter map that onto the runtime's tool selector. That is a policy field, not a tool field.

### Memory is one warm-up session and a RAG bag

aichat's memory architecture is `agent_prelude` (an optional warm-up session loaded at start) and `documents:` (a per-agent RAG corpus). That is it. No working-memory window management beyond the model's context, no episodic store of prior runs, no semantic memory beyond the RAG, no procedural memory beyond the system prompt.

The cognitive-architectures community settled this taxonomy decades ago. ACT-R distinguished declarative chunks from procedural productions; Soar had working memory plus chunked productions plus, later, semantic and episodic stores; CLARION layered implicit and explicit. The modern LLM-agent literature has been re-deriving these one paper at a time: MemGPT for working/episodic split; Voyager for skill libraries (procedural memory in everything but name); Generative Agents for episodic-with-decay; A-MEM and MemoryBank for eviction policies. Reflexion is procedural memory of "what failed last time."

What is genuinely missing from aichat:

- **Provenance on documents.** A RAG path is just a path. There is no field saying "this document is authoritative as of date X" or "this is a draft, do not cite." In a long-lived agent, that gap will rot the corpus.
- **Eviction / staleness.** No TTL on documents, no notion of which preludes are still valid against which model versions.
- **Episodic recall.** aichat sessions exist but are not first-class memory; the agent has no way to say "load the last three sessions tagged `incident-review` as context."
- **Skill / procedural memory.** No place to register reusable sub-procedures the agent has learned. Voyager's skill library would have nowhere to live.

For brief's purposes, the right move is *not* to model memory architecture in `.brief.md`. Memory is deeply runtime-specific — what works for Claude Code's filesystem-as-memory is wrong for an aichat RAG. But brief should distinguish, in its `context:` frontmatter, between **reference material** (read-once, authoritative) and **working corpus** (indexed, queryable). aichat collapses these into `documents:`; that's the runtime's problem, but brief should not lose the distinction on the way in.

### No evaluation, no traces, no determinism

This is the gap that would have gotten an aichat-style spec rejected at NeurIPS in 2014, let alone now. There is no `evals:` slot, no expected-behavior fixtures, no trace schema, no seed/temperature pinning beyond `temperature:` and `top_p:`, no notion of a regression suite.

The field has settled — finally — on at minimum:

- **Versioned eval suites** colocated with the agent. HELM, BIG-bench, lm-eval-harness, and now Inspect AI have all converged on "the eval is a first-class artifact, lives next to the model/agent, runs in CI."
- **Structured trace schemas.** OpenTelemetry GenAI semantic conventions, LangSmith's run schema, Phoenix's spans. A trace is the only thing that lets you debug a failed run six weeks later.
- **Reproducibility envelopes** — at least the model id, the system prompt hash, the tool versions, and the random seed where applicable.

aichat has none of this. A `version: "0.1.0"` field is cosmetic; nothing checks it, nothing pins behavior to it. Run the same agent twice with `temperature: 0` against the same model id and you will still see drift, because the model id resolves at use-time and the upstream provider may have silently swapped the weights.

Brief should *not* model traces. Brief *should* model the inputs to reproducibility — at minimum: model id, model-version pin (when known), and a content hash of the brief itself. A `brief.id` (sha256 over the canonicalized parse tree) would let any downstream emitter stamp output with provenance. That is one frontmatter field with high leverage.

### Lifecycle and governance: the version field is decorative

`version: "0.1.0"` is in the schema and is, as far as I can tell, never enforced. aichat does not declare a compatibility contract, has no schema-evolution story, no deprecation markers, no ownership/provenance metadata, no security posture (sandbox? permission tier? allowed network egress?), no rollback path.

Software engineering settled this in the 90s with semver, package manifests, and signed artifacts. ML tooling keeps re-learning it. The current best-in-class — DSPy's compiled program artifacts, LangGraph's checkpoint schemas, MCP's protocol-version negotiation — all carry an explicit compatibility field that the runtime checks on load. aichat does not.

For brief, the implication is simple: ship a `version: "1"` that *means* something, document the compatibility rule (additive-only within a major), and put a `brief.id` and `authored_by` in the schema now while it is cheap. You will thank yourself in two years.

### Composition: aichat agents are flat, the field is graphs

aichat agents do not call other aichat agents. There is no planner/executor split, no hand-off, no subagent registry beyond `agents.txt`. If you want multi-agent, you script it from outside.

Meanwhile the field has converged, loudly, on agent graphs. AutoGen made the conversational-multi-agent pattern legible. LangGraph made it a typed state machine. MetaGPT showed role-decomposition. CrewAI productized the pattern. Anthropic's own subagent / orchestrator-worker pattern is now in Claude Code. ReAct, Reflexion, and Voyager all assume a loop with explicit roles even when there is one model behind it.

I would not push aichat to add an orchestrator — that is a runtime decision and aichat is well within its rights to stay flat. But brief, as a *spec* that emits to many runtimes, has a choice: model composition now, or paint itself into a corner.

My read: brief should model **delegation hints, not topology.** A frontmatter field along the lines of `delegates_to: [reviewer, test-writer]` (names, not implementations) lets each emitter decide how to wire it — a Claude subagent, an aichat sibling agent, or a no-op for runtimes that do not support it. Modeling actual graph topology (LangGraph-style) in a 60-second-to-author Markdown file is the wrong scope.

### The non-stationarity problem nobody wants to admit

Here is the thing that will bite every agent spec, including aichat and brief, that nobody has solved: *the model-behavior contract is not stationary*. A spec authored against Sonnet 3.5 in June 2024 will silently miscalibrate against Opus 4.7 in 2026. Constraints that worked as bullet points in `## Rules` start getting ignored. Sacred regions start getting "improved." The same `temperature: 0` produces different distributions because the underlying weights changed under a stable model id.

The honest response is: a *durable* agent spec needs to record what it was *calibrated against*, not just what it targets. That means:

- Pinning at minimum a `model_family + behavioral_snapshot_date` or, ideally, a content hash of a small canary eval that the agent is known to pass.
- Acknowledging that re-calibration on model upgrade is a maintenance task, not a one-time authoring task.
- Carrying a small set of *tripwires* — golden examples whose drift signals the spec needs review.

aichat has no slot for this. Brief does not need one either, *yet*, but the team should reserve the namespace. A `calibrated_against:` frontmatter field (initially documentary, eventually enforceable) is the cheapest forward-compatible move.

## What I would tell the brief team

Distilled, in priority order. Each is a recommendation about what the *brief format* should express, distinguished from what stays a target-specific concern.

1. **Keep the constraint taxonomy. Do not let aichat flatten it back out at the source.** Hard / Soft / Ask-First / Sacred is the single most valuable thing brief has. aichat will inline it as prose, and that is fine — that is the emitter's job. The format-level invariant is that the structured form is the source of truth, and emitters are lossy projections.

2. **Add a `brief.id` (content hash) and a real `version` semantics now.** While the format is young and the cost is one parser change. Every emitted artifact should be stampable with the brief's hash. This is the cheapest provenance/reproducibility win available, and it lets aichat (which has no provenance story) inherit one for free.

3. **Model coarse-grained capability requirements, not tools.** Add a `capabilities:` frontmatter field with a small controlled vocabulary (`fs.read`, `fs.write`, `net.read`, `net.write`, `exec`, `secrets`). Each emitter maps that to its runtime's tool selector — for aichat, it becomes a `use_tools:` hint; for Claude Code, an `allowed-tools:` constraint. This is the right level: brief authors should not be writing JSON Schema for tool I/O, and emitters should not be guessing which tools are needed.

4. **Distinguish reference context from working corpus in the `context:` field.** Either two fields or a tagged shape (`{path: ..., role: reference|corpus}`). aichat collapses to `documents:`, Claude Code uses `@`-mentions, others differ. Preserving the distinction at the source is cheap; recovering it later is not.

5. **Model delegation by name, not by graph.** A `delegates_to: [name, ...]` field that names other briefs. Emitters decide how to wire it. Do not model topology, do not model planner/executor, do not become LangGraph. This is the YAGNI-respectful version of "brief should know about composition."

6. **Reserve a `calibrated_against:` namespace, even if only documentary in v1.** Model id, family, and authoring date. It costs nothing now and pays off the first time a model upgrade silently drifts behavior. Aichat will not consume it; that is fine — it lives in the brief and surfaces in `brief diff` and `brief validate` output.

7. **Resist modeling: tool I/O schemas, memory architecture, trace formats, eval suites, sandbox policy.** These are real and important and *not* brief's job. They are runtime concerns. If brief tries to model them, it stops being the 60-second-to-author artifact in the project's first hard constraint, and starts being PDL-with-extra-steps. The discipline is: brief expresses *intent and constraint*, runtimes express *execution and enforcement*, and the emitter is the contract between them.

The aichat target is a fine first non-trivial backend. It will exercise multi-file emit, registry mutation, and the discipline of lossy projection. It will also surface, by what it cannot represent, exactly which fields brief itself is missing. Use it that way.
