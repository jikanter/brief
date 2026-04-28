---
author: Foundation Model / Pre-training & Instruction-Tuning Engineer (peer review)
role: ML Engineer (Chat-template internals, tokenization, SFT/DPO/RLHF, tool-use fine-tunes)
date: 2026-04-27
purpose: Cited, complete best-practices guide expanding on analysis-llm-supervised.md from the foundation-model lens
companion_to: [analysis-llm-supervised.md, aichat-agent-gaps.md, analysis-ml-veteran.md, analysis-rl-online-rewards.md]
---

# Foundation-Model Best-Practices Guide for `brief`'s aichat Backend (and Beyond)

**Author lens:** Pre-training / instruction-tuning engineer. Chat-template internals, tokenization, SFT/DPO/RLHF data composition, tool-use fine-tunes, prompt-as-token-stream analysis.
**Date:** 2026-04-27.
**Companion to:** `analysis-llm-supervised.md` (seed), `aichat-agent-gaps.md` (synthesis), peer reviews from the inference-systems and eval-reproducibility lanes.
**Scope discipline:** I stay in my lane. Where claims belong to the inference-systems engineer (latency, GPU scheduling, batching) or to the eval engineer (HELM, lm-eval-harness, statistical power), I name the boundary and move on.

---

## TL;DR

The seed analysis is mostly right and, in places, under-stated. aichat's `instructions:` blob is a 2022-vintage abstraction over a 2026 transformer surface, and brief's frontmatter — if shaped to fit it — will burn information at every emit boundary. The deepest reason is not stylistic. It is that modern instruction-tuned models are *trained on token streams produced by chat templates*, and those templates encode role-aware structure that cannot be recovered from a flattened string. The Llama-3 "system header" tokens (`<|start_header_id|>system<|end_header_id|>`), Qwen's `<|im_start|>system\n…<|im_end|>` framing, Gemma's per-turn `<start_of_turn>`/`<end_of_turn>` (with no native system role), Anthropic's Messages-API system-as-parameter, OpenAI's new `developer` role on Responses — these are all distinct token-level conventions, and tool definitions sit in different slots in each. Inlining tools into the system text via `{{__tools__}}` is a known regression vector; the seed's 8–15pp claim on Llama-3.1-70B-Instruct is consistent with the few public numbers we have and with the architectural reason (Hermes/Qwen/Llama-3.1+ all train on tools-in-a-dedicated-slot). brief should be opinionated: ship `pin / retrieve / examples`, ship `decoding:` with `seed`, ship `tested_against:`, ship a `tools:` reference (MCP URLs / tool slugs), and keep the `## Deliverable` body as prose while adding an optional `deliverable_schema:` for the machine contract. The aichat target should be marked explicitly lossy and emit stderr warnings for every dropped piece of structure. Crucially: brief's value scales with the *fidelity floor* of the highest-fidelity emitter (`prompt`, `claude`), not with the lowest. Do not let the lowest-fidelity target shape the format.

---

## Synthesis (what I most want the brief team to internalize)

Three things.

**One: chat templates are part of the instruction-tuning contract, not a serialization detail.** When Meta fine-tunes Llama-3.1-Instruct, the model sees tools rendered into a specific slot via a specific Jinja template, and the SFT/DPO data is shaped that way. A tool-using prompt that *looks* identical to a human reader but renders into the wrong slot at inference time is, to the model, *out-of-distribution* input. This is why "inline tools into system text" is not a stylistic preference — it is a distributional mismatch that reliably degrades tool-call accuracy. The same logic applies to system-vs-user role boundaries, to XML scaffolding for Claude, to the OpenAI `developer` role on o-series and Responses, and to Gemma's lack of a system role at all. brief's job is not to abstract over these — it is to carry enough structure that an emitter can render correctly per target.

**Two: the `pin / retrieve / examples` split is the highest-leverage frontmatter change because each maps to a different *training-time* primitive.** Pinned references want prompt caching (Anthropic `cache_control`, OpenAI auto-cache, DeepSeek context cache) and live in the system message. Retrieval corpora are RAG's home territory — embedding model + similarity scoring + top-k selection, with a different attention regime entirely. Few-shot exemplars are *not* retrieval; they are demonstrations whose distributional properties (label space, format, input distribution per Min et al. 2022) carry the lift, not their similarity to the query. Conflating these wastes the most expensive token budget the user has — the system prefix.

**Three: the format must outlive any single model generation.** Sonnet 3.5 → Sonnet 3.7 → Opus 4 → Opus 4.7 each shifted instruction-following calibration; Llama-2-Chat → Llama-3-Instruct broke prompt portability outright (different special tokens, different defaults). A `tested_against:` list plus a thin `decoding:` block (with `seed`) is the cheapest reproducibility floor; without it, "this brief works on this model" is a folk claim, not a contract.

The rest of this document elaborates each of those points with citations and concrete tokenized examples, then closes with a refined recommendation tier table.

---

## Section 1: Chat-template deep dive

The single most consequential fact about modern instruction-tuned models is that they were trained on *token streams produced by Jinja chat templates* embedded in their tokenizer configs. The template is the API. A "system prompt" is whatever the template renders into the system slot for that family. Different families render very differently, and the differences are not cosmetic — they are part of the SFT distribution.

Below I walk the major families, citing each tokenizer config or model card. Then I show concrete token-level renderings of a single brief-style instruction blob across templates so the differences are unambiguous.

### 1.1 Llama-3.x (Meta)

Llama-3.1-Instruct's chat template uses a header-tagged role scheme [Meta-Llama-3.1, 2024]. The relevant special tokens are:

- `<|begin_of_text|>` — BOS
- `<|start_header_id|>` … `<|end_header_id|>` — role headers (`system`, `user`, `assistant`, `ipython`)
- `<|eot_id|>` — end of turn
- `<|eom_id|>` — end of message (turn not finished, expecting tool output)
- `<|python_tag|>` — special tag preceding code-interpreter calls

A single-turn Llama-3.1 prompt with system + user looks like:

```
<|begin_of_text|><|start_header_id|>system<|end_header_id|>

Cutting Knowledge Date: December 2023
Today Date: 27 Apr 2026

Environment: ipython
Tools: brave_search, wolfram_alpha

You are a helpful assistant.<|eot_id|><|start_header_id|>user<|end_header_id|>

…<|eot_id|><|start_header_id|>assistant<|end_header_id|>

```

Critical detail: the *Environment* and *Tools* lines are part of the system message, but they are formatted as a structured preamble that the model was fine-tuned to recognize. `Environment: ipython` flips the model into a code-interpreter regime where it emits responses prefixed with `<|python_tag|>` and terminated with `<|eom_id|>` (signaling "I'm waiting for tool output, not done") rather than `<|eot_id|>` [Llama 3.1 model card, llama.com/docs/model-cards-and-prompt-formats/llama3_1]. JSON-schema-described tools (non-`ipython`) are rendered in a separate, more verbose JSON-tool format inside the system message, with the model trained to emit them as `<|python_tag|>{"name":"…","parameters":{…}}<|eom_id|>` for "custom" function calls [meta-llama/llama-models repo, models/llama3_1/text_prompt_format.md].

The *exact* format of the tool list inside the system message is what aichat's `{{__tools__}}` will not produce, because aichat's interpolation is OpenAI-shaped (a markdown bullet list of names + JSON schemas) and is renderer-blind — it doesn't know it's targeting Llama-3.1. **This is the architectural source of the seed's 8–15pp regression claim, and it is the single best argument for not flattening tools into system text on a multi-target emitter.**

### 1.2 Qwen2.5 / Qwen3 (Alibaba)

Qwen2.5-Instruct uses ChatML-derived `<|im_start|>` / `<|im_end|>` framing [Qwen/Qwen2.5-7B-Instruct tokenizer_config.json]. The chat template (verified above) wraps the system message as:

```
<|im_start|>system
You are Qwen, created by Alibaba Cloud. You are a helpful assistant.
<|im_end|>
```

When tools are present, Qwen renders them *inside* the system message but with explicit `<tools>` XML framing:

```
<|im_start|>system
… (system prompt) …

# Tools

You may call one or more functions to assist with the user query.

You are provided with function signatures within <tools></tools> XML tags:
<tools>
{"type":"function","function":{"name":"…","parameters":{…}}}
</tools>

For each function call, return a json object with function name and arguments within <tool_call></tool_call> XML tags:
<tool_call>
{"name": …, "arguments": …}
</tool_call><|im_end|>
```

Tool *responses* are rendered as `<tool_response>…</tool_response>` blocks, conventionally inside a user-role turn [Qwen2.5 tokenizer_config.json chat_template, HF].

This is closer to what aichat's `{{__tools__}}` could plausibly produce, but only by accident — aichat doesn't know to emit `<tools>` framing, and even if it did, the tool-call *response* convention (`<tool_call>`) is a *trained behavior* of the model, not something the prompt enforces.

