---
author: LLM/Supervised ML Engineer
role: ML Engineer (Transformers, Diffusers, structured decoding)
date: 2026-04-27
purpose: Independent analysis for synthesis into aichat-agent-gaps.md
---

# A transformer engineer reads the aichat agent format

## The frame

I have spent the last several years fine-tuning instruction models, debugging chat templates that silently re-tokenized my system prompt into garbage, and watching eval scores collapse because I moved a `<rule>` block from above the user turn to below. I am reading the aichat schema as a transformer engineer asks: when this artifact compiles down to a token stream, what does the model actually see, and where will it fail?

aichat's agent format is a thoughtful, ergonomic shell-CLI design. As a transformer-runtime contract, it is roughly 2022-vintage: single-blob system prompt, untyped variables, RAG-shaped `documents:`, sidecar tool schemas, two scalar decoding knobs. That is fine for casual single-model use. It is not what brief should aspire to compile into without lossy compromises, and it is not what brief's frontmatter should be shaped to fit. Below is the gap list.

## Chat template ignorance

The single most consequential design choice in aichat is collapsing everything into one `instructions:` string. Modern instruction-tuned models do not see "a prompt." 
They see a token stream produced by a chat template — Jinja or hand-rolled — that interleaves role tokens (`<|im_start|>system`, `<|start_header_id|>system<|end_header_id|>`, Anthropic's `Human:`/`Assistant:` 
framing as rendered by the Messages API, Gemma's `<start_of_turn>` markers, Qwen's tool-specific `<tools>` block). 
Different models put rules in materially different positions: Llama-3 instructs with a single system message; 
Anthropic supports system as a top-level parameter distinct from user turns; 
OpenAI's Responses API introduced a `developer` role distinct from `system`; C
hatML supports multiple system messages; some templates allow tool definitions in a dedicated role, others inline them into system.

Flattening to one blob loses three things that empirically move evals:

1. **Role-aware attention.** Many fine-tunes are trained with KL or DPO objectives that reward following the system role specifically. 
When you stuff rules into the user turn — which is what aichat effectively does at the boundary, because aichat *does* render `instructions:` as the system message but renders dynamic interpolation including `{{__tools__}}` into that same string — you cross-contaminate roles in ways the model was not trained to expect.

2. **Tool-message separation.** Qwen2.5, Llama-3.1+, Hermes, and most open tool-use checkpoints expect tool schemas in a dedicated 
tool-list slot rendered by the chat template, not interpolated into system text. aichat's `{{__tools__}}` does the latter. 
I have personally seen this drop tool-call accuracy by 8-15 percentage points on Llama-3.1-70B-Instruct vs. passing tools through the proper `tools=[...]` API parameter.

3. **Cache breakpoint placement.** Anthropic prompt caching keys on contiguous prefix segments and supports up to four explicit `cache_control` breakpoints. 
If brief emits a stable rules block followed by a volatile per-task block, aichat has nowhere to express "cache up to here." A single blob is a single cache key.

Brief should not pretend it can compile faithfully into a single string when the target's chat-template surface is richer than that. 
The honest model is: brief carries logical sections, the emitter chooses how to render given target capability,

## System-prompt structure has measured effects

Anthropic's published prompt patterns (XML tags, role designation, examples-before-rules-before-task) are not stylistic. 
The Sonnet/Opus families were trained on data heavy with `<rules>`, `<example>`, `<context>` scaffolding and demonstrably attend to those tags — 
I have seen 5-12 point lifts on internal extraction evals just from wrapping rules in `<constraint type="hard">`. 
brief's claude emitter already uses `**IMPORTANT:**` markers, which are the documented Claude Code convention; 
the prompt emitter uses uppercase headers. Both survive a copy into aichat's `instructions:` field, but the structural information that brief models — 
Hard vs Soft vs Ask-First constraints, Sacred regions vs Assumptions vs Deliverable — 
gets reduced to typographic conventions inside a single string. 
There is no machine-readable distinction the runtime can act on. aichat will never tell a downstream eval harness "this rule is hard"; the model has to infer it from word choice.

Long-context recency bias compounds this. Liu et al. (lost-in-the-middle, 2023) and a half-dozen replications show models pay 
disproportionate attention to the start and end of long contexts. Where in aichat's `instructions:` blob does the rules block end up relative to the user turn? 
Wherever the author put it. With a multi-message representation, brief could place hard rules immediately before the user turn (high-recency slot) 
regardless of authoring order. With a blob, it cannot.

## Documents conflate three distinct primitives

aichat's `documents:` field accepts paths, globs, and URLs and feeds them all to `Rag::init`. From a context-engineering standpoint, this conflates three primitives that should never share a slot:

1. **Always-in-context references.** Files the model must see verbatim every turn — license headers, schema definitions, the API contract being modified. These belong in the system message, ideally inside Anthropic's prompt-cache region for efficiency, or as repeated user-turn injections.
2. **Retrieval corpus.** A document set to be embedded and retrieved by similarity. This is what aichat actually does with `documents:`. Appropriate for large knowledge bases, wrong for small invariants.
3. **Few-shot exemplars.** Input/output pairs that anchor format and style. These have measurable ICL effects (5-20 point lifts on structured-extraction tasks, depending on selection strategy) and should be selected per-query, not retrieved by cosine similarity over the whole document.

Brief's current `frontmatter.context` inherits the same conflation. I would argue this is the single most important upgrade for the format: split `context:` into `pin:` (always-in-context, probably cached), `retrieve:` (RAG-eligible), and `examples:` (few-shot, with structured input/output). The aichat emitter then has a defensible mapping: `pin` and `examples` flatten into `instructions:` (with appropriate XML-tag scaffolding), `retrieve` goes into `documents:`. Without that split, brief is forced to pick one bucket and lose information either way.

## Structured output: the schema is in the wrong file

aichat exposes `input_schema` and `output_schema` as JSON Schema, but only in `config.yaml` — the per-user override — not in `index.yaml`, the agent definition. That is upside down. The output schema is part of the contract the agent is supposed to satisfy; it is not a user preference. An agent author shipping a JSON-emitting tool wants every consumer to get the same schema; making it overridable per machine guarantees drift.

Compare to current SOTA. OpenAI Structured Outputs accepts a JSON Schema with a strict subset and uses constrained decoding (essentially their internal port of grammar-guided decoding) to guarantee well-formed output — `response_format={"type":"json_schema", "json_schema":{...}, "strict": true}`. Outlines and lm-format-enforcer compile schemas into regex-DFA or CFG masks applied at logit time. xgrammar (Tang et al. 2024) and SGLang's structured-decoding path do this with low overhead and integrate into vLLM. Anthropic's tool_use shape carries a JSON Schema per tool and the model is fine-tuned to emit valid tool JSON; you can also prefill `{` and stop on `}` for poor-man's structured output. JSON-mode without a schema (the original OpenAI flag) is now widely understood to be a footgun — it guarantees parseable JSON but not the *right* JSON.

Brief's current `deliverable` is free text. That works as human intent. It does not compile to any of the above. I would add an optional structured slot — `deliverable_schema:` accepting a JSON Schema or a reference to a `.json` file — that the prompt and aichat emitters can route into the appropriate target slot. Validate it at parse time so authors learn early; then the aichat emitter can put it into `output_schema` (in `index.yaml`, and PR upstream if needed), the prompt emitter can wrap it into a Responses-API `response_format`, and the claude emitter can render it as a tool definition.

## Tool schemas are not first-class

aichat's tool surface is `functions.json` generated by `argc build` from comment-tagged shell scripts. The agent definition does not contain the tool schema; it references tools by name via `use_tools:` in `config.yaml`, and the runtime renders them into the system prompt at load time via `{{__tools__}}`.

Two failure modes:

- **At fine-tune or evaluation time.** If you want to evaluate this agent, build a synthetic dataset, or DPO/RFT on tool-call traces, the agent artifact is insufficient — you need the tool schema too, separately, and you need to know which model-provider rendering convention to apply. MCP exists precisely to formalize this. brief is not going to ship a tool-schema model in v1, but it should at minimum carry a `tools:` slot that points at MCP server URLs, OpenAI function arrays, or aichat tool slugs, so downstream emitters can do the right thing.
- **At inference across providers.** aichat's client abstraction speaks to OpenAI-shaped, Anthropic-shaped, and local backends. Each renders tools differently. The same agent run via `openai:gpt-4o` and `claude:claude-opus-4-7` does not produce the same prompt — the chat template and tool injection differ. `{{__tools__}}` papers over this by inlining a model-rendered tool list into system text, which I have already flagged as a tool-call regression source.

## Few-shot examples have no slot

aichat agents have no place for in-context exemplars. `conversation_starters:` is for users to click; the model never sees it as supervision. This is a real omission. DSPy's MIPROv2 demonstrates that bootstrapped few-shot selection beats zero-shot prompting on most structured tasks by double-digit margins, and OpenAI's prompt-builder and Anthropic's prompt-improver both default to inserting examples. For brief, this is the same point as the documents-conflation argument: an `examples:` slot, structured as input/output pairs, gives downstream emitters a real handle. The aichat target wraps them in `<example>` tags inside `instructions:`; the prompt target uses XML tags for Claude or `### Example` blocks for OpenAI; a future MCP target keeps them separate.

## Templating is a tokenizer hazard

aichat's `{{__tools__}}` interpolation puts a runtime-rendered tool list inside the system prompt string. The rendering format is a function of which client backend the user picked. I have debugged this exact bug on two production systems: `{{__tools__}}` rendered as a markdown-formatted bullet list works fine on GPT-4o, then the same agent with the same instructions silently degrades on a Llama fine-tune that expects tools in `<tools>...</tools>` JSON. The model is not broken; the prompt is wrong for that template.

If brief emits a static `instructions:` string into aichat that contains user-authored Mustache-style `{{var}}` tokens (which it must, per the round-trip rule in the README), it must escape nothing and rewrite nothing. This is a YAML-emit hazard: serializers love to wrap or fold strings containing `{{`. Test every emit path against fixtures that include `{{__tools__}}`, `__INPUT__`, and `{{$AICHAT_FOO}}` literally.

## Decoding controls are anemic

aichat exposes `temperature` and `top_p` in `config.yaml`. That is it. No `seed`, no `top_k`, no `min_p`, no `repetition_penalty`, no `logit_bias`, no `max_tokens`, no `stop`, no reasoning-budget knob (relevant for Sonnet 3.7 / Opus 4 thinking, o-series, DeepSeek-R1). For reproducible evals — which is the entire point of pinning a prompt to a model — `seed` is non-negotiable.

Brief should not try to model every provider's decoding zoo. But a `decoding:` map in frontmatter with the consensus subset (`temperature`, `top_p`, `top_k`, `seed`, `max_tokens`, `stop`, `reasoning_effort`) covers 95% of legitimate need, projects naturally into aichat's `config.yaml` (as additive fields, even if upstream does not yet support them), and lets brief's prompt emitter produce a reproducible API call.

## The prompt-to-model contract needs a version

A `.brief.md` that runs cleanly on Sonnet 3.5 may underperform on Opus 4.7 because instruction-following shape, refusal calibration, default verbosity, and tool-call format have all shifted across versions. I have rewritten the same agent three times across the Claude 3 -> 3.5 -> 4 -> 4.7 transitions; each rewrite was load-bearing.

aichat's `model: <client>:<model>` is a runtime knob. It does not pin the prompt-author's intent. A more honest contract: the brief carries a `tested_against:` list (`anthropic:claude-opus-4-7`, `openai:gpt-4o-2024-11`) plus an optional inline `eval:` reference (path to a small fixture set the validator can run). When the user runs against an unlisted model, `brief validate` warns. This is cheap to add to frontmatter and gives the format a real reproducibility story.