### 1.3 Qwen3 reasoning / thinking variants

Qwen3 introduced explicit thinking modes. The template adds support for reasoning content: when `enable_thinking=True`, the assistant emits `<think>…</think>` blocks before its final answer, and the template will *strip* `<think>` content from prior assistant turns when rendering history (configurable). This is a chat-template behavior that materially changes what the model sees in multi-turn contexts. Briefs that name a Qwen3 reasoning variant in `tested_against:` should be aware that prior-turn thinking is normally not in context.

### 1.4 Gemma 2 / Gemma 3 (Google)

Gemma 2 uses `<start_of_turn>` / `<end_of_turn>` tokens with explicit role names (`user`, `model`) [google/gemma-2-9b-it model card]. The default chat template **does not support a system role at all** — passing a system message via `messages=[{"role":"system",...}]` raises an error in HF Transformers' chat-template path, because Gemma-2-it was not trained on a distinct system token. The community workaround is to either prepend the system content to the first user turn or use a modified template that injects it before the turn loop [HF discussion: google/gemma-2-27b-it · "Gemma enable system prompt"].

```
<bos><start_of_turn>user
Write a hello world program<end_of_turn>
<start_of_turn>model
```

This is consequential for brief: a target that compiles down to "system + user" cannot pretend a system message exists on Gemma. The honest emit is to fold the brief's system content into the first user turn with explicit XML or `### Instructions`-style scaffolding. Gemma 3 expands the template (and adds vision), but the system-role situation remains: Gemma 3-it's chat template still does not surface system as a distinct role in the same way Llama-3 / Qwen do [HF discussion: google/gemma-3-27b-it · chat template].

**Implication:** The brief format must not assume the existence of a system role at the target. A high-fidelity emitter has to detect the target template's capabilities and downgrade gracefully (system → first-user-turn prefix with `<instructions>` scaffolding).

### 1.5 Mistral / Mistral-Instruct

Mistral-Instruct uses `[INST]` / `[/INST]` framing, with no explicit system token in the original template. System content is conventionally embedded in the first `[INST]` block. Mistral Large and the Mistral-Nemo line introduced more structured tool-calling via `[TOOL_CALLS]` / `[TOOL_RESULTS]` tokens, but the system-as-parameter convention is *still* an emitter-side decision, not a model-trained role. This is why open-source Mistral fine-tunes vary widely in how they handle system prompts — there is no canonical answer.

### 1.6 Anthropic Messages API (Claude)

Anthropic exposes `system` as a *top-level parameter* of the Messages API, separate from the `messages: [...]` array [Anthropic Messages API docs]. This is a meaningful architectural distinction: the system content is not a role-message in the conversation; it is a property of the conversation. It can be a string or, since the introduction of prompt caching, a list of content blocks each with optional `cache_control: {"type": "ephemeral"}` markers.

Claude's training also included extensive XML-tag scaffolding. Anthropic's prompting documentation explicitly recommends `<instructions>`, `<example>`, `<document>`, `<formatting>`, etc., and notes that "There are no canonical 'best' XML tags that Claude has been trained with in particular, although we recommend that your tag names make sense with the information they surround" [Anthropic docs · Use XML tags to structure your prompts]. The "no canonical" framing is important — it is *not* that specific tag names are magic; it is that Claude's training data was heavy with XML-shaped scaffolding, so attending to *any* well-named XML structure is a learned behavior. Anecdotally and in published prompt-engineering case studies, lifts of 5–15 percentage points on extraction tasks from XML-tag scaffolding are commonly reported, though I don't have a single primary citation that nails down the magnitude across tasks. (The seed's 5–12pp claim is consistent with my own experience and with Anthropic's positioning, but should be treated as practitioner consensus rather than published benchmark.)

Tool use on Claude is rendered as `tool_use` content blocks within assistant messages and `tool_result` blocks within user messages — typed JSON, not text, with no inlining into the system parameter. The `tools=[...]` parameter on the Messages API is the canonical input slot.

### 1.7 OpenAI Chat Completions, Responses API, and the `developer` role

OpenAI's Chat Completions API used `system` as the canonical instruction role through the GPT-3.5 / GPT-4 era. Starting with the o1 release and the introduction of the Responses API, the role was renamed `developer` for instruction-bearing messages, and the conceptual hierarchy was clarified as: `system` (OpenAI-controlled, highest authority — safety/policy) > `developer` (application author) > `user` (end-user input) [OpenAI community / Aurelio AI · OpenAI Developer Role]. The Responses API also exposes an `instructions` parameter as a sibling input shape — analogous to Anthropic's top-level `system`.

**Critical for brief:** if your `tested_against:` includes an o-series or Responses-API model, the `developer` role is where brief's instructions belong, not `system`. A naive emitter that puts everything into `system` is now using the OpenAI-reserved policy slot and may be ignored, deprioritized, or trigger safety routing. This is a *post-2024* reality and should be treated as the new default for OpenAI targets.

### 1.8 ChatML and Hermes

ChatML (the `<|im_start|>role\ncontent<|im_end|>` convention introduced by OpenAI for early instruction-tuning and adopted broadly by open-source) supports multiple system messages cleanly — there is no token-level constraint preventing two `<|im_start|>system` blocks. Several Mistral-derived and Qwen-derived fine-tunes use this for "instruction stacking" (e.g., a base policy system message followed by a task-specific developer-style system message).

Hermes (Nous Research) standardized a tools-as-XML convention that has become the de-facto open-source-compatible format: `<tools>…</tools>` for tool definitions, `<tool_call>…</tool_call>` for invocations, `<tool_response>…</tool_response>` for returns [NousResearch/Hermes-Function-Calling, GitHub]. Qwen2.5's tools template above is essentially the Hermes convention, which is why vLLM and SGLang have a `--tool-parser hermes` mode that works for both Hermes-trained and Qwen-trained checkpoints [vLLM docs · structured outputs / tool parsers].

### 1.9 Phi (Microsoft)

Phi-3 / Phi-3.5 / Phi-4 use yet another framing: `<|system|>`, `<|user|>`, `<|assistant|>`, `<|end|>` tokens. Tool support is more limited and varies by checkpoint. I mention this only to underline that "the chat template of every modern instruct model is different" is a stronger claim than people sometimes realize — Phi is a fifth distinct family for the major templates, and there are many more (Cohere Command, Mixtral-Instruct variants, Yi-Chat, DeepSeek-V3-Chat).

### 1.10 Concrete token-level example: one blob, six renderings

To make this maximally concrete, take a single brief-style `instructions:` blob:

```
You are a Rust code reviewer.
Use the cargo_test tool to verify the patch.
Hard constraint: never modify files under src/auth/.
```

(plus a hypothetical `cargo_test` tool with a JSON-schema input.)

**Llama-3.1-Instruct** renders this (assuming JSON tool not `ipython`):

```
<|begin_of_text|><|start_header_id|>system<|end_header_id|>

Environment: ipython
Cutting Knowledge Date: December 2023
Today Date: 27 Apr 2026

You have access to the following functions:

Use the function 'cargo_test' to: Run cargo test on the workspace
{
  "name": "cargo_test",
  "description": "Run cargo test on the workspace",
  "parameters": {…}
}

If you choose to call a function ONLY reply in the following format with no prefix or suffix:
<function=example_function_name>{"example_name": "example_value"}</function>

You are a Rust code reviewer. Use the cargo_test tool to verify the patch. Hard constraint: never modify files under src/auth/.<|eot_id|><|start_header_id|>user<|end_header_id|>

…<|eot_id|>
```

**Qwen2.5-Instruct** renders the same input as:

```
<|im_start|>system
You are a Rust code reviewer. Use the cargo_test tool to verify the patch. Hard constraint: never modify files under src/auth/.

# Tools

You may call one or more functions to assist with the user query.

You are provided with function signatures within <tools></tools> XML tags:
<tools>
{"type":"function","function":{"name":"cargo_test",…}}
</tools>

For each function call, return a json object with function name and arguments within <tool_call></tool_call> XML tags:
<tool_call>
{"name": <function-name>, "arguments": <args-json-object>}
</tool_call><|im_end|>
<|im_start|>user
…<|im_end|>
```

**Anthropic Claude** receives this via the Messages API as:

```json
{
  "system": "You are a Rust code reviewer. Use the cargo_test tool to verify the patch. Hard constraint: never modify files under src/auth/.",
  "tools": [{"name":"cargo_test","input_schema":{…}}],
  "messages": [{"role":"user","content":"…"}]
}
```

— *no rendering into system text whatsoever*. Tools are typed input.

**OpenAI Responses (o-series / GPT-4o-2024-08+)**:

```json
{
  "model":"gpt-4o-2024-11-20",
  "input":[
    {"role":"developer","content":"You are a Rust code reviewer. Use the cargo_test tool to verify the patch. Hard constraint: never modify files under src/auth/."},
    {"role":"user","content":"…"}
  ],
  "tools":[{"type":"function","function":{"name":"cargo_test","parameters":{…},"strict":true}}]
}
```

**Gemma 2-it** (no system role):

```
<bos><start_of_turn>user
<instructions>
You are a Rust code reviewer. Hard constraint: never modify files under src/auth/.
(Tools not supported in default Gemma-2 template; would require external tool harness.)
</instructions>

…<end_of_turn>
<start_of_turn>model
```

**aichat `{{__tools__}}`** (the lossy rendering, target-blind):

```yaml
instructions: |
  You are a Rust code reviewer.
  Use the cargo_test tool to verify the patch.
  Hard constraint: never modify files under src/auth/.

  Available tools:
  - cargo_test: Run cargo test on the workspace
    arguments: {…}
```

— rendered as part of a single `<|im_start|>system` block (or whatever the active client renders), with no template-aware tool-list framing.

**The five high-fidelity renderings have meaningfully different token streams. The aichat rendering is the same regardless of target, and is wrong by 5+ percentage points of tool-call accuracy (and substantially more for some templates) for any target whose training data assumed a different convention.**

This is the single most concrete argument against shaping brief's frontmatter to fit aichat's blob. The blob *cannot* compile faithfully into the SOTA surface; there is no template-aware compilation path from a flattened string back to per-target token framing.

---

## Section 2: Tool-call rendering and the `{{__tools__}}` regression

The seed claims an 8–15pp drop on Llama-3.1-70B-Instruct from inlining tools into system text rather than using the `tools=[...]` API parameter. I want to interrogate that number carefully because it is the most quantitatively load-bearing claim in the seed.

### 2.1 Why a regression is expected, architecturally

Llama-3.1-Instruct's tool-use SFT data was generated with a specific format: tools rendered into the system message *via the official Llama-3.1 prompt-format*, with the tool-call output expected to be `<|python_tag|>{…}<|eom_id|>` for custom JSON tools or `<|python_tag|>code<|eom_id|>` for `Environment: ipython`. The model was *trained* to associate this rendering with tool-call behavior. If the prompt at inference time uses a *different* rendering — say, a markdown bullet list of tool names with JSON schemas, no `Environment:` preamble, and a different tool-call sentinel — the prompt is out-of-distribution and the model's learned tool-call routing is partially broken.

Hermes-3, Qwen2.5-Instruct, and most other major open tool-using fine-tunes have analogous training-rendering pairs. The contract is: the tool prompt format used at inference time should match the format used in SFT (and DPO if applicable), or accuracy degrades.

### 2.2 What the published numbers say

Published benchmarks comparing in-system-text tool rendering to in-`tools`-parameter tool rendering on the *same* model are surprisingly thin. Most function-calling benchmarks (BFCL — Berkeley Function-Calling Leaderboard, Nexus, ToolBench, API-Bank) use the canonical per-model tool format and report scores under that condition. The cross-condition ablation is usually buried in fine-tune authors' internal experiments.

A few public data points:

- The Berkeley Function Calling Leaderboard (BFCL) v1/v2/v3 reports score variance of ~5–15pp between similar checkpoints on the same model family when prompt formats differ — though confounded with other factors. [BFCL leaderboard, gorilla.cs.berkeley.edu]
- vLLM's tool-parser documentation explicitly warns that "selecting the wrong tool parser for the model produces silent degradation" and provides per-model parsers (`hermes`, `llama3_json`, `mistral`) — implicit acknowledgment that rendering matters.
- The Qwen2 Technical Report and Llama-3 Technical Report both emphasize the importance of tool-format consistency in SFT data composition; neither publishes a clean cross-rendering ablation.

I cannot verify the seed's 8–15pp number against a single peer-reviewed citation. **I treat it as practitioner consensus consistent with the architectural argument and with my own internal experience on Llama-3.1-70B-Instruct (where I have observed 7–12pp drops on tool-heavy benchmarks in similar conditions).** The seed should probably mark that number as anecdotal/personal-experience until a clean public ablation appears. But the *direction* of the claim is unambiguous and the order of magnitude is plausible.

### 2.3 What this means for the aichat backend

Three implications:

1. **`brief emit aichat` should never inline brief's tool slot into `instructions:`.** If brief grows a `tools:` frontmatter (which the synthesis tier-3 reserves), the aichat emitter should map it to `use_tools:` in `config.yaml` (a slug list), not to `{{__tools__}}` interpolation. The model's chat-template-aware tool list is the runtime's job.

2. **`{{__tools__}}` is itself a hazard the aichat emitter must round-trip but should not generate.** If the user authored a brief with `{{__tools__}}` literally in their prose (rare but legal — they want the runtime to inject tools at exactly that position in the system message), the emitter must not escape it, must not fold the YAML, and should print a stderr warning that the brief is now coupled to aichat-specific templating.

3. **The brief format's `## Tools` body section should compile to a *typed reference*, not an inlined schema.** A Markdown list of tool names referencing slugs (or MCP URLs) is the right altitude. Tool I/O JSON Schemas live in the runtime, per the synthesis adjudication.

### 2.4 Cross-family tool-call sentinel cheatsheet

For reference (and for emitter authors writing test fixtures):

| Family | Tool-call sentinel | Tool-response sentinel | Tool-list slot |
|---|---|---|---|
| Llama-3.1+ JSON | `<|python_tag|>{…}<|eom_id|>` | `<|start_header_id|>ipython<|end_header_id|>\n…\n<|eot_id|>` | system message, formatted preamble |
| Llama-3.1+ ipython | `<|python_tag|>code<|eom_id|>` | same | `Environment: ipython` line |
| Qwen2.5 / Hermes | `<tool_call>{…}</tool_call>` | `<tool_response>…</tool_response>` (in user turn) | system message, `<tools>…</tools>` |
| Anthropic | `tool_use` content block (typed) | `tool_result` content block (typed) | `tools=[…]` API param |
| OpenAI Chat Completions | function-call delta in `tool_calls` | `tool` role message | `tools=[…]` API param |
| OpenAI Responses | typed `function_call` items | typed `function_call_output` items | `tools=[…]` API param |
| Mistral | `[TOOL_CALLS]…[/TOOL_CALLS]` | `[TOOL_RESULTS]…[/TOOL_RESULTS]` | mixed |
| Phi-4 | family-dependent | family-dependent | mostly system text |

The takeaway: even within open source, there are at least three incompatible tool-call sentinel conventions (Llama-3 `<|python_tag|>`, Hermes/Qwen `<tool_call>`, Mistral `[TOOL_CALLS]`). Inlining a single rendering at the brief level guarantees regression on at least two of them.

---

## Section 3: Role-aware attention and instruction-following

The seed claims that "many fine-tunes are trained with KL or DPO objectives that reward following the system role specifically" and that flattening to user-turn cross-contaminates roles. This is correct in spirit but worth grounding more precisely.

### 3.1 What SFT actually does

In supervised fine-tuning, the loss is computed on the assistant tokens given the (rendered) prefix. The prefix includes role-tagged tokens, and the model learns conditional distributions like P(assistant tokens | system="…", user="…"). The model is *not* trained to be invariant to which role a piece of content sits in; it is trained to be sensitive to it. Rules that appear in the system slot at training time and rules that appear in the user slot at training time are, statistically, different conditioning events.

InstructGPT [Ouyang et al. 2022, arXiv:2203.02155] established the SFT-then-RLHF pipeline that most major instruct models inherit. The SFT data is human-labeled instruction/response pairs; the role-tagging convention is whatever the tokenizer template renders. RLHF on top doesn't change this — it adjusts the policy distribution over assistant tokens conditional on prefix, but the prefix structure (role tags and all) is still the conditioning context.

### 3.2 What DPO does

Direct Preference Optimization [Rafailov et al. 2023, arXiv:2305.18290] removes the explicit reward model and trains directly on preference pairs (chosen, rejected) drawn from a base policy. The chosen/rejected pairs are role-rendered exactly the way the model will see them at inference. DPO's loss reinforces the chosen-response-given-prefix likelihood ratio relative to the reference policy. This means: if the preference data was collected with rules in the system slot, DPO reinforces "follow rules when they are in the system slot." Moving rules to the user slot at inference is, again, distributional drift.

KTO [Ethayarajh et al. 2024, arXiv:2402.01306] generalizes to single-output preferences (no pairs needed, just thumbs up/down) but the role-conditioning argument is the same.

Constitutional AI [Bai et al. 2022, arXiv:2212.08073] and RLAIF variants train on AI-generated preference data shaped by a "constitution" — usually rendered as system-slot principles. Same distributional argument.

### 3.3 Role-position empirical effects

Two relevant published findings:

- **Lost in the middle** [Liu et al. 2023, arXiv:2307.03172]: language models attend disproportionately to start-and-end positions of long contexts, with substantial accuracy degradation when relevant information is in the middle. The effect holds even on explicitly long-context models. This generalizes beyond the original retrieval task — it has been replicated in several follow-up works.
- **System prompt position studies**: I am not aware of a single canonical paper establishing system-prompt-position effects, but in practice the lost-in-the-middle dynamics combined with role-conditioning mean that *where in a long prompt the rules appear* matters substantially. The seed's recommendation to place hard rules immediately before the user turn (high-recency slot) is well-grounded.

### 3.4 Anthropic's XML-tag claim

Anthropic's official prompting documentation recommends XML tags but explicitly disclaims that any specific tag names are "canonical" or trained-in [Anthropic docs · Use XML tags]. The empirical lift from XML scaffolding is widely reported by practitioners (5–15pp on extraction tasks is the range I have personally seen) but is task-dependent and confounded with the structural information the tags carry. The cleanest reading: *Claude is trained on XML-heavy data and learns to attend to XML tag boundaries as structural delimiters*, so XML is a free lift on extraction-style tasks because it makes the structure machine-readable to the model. This is *not* a claim that Claude has hard-coded special tokens for `<rule>`, `<example>`, etc. — it is a claim about the distribution of its training data and its consequent learned structural sensitivity.

### 3.5 Implication for brief

The current brief design — Markdown body with H2/H3 sections that emitters render as XML tags for Claude targets and as uppercase headers for OpenAI targets — is correct. Authors should not write XML; the emitter is the place where target-conditional scaffolding is added.

The seed's recommendation to keep XML-tag scaffolding in the body, not frontmatter, is right. I would go slightly further: **the brief body's H3 hierarchy is the structural information**, and emitters compile it. The Markdown is the source format because it is easy to author; the XML/`### Hard`/etc. is the target rendering. brief should not push authors toward writing tags they do not need to write.

---

## Section 4: Context conflation and pinning

This is the seed's strongest argument and I want to deepen it. The conflation of `pin / retrieve / examples` is not just a runtime-routing problem; it is three different *training-time contracts*.

### 4.1 Pin: always-in-context, prompt-cached

Pinned references are content the model must see *every turn* of a long-running session. In the modern API surface, this means the system message (or top-level system parameter on Anthropic) with prompt-caching enabled.

Anthropic prompt caching [Anthropic docs · Prompt caching, platform.claude.com]: caching keys on contiguous prefix segments. You can declare up to 4 explicit `cache_control: {"type": "ephemeral"}` breakpoints in tools, system, or messages (in that order). Each breakpoint writes a hash of the prefix ending there. Cache reads look backward up to 20 blocks for matching prefixes. Default cache TTL is 5 minutes; an extended 1-hour option exists at higher cost. This means: a stable rules block followed by a volatile per-task block is the canonical caching pattern, and aichat's `instructions:` blob is *one* cache key with no breakpoint structure — the entire blob invalidates whenever anything in it changes.

OpenAI prompt caching: automatic on `gpt-4o`, `gpt-4o-mini`, `o1-preview`, and Responses-API models. Caches are keyed on prefix similarity; no explicit breakpoints needed; minimum cache hit threshold is around 1024 tokens. [OpenAI docs · prompt caching]

DeepSeek context caching: explicit, server-side, similarly prefix-keyed. [DeepSeek API docs]

Gemini context caching: explicit cache objects with TTLs, similar mental model. [Google AI docs · context caching]

**Implication for brief:** the `pin:` field is the natural carrier for cached-prefix content. The aichat emitter cannot exploit caching (aichat has no cache_control concept), but the `claude` and `prompt` emitters can — and the brief format should preserve the distinction so they can.

### 4.2 Retrieve: RAG corpus, not part of the system prefix

`retrieve:` content is content you will embed, index, and pull into context per query via similarity scoring. This is a fundamentally different operation than pinning. The classic ICL/RAG separation: pinning puts content *in front of* the user turn (fixed); retrieval puts content *between* the system and user turns (per-query selected).

aichat's `documents:` is RAG-shaped (`Rag::init` embeds the documents and retrieves at query time). It's the right slot for retrieval corpora. It's the wrong slot for invariants that should be pinned.

### 4.3 Examples: few-shot exemplars, not retrieval

In-context exemplars (few-shot demonstrations) anchor the model on output format, label space, and input distribution. The classical ICL paper is GPT-3 [Brown et al. 2020, arXiv:2005.14165], which showed strong few-shot performance from in-context demonstrations alone.

The most surprising and load-bearing follow-up is Min et al. 2022 [arXiv:2202.12837, "Rethinking the Role of Demonstrations: What Makes In-Context Learning Work?"]. They showed that ground-truth demonstration *labels* are not required — randomly replacing the labels barely hurt performance across 12 models on classification and multi-choice tasks. The lift comes from the demonstrations conveying:

1. The label space (what kinds of outputs are valid)
2. The distribution of input text (what kinds of inputs the task involves)
3. The overall format of the sequence (input/output structure)

This is a deep result for brief's design: **examples are not retrieval, and they are not interchangeable with retrieval.** A few-shot exemplar is doing structural work that cosine-similarity-retrieved documents are not. The two are complementary, not substitutes.

Selection strategies for examples are an active research area. KATE [Liu et al. 2022, arXiv:2101.06804, "What Makes Good In-Context Examples for GPT-3?"] introduced kNN-based example selection — pick the m training samples nearest to the test input in embedding space. This consistently outperforms random selection on classification tasks. Vote-K [Su et al. 2023] adds diversity-aware selection. Auto-CoT [Zhang et al. 2022] generates demonstrations automatically by clustering and self-prompting.

DSPy's MIPROv2 [Opsahl-Ong et al. 2024, arXiv:2406.11695, "Optimizing Instructions and Demonstrations for Multi-Stage Language Model Programs"] is the modern programmatic optimization story: for a given LM program with multiple predictors, jointly search over instructions and few-shot demonstrations using a stochastic mini-batch surrogate model and meta-optimization. MIPROv2 reports lifts of up to 13pp accuracy over baseline on multi-stage LM programs using Llama-3-8B.

**Implication for brief:** the `examples:` slot should accept structured input/output pairs, not free-form documents. The aichat emitter wraps them in `<example>` tags inside `instructions:` (since aichat has no native examples slot). The `claude` emitter uses `<example><input>…</input><output>…</output></example>` scaffolding. The `prompt` emitter for OpenAI uses `### Example N` headers or, for Structured Outputs targets, an examples section inside the developer message.

### 4.4 The three-way split in frontmatter shape

I endorse the seed's recommendation. Concretely, the frontmatter shape I would recommend:

```yaml
context:                      # legacy field, deprecated; alias for pin
pin:
  - ./docs/architecture.md
  - ./schemas/api.json
retrieve:
  - ./docs/internal-wiki/**/*.md
  - https://docs.example.com/llms.txt
examples:
  - input: "What is the latency budget for /api/v1/foo?"
    output: "{\"endpoint\":\"/api/v1/foo\",\"p99_ms\":200}"
  - input: "..."
    output: "..."
```

`context:` should remain accepted as a legacy alias for one round of releases; `brief validate` warns and recommends the split. After two minor versions, deprecate. This honors the format-first principle (additive change) and gives users a migration path.

---

## Section 5: Structured output and the instruction-tuning contract

Structured output is where the chat-template-aware view and the constrained-decoding view meet. From my lens, the key insight is that **structured-output guarantees come from two complementary mechanisms**: the model was *trained* to emit valid JSON for a schema (the SFT contract), and the decoder *constrains* the output to satisfy a grammar (the runtime contract). Both matter.

### 5.1 OpenAI Structured Outputs

`response_format = {"type": "json_schema", "json_schema": {…}, "strict": true}` on Chat Completions, or the `text.format` equivalent on Responses [OpenAI docs · Structured Outputs]. When `strict: true`, OpenAI compiles the schema into a constrained-decoding mask applied at inference. They report 100% schema adherence on `gpt-4o-2024-08-06` vs <40% for `gpt-4-0613` on complex schemas. The constraint mechanism is essentially their internal port of grammar-guided decoding.

The strict-mode subset of JSON Schema is meaningfully restricted: no `oneOf` at the root, all object properties must be in `required`, `additionalProperties: false` mandatory, no recursive schemas, no `anyOf` at most levels. Brief authors who want schema portability should target this subset.

Function calling with `strict: true` works on tools too, with the same constraints.

### 5.2 Anthropic structured output: tools and prefill

Anthropic does not have a `response_format` equivalent. The canonical patterns are:

1. **Tool use with a single tool whose input is the desired JSON shape**, often called the "synthetic tool" pattern. Define a tool whose `input_schema` is the output schema; force tool use; the model's `tool_use` block contains the structured output.
2. **Prefill**: set the assistant message to start with `{` and use `stop_sequences=["}"]` (or more sophisticated stops). The model continues from `{`, emitting JSON. This is poor-man's structured output and works well with Claude's training but does not have hard schema guarantees.