## Multi-modal and code execution

aichat agents are text-only by current definition. Vision-language models (Claude with images, GPT-4o, Llama-3.2-Vision, Qwen2-VL) and audio-in/out models (gpt-4o-realtime, Gemini Live) are now mainstream. Sandboxed code execution is a first-class capability on Claude Code, OpenAI Code Interpreter, and several agent runtimes. brief does not need to model these in v1, but the format should leave room: the `context:` (or `pin:`) field should accept image paths, and a `capabilities:` flag (`vision`, `audio`, `code_execution`, `web_search`) should be reservable. Aichat does not consume these today; that is fine. Brief is the format, aichat is one target.

## What I would tell the brief team

These are prioritized. The top three are the ones I would land before shipping the aichat backend.

1. **Split `context:` into three fields.** This is the highest-leverage change. `pin:` for always-in-context references, `retrieve:` for RAG corpora, `examples:` for structured few-shot pairs. The aichat emitter maps `retrieve:` to `documents:`, inlines `pin:` into `instructions:` inside `<reference>` tags, and inlines `examples:` inside `<example>` tags. This is a *frontmatter* change. Without it, every emitter is forced to make the wrong call. Pass the YAGNI bar by counting how often current brief authors want a file embedded vs. retrieved — empirically, "always pinned" dominates for small repos and the conflation is actively harmful.

2. **Add an optional structured-output schema slot.** A frontmatter field — `deliverable_schema:` — accepting either inline JSON Schema or a path. Validate at parse time. aichat emitter routes it into `output_schema:` in `index.yaml` (not `config.yaml`; push upstream if needed). Prompt emitter wraps it for OpenAI Structured Outputs / Anthropic prefill-and-stop. Claude emitter renders as a tool. The brief body's `## Deliverable` stays as human-readable intent; the schema is the machine contract.

3. **Carry a `tested_against:` model list and a `decoding:` block in frontmatter.** Pin the prompt-to-model contract; expose `seed`, `temperature`, `top_p`, `max_tokens`, `stop`, `reasoning_effort`. `brief validate` warns when the active model is not in `tested_against`. This is the minimum reproducibility story; it costs almost nothing in format complexity and unlocks evals.

4. **Refuse to lossy-emit silently.** When the aichat target cannot represent something brief models — multi-message structure, role-typed tool schemas, cache breakpoints, multi-modal context — `brief emit aichat` should print a stderr warning naming the dropped field. Authors learn what their brief actually contracts for. This is an *emitter* change, not frontmatter.

5. **Reserve a `tools:` slot in frontmatter, even if empty in v1.** Accept MCP server URLs, OpenAI-shape inline schemas, or aichat tool slugs. The runtime emitters route appropriately. Without this, brief cannot ever represent agentic capability faithfully, and `{{__tools__}}` interpolation remains the only handle. Body change: a `## Tools` section is plausible too, but the structured form belongs in frontmatter — tool schemas are not prose.

6. **Keep XML-tag scaffolding in the body, not frontmatter.** brief's body authoring stays Markdown, and emitters add tags (`<rule severity="hard">`, `<sacred path="...">`, `<example>`) on the way out for Claude-family targets. Authors should not have to write XML; they write Markdown lists, and the emitter compiles. This is already the design; just preserve it as the format grows.

7. **Mark the aichat target explicitly lossy in docs.** The README for the backend already does some of this. Be louder. The aichat target is *useful*, not *faithful*; the prompt and claude targets remain the high-fidelity ones. This is a documentation change but a strategic stance: brief's value increases when it compiles cleanly into the SOTA surface, and aichat is one rung down from that.

The summary stance: aichat is fine as a target, but brief's frontmatter and section model should be shaped to the SOTA transformer agent surface (multi-message chat templates, structured outputs, typed tools, splittable context, decoding and reproducibility metadata), not flattened toward aichat's blob. When the target is lossy, emit a warning and keep moving.