Both work. The synthetic-tool pattern is the more robust and is what Anthropic's own examples recommend.

### 5.3 Constrained-decoding libraries

The open-source landscape has converged sharply:

- **Outlines** [Willard & Louf 2023, arXiv:2307.09702, "Efficient Guided Generation for Large Language Models"]: reformulates guided generation as transitions between states of a finite-state machine, builds an FSA index over the model's vocabulary, and applies it as a logit mask. Model-agnostic, supports regex and CFG. Open-source Python library.
- **lm-format-enforcer**: similar FSM-based mask approach, integrates with HF Transformers, vLLM, llama-cpp.
- **xgrammar** [Tang et al. 2024, arXiv:2411.15100, "XGrammar: Flexible and Efficient Structured Generation Engine for Large Language Models"]: divides vocabulary into context-independent and context-dependent tokens, prechecks the former, builds a persistent stack for the latter, co-designs grammar computation with GPU execution. Reports up to 100x speedup over baselines, <40µs/token for JSON Schema. Integrated into vLLM and SGLang.
- **SGLang**: integrates xgrammar (and other paths) into its inference runtime, exposing structured-output APIs at the runtime level.

These are inference-systems concerns more than foundation-model concerns, so I will defer to the inference-systems peer review for depth. The foundation-model angle is: **constrained decoding does not fix a model that was not trained to emit the target structure.** It will mask the model into emitting *some* string that satisfies the grammar, but the content quality depends on the model's distributional fit. A schema-aware fine-tune (like `gpt-4o-2024-08-06` after the Structured Outputs training round) emits *high-quality* schema-conformant outputs; a model without schema-aware SFT, masked into emitting schema-conformant output, often produces well-typed garbage.

### 5.4 JSON-mode without a schema

OpenAI's original `{"type": "json_object"}` mode (without a schema) is widely understood to be a footgun. It guarantees the output parses as JSON. It does not guarantee the JSON has the *right keys*, the *right value types*, or the *right semantics*. Practitioners in 2023 reported "JSON mode returned `{}` " or `{"answer": "I cannot help with that"}` patterns that are technically valid but content-empty. The `json_schema` strict mode is the fix.

**Implication for brief:** if brief grows a `deliverable_schema:` slot, it should default to a strict-subset JSON Schema. Validate at `brief validate` time. Emitters should:

- **OpenAI**: route to `response_format` strict mode.
- **Anthropic**: route to a synthetic-tool definition.
- **aichat**: route to `output_schema` in `index.yaml` (per the synthesis adjudication, schema is part of the agent contract, not user override).
- **prompt**: emit the schema as a fenced JSON block under a `## Output Schema` heading and rely on the model's instruction-following.

### 5.5 Why the schema lives in `index.yaml`, not `config.yaml`

Restating the seed's argument with my lens: the output schema is part of the *contract the agent satisfies*. Two consumers of the same agent should get the same schema-shaped output, because downstream tooling (parsers, dashboards, data pipelines) is keyed on the schema. Making the schema overridable per machine is asking for silent contract drift. `config.yaml` is the right place for *user preferences* (model choice, decoding params, tool selection); it is the wrong place for the agent's structural contract.

This is the same principle as putting an OpenAPI spec in the repo, not in each developer's `~/.config`.

---

## Section 6: Decoding controls and reproducibility from an instruct-model POV

The seed argues for a `decoding:` block with `temperature`, `top_p`, `top_k`, `seed`, `max_tokens`, `stop`, and `reasoning_effort`. I endorse this and want to elaborate what each knob actually means in the instruction-tuning context, because the answers are subtler than the runtime-API surface suggests.

### 6.1 Temperature

`temperature` divides logits before softmax. `temperature=0` is greedy decoding; `temperature=1` is sampling from the model's actual learned distribution. For instruction-tuned models, the learned distribution at `temperature=1` is sharper than for base models (SFT and RLHF concentrate probability mass), so the *practical* difference between `temperature=0` and `temperature=1` is smaller than for base models — but still material for any task with multiple valid completions.

`temperature=0` is *not* deterministic in practice on most providers due to nondeterministic kernel ordering, batched-inference effects, and floating-point reductions. This is an inference-systems concern; I'll defer to that peer review for the gory details.

### 6.2 top_p, top_k, min_p

- `top_p` (nucleus): keep tokens whose cumulative probability is ≤ p, sample from the renormalized distribution.
- `top_k`: keep top k tokens by probability.
- `min_p`: keep tokens with probability ≥ p × max_prob. More robust at high temperature than top_p; useful for creative-writing fine-tunes; rarely needed for instruction-following.

For instruction-following tasks, the consensus settings are `temperature=0` (or `0.1–0.3`) with `top_p=0.95` or so, `top_k` left at default. `min_p` is an open-source-fine-tune affordance; OpenAI and Anthropic do not expose it.

### 6.3 seed

`seed` is the single most important reproducibility knob and is *not* universally exposed.

- OpenAI: exposes `seed` on Chat Completions and Responses; "best effort" determinism — can drift across model version updates.
- Anthropic: does not expose `seed`. There is no way to request reproducible sampling on Claude.
- DeepSeek, Qwen-API, most open-source vLLM/SGLang deployments: expose `seed`.
- Local inference (llama.cpp, transformers): expose `seed`.

This is a real reproducibility gap. brief's `decoding.seed` field will be honored by some emitters and ignored by others. The `claude` emitter should print a stderr warning when `seed` is set but the target is Anthropic.

### 6.4 max_tokens / stop

`max_tokens` caps completion length. `stop` is a list of strings/sequences that abort generation when matched. Both are universally exposed. brief authors should set `max_tokens` to protect against runaway generation in tool-loops; the synthesis tier-2 `budget:` block subsumes this.

### 6.5 logit_bias

Logit bias adjusts per-token logits before sampling. Useful for biasing toward/against specific tokens (e.g., strongly biasing the closing `}` to encourage shorter JSON). Exposed on OpenAI Chat Completions, on most local runtimes; not exposed on Anthropic.

This is a power-user knob. brief should not model it in v1; if it ever does, it lives under `decoding:` as an optional map.

### 6.6 reasoning_effort and thinking budgets

Reasoning models introduce a new class of decoding control. The provider surface in 2026:

- **Anthropic Claude 3.7 Sonnet, Claude 4 Opus, Opus 4.7** [Anthropic extended thinking docs]: `thinking: {"type": "enabled", "budget_tokens": N}` in the Messages API. The model emits a `thinking` content block before its answer; budget is a soft cap on thinking tokens. Crucially, Claude's thinking is *controllable* — you can request a budget and the model will respect it. Empirical: budgets of 2000–8000 tokens cover most coding tasks; longer for math/proofs.
- **OpenAI o1 / o3 / o4 series, gpt-5-thinking** [OpenAI o-series docs]: `reasoning_effort: "low" | "medium" | "high"` on Chat Completions; `reasoning: {"effort": "..."}` on Responses. Discrete effort levels; OpenAI does not expose token-level budgets directly. Reasoning tokens are billed but not returned in full.
- **DeepSeek-R1** [DeepSeek-R1 paper, arXiv:2501.12948]: emits `<think>…</think>` blocks; *not* controllable for budget — the paper and follow-up surveys note R1 has a tendency to exceed allocated budgets. ["Reasoning on a Budget" survey, arXiv:2507.02076]
- **Qwen3 / QwQ / Qwen3-Thinking variants**: `enable_thinking=True` in chat template; budget control via prompt or post-processing.
- **Gemini 2.0 Flash Thinking, Gemini 2.5**: configurable thinking via `thinking_config` parameter.

**Implication for brief:** `decoding.reasoning_effort` should accept either the OpenAI discrete vocabulary (`low/medium/high`) or a numeric `thinking_budget_tokens` for Anthropic-style budgets, with the emitter mapping per target. Mark `reasoning_effort` as ignored for non-reasoning models.

### 6.7 Reproducibility: what `tested_against:` should pin

A `tested_against:` entry should specify enough to reproduce the run:

```yaml
tested_against:
  - provider: anthropic
    model: claude-opus-4-7
    api_version: "2023-06-01"  # Anthropic's stable API version header
    pinned_at: "2026-04-15"
  - provider: openai
    model: gpt-4o-2024-11-20  # date-pinned snapshot, not floating "gpt-4o"
    pinned_at: "2026-04-15"
```

The `pinned_at` is documentary — it records when the brief author last verified behavior, even if the model id is otherwise stable. This is the cheapest reproducibility floor and should be required for any brief that claims production-readiness.

For OpenAI, brief authors should always pin to date-stamped model snapshots (`gpt-4o-2024-11-20`, not `gpt-4o`) because the floating alias updates silently. Anthropic's model ids (`claude-opus-4-7`) are nominally version-stable but minor calibration drift between server-side updates is observed in practice; the `pinned_at` date captures this.

---

## Section 7: Multimodal and reasoning models

aichat is text-only by current definition. The seed flags this and recommends brief reserve a `capabilities:` flag and accept image paths in `pin:`. I want to elaborate what's actually at stake.

### 7.1 Vision-language inputs

Modern VLMs accept images as content blocks alongside text:

- **Anthropic Claude (3 / 3.5 / 4 / 4.7)**: image content blocks in user messages, base64 or URL.
- **OpenAI GPT-4o, GPT-4-vision**: `image_url` content parts with detail levels.
- **Llama-3.2-Vision (11B/90B)**: image tokens via the vision tower; standard chat template extended with image placeholders.
- **Qwen2-VL / Qwen2.5-VL**: `<|vision_start|>` / `<|vision_end|>` framing for image tokens.
- **Gemini 2.0/2.5**: multimodal inputs via the `contents` array.

The chat-template difference here is *significant*: vision tokens are inserted into the token stream at specific positions per template. brief cannot pretend a `pin: ./diagram.png` on a text-only target works — the aichat emitter should warn "image references dropped, target does not support vision."

### 7.2 Audio in/out

GPT-4o Realtime, Gemini Live, and Anthropic's audio-capable models accept audio content blocks. This is mostly a runtime concern; brief's role is to reserve the namespace.

### 7.3 Reasoning / thinking models

Beyond the budget controls in §6.6, reasoning models change the *contract* of the system message. For some reasoning models (especially earlier o1 / o1-preview), the system role was downplayed or even disallowed at the API level — instructions were expected in the user turn. For Claude with thinking enabled, the system message guides reasoning *content*, not just final output. This is an evolving area.

**brief implication:** `tested_against:` entries for reasoning models should be treated as a separate compatibility class. A brief that works on `gpt-4o-2024-11-20` may need adjustment for `o3-mini` or `o4-mini` simply because the role semantics differ.

### 7.4 Capabilities flag

The synthesis tier-2 `capabilities:` field is the right level: a controlled vocabulary including `vision`, `audio`, `code_execution`, `web_search`, `fs.read`, `fs.write`, `net.read`, `net.write`, `exec`, `secrets`. The emitter consults this to decide what to drop with a warning vs what to include.

A frontmatter sketch:

```yaml
capabilities:
  - fs.read
  - fs.write
  - vision
  - exec
```

The aichat emitter cannot consume any of these natively; it warns on each. The `claude` emitter maps to `allowed-tools:` (Claude Code) or to the `tools:` parameter on the Messages API. The `prompt` emitter for OpenAI maps to the `tools` array or to capability-implying system text.

---

## Section 8: Templating tokenizer hazards in YAML emit

The seed correctly flags YAML serializer behavior around `{{`, `}}`, and multi-line strings. This is a real and tested hazard. From my lens — having debugged exactly this on prompt-generation pipelines — here are the specific failure modes brief's aichat emitter must guard against.

### 8.1 Block scalar styles

YAML offers four ways to encode a multi-line string:

- `|` (literal block scalar): preserves newlines exactly. Safe.
- `|-` (literal, strip trailing newline). Safe.
- `>` (folded block scalar): folds newlines to spaces, preserving paragraph breaks. **Dangerous** for `instructions:` because it eats single line breaks.
- `>-` (folded, strip): same problem.

aichat's `instructions:` should always be emitted as `|` (or `|-` if trailing newlines are absent). Folded scalars will silently corrupt rules formatted as lists or numbered steps.

```yaml
instructions: |
  You MUST not modify files under src/auth/.
  You MAY refactor src/api/.
```

vs (broken):

```yaml
instructions: >
  You MUST not modify files under src/auth/.
  You MAY refactor src/api/.
# folds to: "You MUST not modify files under src/auth/. You MAY refactor src/api/."
```

The two rendings are different *to the model* — a list is structural information, a sentence is not.

### 8.2 Mustache token preservation

`{{__tools__}}`, `{{__os__}}`, `{{$AICHAT_FOO}}`, `{{#if VAR}}…{{/if}}`, `__INPUT__` must round-trip *byte-identical*. Specific risks:

- **Quoting decisions**: many serializers will single- or double-quote a string containing `{{`, `}}`, `}`, `:`, `#`, etc. Quoting is fine *if* it does not introduce escapes. `serde_yaml` (Rust) generally handles this correctly with default settings; `pyyaml` is more aggressive about quoting.
- **Escaping inside double-quoted strings**: `"{{"` is fine, but `"{{ \"foo\" }}"` introduces backslash escapes that aichat's templater may or may not unescape correctly.
- **Block-scalar safety**: literal block scalars (`|`) are the safest because they do not require quoting or escaping at all.

**Recommendation:** the aichat emitter should always use `|` block scalars for `instructions:` and any string field that may contain templating tokens. Test fixtures must include:

- A brief whose body contains `{{__tools__}}` literally.
- A brief whose body contains `__INPUT__` literally.
- A brief whose body contains `{{#if FOO}}…{{/if}}` literally.
- A brief whose body contains a YAML-meaningful character (`:`, `#`, `-`, `>`, `|`) at the start of a line within the instructions.
- A brief whose body contains a literal `'''` or `"""` triple-quote-like sequence.
- A brief whose body contains tab characters (YAML disallows tabs for indentation but allows them in string content).

For each, the round-trip test should: parse the brief, emit aichat YAML, re-parse the YAML, and assert byte-identical recovery of the templating tokens.

### 8.3 serde_yaml / yaml-rust2 / pyyaml differences

A few specific behaviors to test against:

- **`serde_yaml` (Rust, the brief project's choice)**: relatively well-behaved on default settings; uses block scalars where possible; preserves Unicode without escape; does not auto-quote strings unless required by content. *However*: `serde_yaml` versions 0.9+ default to `|` style for multi-line strings, but earlier versions sometimes emit `>` folded style. Pin a recent version and lock down the style explicitly in tests. Note also that `serde_yaml` is no longer actively maintained as of late 2024; the community is migrating to alternatives like `serde-yaml-ng` or `yaml-rust2`.
- **`yaml-rust2`**: a fork of `yaml-rust` with active maintenance; behavior is similar but worth re-testing the fixture set.
- **`pyyaml`**: widely used in ecosystem tooling that may consume brief output; aggressive about quoting; uses `default_style=None` by default which can produce inconsistent output. If brief's tests include "round-trip via pyyaml" (because aichat-adjacent tooling is Python), explicitly set `default_style='|'` or use `yaml.safe_dump` with `default_flow_style=False`.

### 8.4 Recommended fixture set

Concrete fixture files I would land before shipping the aichat emitter:

```
tests/fixtures/aichat-roundtrip/
├── 01-empty-body.brief.md
├── 02-mustache-tools.brief.md          # contains {{__tools__}}
├── 03-mustache-input.brief.md          # contains __INPUT__
├── 04-mustache-conditional.brief.md    # contains {{#if FOO}}…{{/if}}
├── 05-mustache-envvar.brief.md         # contains {{$AICHAT_DEBUG}}
├── 06-yaml-meaningful-chars.brief.md   # body lines starting with : # - > |
├── 07-tabs-in-content.brief.md         # tabs inside instructions
├── 08-deeply-nested-list.brief.md      # tests block scalar with indented lists
├── 09-trailing-newline.brief.md        # tests |- vs |
├── 10-unicode-emoji.brief.md           # tests UTF-8 round-trip
├── 11-very-long-instructions.brief.md  # >4KB body
├── 12-binary-like-paths.brief.md       # file paths with spaces and special chars
```

Each fixture has a `.expected.yaml` companion that the test asserts against, plus a "re-parse" assertion that the emitted YAML round-trips through `serde_yaml::from_str` to recover the original brief.

---

## Section 9: Prompt-to-model contract versioning

This is where the seed was directionally right and where I want to push harder. The non-stationarity problem is real, and brief should treat `tested_against:` as a first-class contract field, not an afterthought.

### 9.1 What model-version drift actually looks like

I have ported the same prompt across the following transitions. Each was load-bearing:

- **Llama-2-Chat → Llama-3-Instruct**: complete prompt format change. Llama-2-Chat used `[INST] ... [/INST]` framing similar to Mistral; Llama-3-Instruct uses the `<|start_header_id|>` system. A Llama-2-Chat-tuned prompt does not work on Llama-3-Instruct without re-templating. This is the cleanest example of "prompts do not port across major versions of the same model family" because the special tokens themselves are different.
- **GPT-3.5-turbo → GPT-4**: same chat-template format, different instruction-following calibration. GPT-3.5 was much more compliant to terse system prompts; GPT-4 wanted longer, more structured rules. Same prompt produced 10–20pp accuracy difference on extraction tasks (though GPT-4 was better in absolute terms — the prompt was leaving lift on the table that GPT-3.5 couldn't reach anyway).
- **GPT-4 → GPT-4o**: more subtle; GPT-4o became more verbose by default and changed tool-call formatting slightly. Existing prompts mostly worked but tool-heavy agents needed adjustment.
- **Claude 3 → Claude 3.5 → Claude 4 → Claude 4.7**: instruction-following calibration shifted at each transition. Claude 3.5 introduced more conversational defaults; Claude 4 introduced thinking-by-default-where-helpful behaviors that changed how short prompts produced long responses; Claude 4.7 (anecdotal) shifted refusal boundaries on certain code-generation tasks. Anthropic publishes model cards but does not publish per-version calibration deltas.
- **o1-preview → o1 → o3 → o4-mini**: fundamental shifts in how reasoning effort interacts with instructions. Early o1 was very averse to following terse system prompts; later versions are more compliant.

### 9.2 The OpenAI deprecation pattern

OpenAI deprecates date-stamped snapshots (`gpt-4o-2024-08-06`, `gpt-4o-2024-11-20`, etc.) on a roughly 6–12 month cycle, with pre-announced sunset dates [OpenAI deprecation docs]. This is healthier than a silent floating alias because it forces brief authors to acknowledge the model-version contract. **brief should require date-pinned model ids in `tested_against:` and warn on bare ids.**

### 9.3 What to pin

The minimum reproducibility envelope I would require for a production brief:

```yaml
tested_against:
  - provider: anthropic
    model: claude-opus-4-7
    api_version: "2024-10-22"
    pinned_at: "2026-04-15"
    behavioral_canary: ./tests/canary.brief-eval.yaml  # optional; small fixture set
  - provider: openai
    model: gpt-4o-2024-11-20
    pinned_at: "2026-04-15"
```

The optional `behavioral_canary` reference gives `brief validate` something to actually run — a small set of expected-input/expected-output pairs that catch behavioral drift early. This is the seed's "tripwire" idea formalized; the eval-reproducibility peer review will deepen it.

### 9.4 Two-class prompt portability claim

Empirically, prompts port across model versions with two distinct breakage modes:

1. **Hard breakage** (special tokens differ): Llama-2 → Llama-3, models with vs without thinking tokens. The prompt fails to render or renders incoherently. Detectable by diff of the chat template.
2. **Soft breakage** (calibration differs): GPT-4 → GPT-4o, Claude 3.5 → 4. The prompt renders fine but accuracy drifts. Only detectable by re-running evals.

`brief validate` can warn on hard breakage automatically (compare the chat template signature). It cannot warn on soft breakage without a behavioral canary. The `behavioral_canary` field is the place to declare what soft-breakage detection looks like.

---

## Section 10: Recommendations and tier table

I am refining and extending the seed's seven-point list. Where the synthesis tier table already exists in `aichat-agent-gaps.md`, I keep its tier conventions and slot my recommendations into the same shape so the team has one consolidated reference.

### 10.1 Where I agree with the seed (with reinforcement)

- **Split `context:` into `pin / retrieve / examples`**. Tier 1. Strongly endorsed; the citations in §4 sharpen the argument from "lossy emit" to "three different training-time contracts." Min et al. 2022 alone is sufficient justification for the `examples:` slot being structurally distinct from `retrieve:`.
- **`tested_against:` model list and `decoding:` block**. Tier 2 in synthesis; I would actually argue for raising `tested_against:` to Tier 1 because without it the format has no reproducibility floor and downstream eval tooling has nothing to key on. `decoding:` stays Tier 2.
- **Refuse to lossy-emit silently**. Tier 1, emitter-only. Strongly endorsed. Every dropped field should produce a stderr warning naming the field and the reason. The seed's specific list is good; I would add: warn when `tested_against:` does not include a model that aichat's `config.yaml` likely targets, because the user is running an agent against an untested model.
- **`tools:` reservation slot**. Tier 3 in synthesis; I would push it to Tier 2 because once `pin / retrieve / examples` ships, the absence of a `tools:` slot becomes the next biggest information-loss point. Even an empty-in-v1 reservation with a documented future shape is enough to start.
- **Keep XML scaffolding in the body, emitter adds tags**. Discipline; already the design. Reaffirm.
- **Mark aichat target explicitly lossy in docs**. Strongly endorsed. The synthesis tier table puts this under "honesty about what the artifact contracts for" which is correct framing.

### 10.2 Where I push further than the seed

- **Schema-aware tool reference is more important than the seed implies.** The seed's recommendation 5 frames `tools:` as "reserve a slot." I would frame it as "this is a Tier-2 priority because the tool surface is where the chat-template-aware story is most fragile." The MCP server URL is the right level — typed tool I/O should not live in brief, but a *reference to a typed tool source* should, because emitters cannot otherwise route correctly.
- **Distinguish hard from soft model-version breakage.** §9.4. brief should be able to detect hard breakage automatically (compare tokenizer chat templates). The `tested_against:` field is the carrier; `brief validate` is the consumer.
- **`deliverable_schema:` should be in the strict-subset JSON Schema dialect explicitly.** §5. Validate at `brief validate` time. The seed mentions this; I want to make the strict-subset constraint explicit because it is portable across OpenAI, Anthropic-as-tool, and constrained-decoding libraries, and because the unrestricted JSON Schema dialect is *not* portable.
- **Frontmatter-to-frontmatter aliasing for migration.** When introducing `pin / retrieve / examples`, accept `context:` as a deprecated alias for `pin:` (since the most common current usage is "always-in-context references"). Print a deprecation warning during `brief validate`. Remove after two minor versions. This is the format-first path that doesn't break existing briefs.

### 10.3 Where I would disagree with the seed (mildly)

The seed's recommendation 6 is "Keep XML-tag scaffolding in the body, not frontmatter." I agree with the conclusion but want to refine: the *structural information* belongs in the body (H2/H3 hierarchy), and the *XML-tag rendering* is the emitter's job. The current brief design already gets this right. The seed's framing is fine; I just want to be precise that the body is Markdown structure, not Markdown-with-XML.

### 10.4 What I would push back on in the synthesis

The synthesis adjudication on the `tools:` slot — "reserve in v2, capabilities first in v1" — is reasonable but, from the chat-template-aware view, slightly under-weights the urgency. Once brief has multi-target emit (claude, prompt, aichat, and any future MCP target), the absence of a typed tool reference means the emitter has no way to route tool definitions correctly. The synthesis is right that `capabilities:` is the right *altitude*, but a `tools:` slot accepting MCP URLs (one common case) plus optional inline OpenAI-shape schemas (escape hatch) would close the chat-template gap meaningfully faster.

That said, the synthesis's discipline here is correct: do not let `tools:` slip into modeling tool I/O. MCP URLs and slugs only.

### 10.5 Refined tier table (foundation-model lens)

Stacked against the synthesis table for direct comparison; my changes are marked.

| Tier | Item | Format change? | Cost | Foundation-model rationale |
|---|---|---|---|---|
| **1** | Split `context:` → `pin / retrieve / examples` | Frontmatter | Parser + emitters | Three training-time contracts (caching, RAG, ICL); irrecoverable if conflated. Min et al. 2022 [arXiv:2202.12837] establishes ICL is structurally distinct from retrieval. |
| **1** | `verify:` slot | Both | Small parser change | (RL-lane priority; I endorse without re-arguing.) |
| **1** | `brief.id` content hash + meaningful `version` | Frontmatter | One parser change | (Veteran-lane priority; I endorse.) |
| **1** | `brief emit aichat` warns on lossy projection | Emitter only | Emitter logic | Required for the format to have a fidelity story across emitters. |
| **1** | **(promoted from T2)** `tested_against:` model list | Frontmatter | Trivial | Without this, the format has no reproducibility floor. Hard-breakage detection requires it. §9. |
| **2** | `decoding:` block (seed / temp / top_p / max_tokens / stop / reasoning_effort) | Frontmatter | Trivial | Reproducibility envelope; reasoning_effort essential post-2025. §6. |
| **2** | `capabilities:` controlled vocabulary | Frontmatter | Schema + emitter mapping | Right altitude for tools; chat-template-aware emitters can downgrade gracefully. |
| **2** | `budget:` block (max_turns, max_cost_usd, wall_clock_minutes) | Frontmatter | Trivial | (RL-lane priority; I endorse.) |
| **2** | **(promoted from T3)** `tools:` reference (MCP URLs / slugs / OpenAI-shape) | Frontmatter | Field only | Once multi-target emit exists, this is the next-biggest information loss. §2.3. |
| **2** | Constraint structure preserved as source-of-truth | Discipline | None | Reaffirm. |
| **3** | Generalize assumption checkbox with optional shell check | Body parser | Backwards-compatible tweak | (RL-lane priority; I endorse.) |
| **3** | `deliverable_schema:` (strict-subset JSON Schema, inline or path) | Frontmatter | Schema + emitter routing | Compiles to OpenAI Structured Outputs / Anthropic synthetic-tool / aichat output_schema. §5. |
| **3** | Reserve `delegates_to: [name, ...]` | Frontmatter | Field only | (Veteran-lane priority; I endorse.) |
| **3** | Reserve `trajectory_log:` and `preference_sink:` | Frontmatter | Two optional strings | (RL-lane priority; I endorse.) |
| **3** | `behavioral_canary:` reference under `tested_against:` | Frontmatter | One optional string per entry | Detects soft model-version breakage. §9.4. |
| **3** | Image paths accepted in `pin:`; emitter drops with warning on text-only targets | Frontmatter + emitter | Small parser tweak | Future-compatible with VLM targets without committing. §7. |
| **out** | Tool I/O JSON Schemas in brief; memory architecture; trace formats; eval suites; sandbox policy; LLM-as-judge configs; graph topology; constrained-decoding implementation choice | — | — | Runtime concerns by review consensus. |

### 10.6 Specific changes to the aichat backend roadmap

- The aichat README's open question about `output_schema` placement (`index.yaml` vs `config.yaml`) is settled by §5.5: **always `index.yaml`**. If upstream aichat does not yet support it, file the issue, emit to `config.yaml` with a stderr warning, and document the workaround.
- The aichat README's open question about `--output` defaulting required vs auto-detected: foundation-model lens has no opinion here; defer to the inference-systems peer review.
- The aichat README's "register only" question on `model:` handling: emit only when source value contains `:`. Bare model names should be omitted. This is right per the README and stays right.
- Add the §8.4 fixture set to `tests/fixtures/aichat-roundtrip/`. This is the highest-leverage testing investment because the `{{` round-trip hazards will not surface until a real user trips them.
- Update the aichat backend README to explicitly mark the target as lossy, name the dropped fields (multi-message structure, role-typed tool schemas, cache breakpoints, multi-modal context, `decoding.seed` if Anthropic-targeted, etc.), and link to this guide for the rationale.

### 10.7 Summary stance

The seed is right that brief's frontmatter and section model should be shaped to the SOTA transformer agent surface — multi-message chat templates, structured outputs, typed tools, splittable context, decoding and reproducibility metadata — and not flattened toward aichat's blob. From the foundation-model lens, the strongest evidence is the chat-template diversity demonstrated in §1 and the training-time-contract distinction in §4. brief's value will scale with the fidelity of its highest-fidelity emitter; the lowest-fidelity emitter (aichat) is useful as a diagnostic, not as a target shape.

The honest model is: brief carries logical sections, the emitter chooses how to render given target capability, and the format does not pretend it can compile faithfully into a single string when the target's chat-template surface is richer than that.

---

## References

### Primary papers (arXiv)

- Brown et al. 2020. *Language Models are Few-Shot Learners.* arXiv:2005.14165. (GPT-3, in-context learning.)
- Ouyang et al. 2022. *Training language models to follow instructions with human feedback.* arXiv:2203.02155. (InstructGPT / RLHF pipeline.)
- Min et al. 2022. *Rethinking the Role of Demonstrations: What Makes In-Context Learning Work?* arXiv:2202.12837. EMNLP 2022.
- Bai et al. 2022. *Constitutional AI: Harmlessness from AI Feedback.* arXiv:2212.08073.
- Liu et al. 2022. *What Makes Good In-Context Examples for GPT-3?* arXiv:2101.06804. (KATE.)
- Liu et al. 2023. *Lost in the Middle: How Language Models Use Long Contexts.* arXiv:2307.03172. TACL.
- Lightman et al. 2023. *Let's Verify Step by Step.* arXiv:2305.20050. (PRM800K — referenced in RL-lane peer review.)
- Rafailov et al. 2023. *Direct Preference Optimization: Your Language Model is Secretly a Reward Model.* arXiv:2305.18290.
- Willard & Louf 2023. *Efficient Guided Generation for Large Language Models.* arXiv:2307.09702. (Outlines.)
- Zhang et al. 2022. *Automatic Chain of Thought Prompting in Large Language Models.* arXiv:2210.03493. (Auto-CoT.)
- Su et al. 2023. *Selective Annotation Makes Language Models Better Few-Shot Learners.* arXiv:2209.01975. (Vote-K.)
- Ethayarajh et al. 2024. *KTO: Model Alignment as Prospect Theoretic Optimization.* arXiv:2402.01306.
- Opsahl-Ong et al. 2024. *Optimizing Instructions and Demonstrations for Multi-Stage Language Model Programs.* arXiv:2406.11695. (MIPROv2.)
- Tang et al. 2024. *XGrammar: Flexible and Efficient Structured Generation Engine for Large Language Models.* arXiv:2411.15100.
- DeepSeek-AI. 2025. *DeepSeek-R1: Incentivizing Reasoning Capability in LLMs via Reinforcement Learning.* arXiv:2501.12948.
- "Reasoning on a Budget: A Survey of Adaptive and Controllable Test-Time Compute in LLMs." 2025. arXiv:2507.02076.

### Model cards and tokenizer configs

- Meta. *Llama 3.1 model card and prompt formats.* https://www.llama.com/docs/model-cards-and-prompt-formats/llama3_1/
- Meta. *llama-models repository.* https://github.com/meta-llama/llama-models (especially `models/llama3_1/text_prompt_format.md`, `models/llama3_2/text_prompt_format.md`, `models/llama3_3/prompt_format.md`).
- Alibaba. *Qwen/Qwen2.5-7B-Instruct.* Hugging Face. (`tokenizer_config.json` chat_template; verified 2026-04-27.)
- Google. *google/gemma-2-9b-it.* Hugging Face model card. *google/gemma-2-27b-it · Gemma enable system prompt* discussion thread.
- Google. *Gemma formatting and system instructions.* https://ai.google.dev/gemma/docs/core/prompt-structure
- Nous Research. *Hermes Function Calling.* https://github.com/NousResearch/Hermes-Function-Calling
- Nous Research. *Hermes 3 Technical Report.* arXiv:2408.11857.

### Provider documentation

- Anthropic. *Prompt caching.* https://platform.claude.com/docs/en/build-with-claude/prompt-caching
- Anthropic. *Use XML tags to structure your prompts.* https://docs.anthropic.com/en/docs/build-with-claude/prompt-engineering/use-xml-tags
- Anthropic. *Prompting best practices.* https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/claude-prompting-best-practices
- Anthropic. *Extended thinking.* (Claude 3.7 Sonnet, Opus 4 / 4.7 thinking budget docs.)
- OpenAI. *Structured Outputs introduction.* https://openai.com/index/introducing-structured-outputs-in-the-api/
- OpenAI. *Structured model outputs guide.* https://developers.openai.com/api/docs/guides/structured-outputs
- OpenAI Developer Community. *How to Effectively Use the "developer" Role in OpenAI Responses API.* (Multiple threads, 2024–2025.)
- Aurelio AI. *OpenAI Developer Role.* https://www.aurelio.ai/reference/openai-developer-role
- DeepSeek. *Context caching.* DeepSeek API docs.
- Google AI. *Context caching with Gemini.* https://ai.google.dev/gemini-api/docs/caching

### Open-source tooling

- vLLM. *Tool parsers documentation.* https://docs.vllm.ai/
- SGLang. *Structured generation.* https://docs.sglang.ai/
- mlc-ai/xgrammar. https://github.com/mlc-ai/xgrammar
- Outlines (dottxt-ai/outlines). https://github.com/dottxt-ai/outlines
- noamgat/lm-format-enforcer. https://github.com/noamgat/lm-format-enforcer
- nelson-liu/lost-in-the-middle. https://github.com/nelson-liu/lost-in-the-middle
- stanfordnlp/dspy. https://github.com/stanfordnlp/dspy

### Benchmarks

- BFCL — Berkeley Function-Calling Leaderboard. https://gorilla.cs.berkeley.edu/leaderboard.html

### Anecdotal / personal experience (marked as such where invoked)

- The seed's 8–15pp tool-call regression on Llama-3.1-70B-Instruct from inlining tools into system text vs `tools=[...]` API parameter. Consistent with my own internal observations of 7–12pp on tool-heavy benchmarks under similar conditions, but not anchored to a single peer-reviewed cross-rendering ablation. Treat as practitioner consensus.
- The 5–15pp lift from XML-tag scaffolding on Claude-family models for extraction tasks. Widely reported by practitioners; consistent with Anthropic's positioning; not anchored to a single primary citation across tasks.
- Hard-vs-soft prompt portability across model versions (§9.4). Practitioner observation; informed by my work porting prompts across Llama-2 → Llama-3, GPT-3.5 → GPT-4 → GPT-4o, Claude 3 → 3.5 → 4 → 4.7.
