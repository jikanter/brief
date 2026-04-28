---
author: LLM Inference & Structured Decoding Systems Engineer (peer review)
role: Production LLM Systems Engineer (constrained decoding, prompt caching, decoding params, batching)
date: 2026-04-27
purpose: Cited, complete best-practices guide expanding on analysis-llm-supervised.md from the inference-systems lens
companion_to: [analysis-llm-supervised.md, aichat-agent-gaps.md, analysis-ml-veteran.md, analysis-rl-online-rewards.md]
---

# Best-Practices Guide: Inference-Systems Lens on `brief` and the aichat Backend

**Author:** LLM Inference & Structured Decoding Systems Engineer (peer reviewer 3 of 3)
**Date:** 2026-04-27
**Companion to:** `analysis-llm-supervised.md` (seed), `aichat-agent-gaps.md`, `analysis-ml-veteran.md`, `analysis-rl-online-rewards.md`

---

## TL;DR

`brief` is positioned correctly as a format-first artifact with cooperative emit targets, but the seed analysis of the aichat backend understates how much of the artifact's downstream value lives in the *inference-systems* layer the seed treats as table stakes. From an inference engineer's vantage, three classes of information the format must carry — **structured-output schemas**, **prompt-cache breakpoints**, and **decoding/reasoning controls** — are not optional metadata. They are load-bearing for cost, reproducibility, and correctness, and aichat's current schema cannot represent any of them faithfully.

The seed gets the seven big calls right: split `context:`, add a structured-output schema slot, add `tested_against:` and `decoding:`, refuse to lossy-emit silently, reserve `tools:`, keep XML-tag scaffolding in the body, mark aichat explicitly lossy. I endorse all seven and push further on three. **First**, the `decoding:` block needs to be richer than the consensus subset proposed — `reasoning_effort` / `thinking.budget_tokens` is now mainstream across Anthropic, OpenAI, DeepSeek, Qwen, and Gemini and belongs in the spec, not as an afterthought. **Second**, `brief` should add a `cache:` primitive (or equivalent breakpoint syntax) before any aichat-class blob target ships, because every emit target except `prompt` and `claude` will lose breakpoint information silently and the cost differential is 10× per cached token. **Third**, the aichat blob is not just lossy — it is *anti-shaped* for production-batched inference, because RadixAttention and its peers cannot share prefixes across runs that interleave variable content into the system prefix.

The single recommendation I'd elevate above the seed's list: **role-tagged messages, not blob, are the SOTA contract**. Until `brief` models multi-message structure, every emit target downstream of vLLM, SGLang, TGI, llama.cpp, or any provider with prefix caching is paying both a correctness tax (chat-template misalignment) and a cost tax (prefix-cache miss). The aichat target compounds both. The honest framing in the README — "lossy projection" — should be operationalized as stderr warnings that name dropped capabilities by reference to inference-systems primitives, not as a documentation footnote.

## Synthesis

Three things from this lens I want the brief team to internalize.

**The format must own three machine contracts that prose cannot carry.** A briefing has two audiences: the human author and the inference engine. Markdown body is for the human; frontmatter is for the engine. Three pieces of information have no faithful prose form because they are consumed by code, not models: (1) the JSON Schema that bounds the output (consumed by Outlines/xgrammar/Structured Outputs at the logits layer); (2) the cache breakpoint indices that determine whether the system prefix is reused across requests (consumed by Anthropic's `cache_control`, vLLM's RadixAttention, SGLang); (3) the decoding parameters that pin reproducibility (consumed by every provider's HTTP API). The seed correctly identifies (1) and (3); it understates (2). All three belong in frontmatter. None belong in the body.

**aichat's blob is not a representational issue — it's an inference-economics issue.** The seed correctly identifies that single-blob `instructions:` loses role-aware attention and tool-message separation. What it doesn't fully unpack: a single blob is also a single cache key, a single prefix-tree node in RadixAttention, and a single point of variability for token-level masking. When aichat interpolates `{{__tools__}}` or any per-request variable into the system string, it invalidates the entire cached prefix on every request — which on Anthropic 1h-cache pricing means paying 2× input rate instead of 0.1× input rate. This is a 20× cost regression that will not show up in any test fixture; it will show up in a billing surprise.

**Reproducibility is now a tooling story, not a research nicety.** OpenTelemetry's GenAI semantic conventions, OpenInference, Phoenix, LangSmith, and Anthropic's own console traces have all settled on the same trace envelope: `model_id + system_fingerprint + decoding_params + seed + content_hash + token_counts`. `brief`'s `tested_against:` and `decoding:` blocks must carry exactly this envelope, in field names that match the conventions, so that downstream tools can ingest a `brief.id` without a translation layer. Picking field names that match `gen_ai.request.*` from OpenTelemetry semantic conventions costs nothing now and pays a forever-dividend in trace integration.

The rest of this document expands these into the prescriptive guide.

---

## Section 1 — Structured Decoding: State of the Art

The constrained-decoding literature has consolidated around five technical approaches, each with different overhead/expressiveness tradeoffs. `brief` does not need to *implement* any of these — it needs to emit into formats that select the right one — but the format must carry enough information for downstream tooling to make that choice.

### 1.1 Regex / DFA-based (Outlines)

Willard & Louf 2023 [^willard2023] established the canonical formulation: regular expressions and JSON Schema (the subset expressible as regex) compile to a finite-state machine, and the machine's transitions are precomputed against the model's vocabulary into an *index*. At each generation step, the index returns the set of tokens that keep the FSM in a valid state, and a logit mask zeroes out everything else. This is implemented in the Outlines library [^outlines]. Overhead: near zero per token after FSM compilation, because the index is a precomputed lookup. Compilation cost: linear in (regex length × vocab size), one-time per schema.

Strengths: rigorous correctness guarantee for the regex/FSM-expressible subset; very low per-token overhead; integrates with most inference engines via logit-processor hooks.

Weaknesses: pure regex/DFA cannot express recursive JSON (recursion requires CFG/pushdown); vocabulary indexing is per-tokenizer (so changing tokenizers means recompilation); JSON Schema features like `oneOf` discriminated unions and `$ref` cycles fall outside the strict regex subset and must be encoded as approximations.

### 1.2 Context-free grammar / pushdown (Guidance, lm-format-enforcer, llguidance)

Where regex/DFA breaks (recursive structures, balanced delimiters, true CFGs), pushdown automata are required. Guidance [^guidance] and lm-format-enforcer [^lmfe] both compile schemas to grammars and walk the grammar with a stack at decode time. The cost is per-token grammar interpretation, which historically has been the bottleneck — vLLM's Outlines CFG mode "runs significantly slower" than its JSON-mode regex equivalent and "can occasionally crash the engine" [^vllm-structured].

Strengths: full CFG expressiveness covers recursive JSON, unlike pure regex.

Weaknesses: per-token interpretation overhead can dominate decode latency at scale.

### 1.3 Compressed FSM / hybrid pushdown (xgrammar)

Dong, Ruan, et al. 2024 [^xgrammar] introduced XGrammar, which is the current state of the art and the default backend for vLLM and SGLang structured outputs. XGrammar's central insight is to partition the vocabulary into *context-independent tokens* (whose validity does not depend on the current grammar state — most of the vocab) and *context-dependent tokens* (a small minority that actually require runtime grammar interpretation). Context-independent tokens are precomputed; only the context-dependent set is checked per step. The paper reports up to 100× speedup over previous solutions and "near-zero overhead structured generation in end-to-end LLM serving." A v2 [^xgrammar2] extended this to dynamic agentic settings.

This is the implementation `brief` authors should assume sits behind `vllm`, `sglang`, and `trtllm` flags. Its existence is the reason structured output is no longer a performance excuse.

### 1.4 Token-by-token logit masking (the universal substrate)

All of the above reduce to the same runtime primitive: at each decode step, compute the set of valid next-tokens, apply `-inf` to invalid logits, sample. This is the layer where every provider — OpenAI's Structured Outputs, Anthropic's tool_use shape, Gemini's `response_schema` — implements its enforcement. The interface is `(grammar_state, logits) -> masked_logits`. The variations are in how grammar_state is represented and how the mask is computed, not in the masking step itself.

### 1.5 Speculative + grammar-guided decoding

Speculative decoding (Leviathan et al. 2023 [^leviathan2023]) accelerates inference by drafting tokens with a small model and verifying with the large model. Combining speculation with grammar guidance is non-trivial — drafted tokens may violate the grammar — but recent work (e.g., the SqueezeBits guided-decoding benchmark on vLLM and SGLang [^squeezebits-benchmark]) shows it is now production-viable for both engines. `brief` does not interact with this directly, but it should not generate output that *forces* a grammar so restrictive that speculation is impossible (e.g., a JSON Schema with extreme branching factor at every position).

### 1.6 Comparison and recommendation

| Approach | Overhead | Correctness | JSON Schema coverage | Engines |
|---|---|---|---|---|
| Outlines (regex/DFA) | ~0 / token | Strict for FSM-expressible | Non-recursive subset | vLLM, MLC, llama.cpp |
| Guidance / lm-format-enforcer (CFG) | Higher | Strict | Full | vLLM, TGI |
| **XGrammar (compressed FSM)** | **~0 / token** | **Strict** | **Full incl. recursive** | **vLLM, SGLang (default)** |
| OpenAI Structured Outputs (proprietary) | Vendor-internal | Strict | "strict subset" of JSON Schema | OpenAI Chat / Responses |
| Anthropic tool_use | Vendor-internal | Empirical (fine-tuned) | Full schema, no formal guarantee | Anthropic Messages |

For brief, the implication is: emit a JSON Schema and let the runtime pick its enforcement engine. **Do not emit a regex** (loses recursive coverage), **do not emit a Pydantic model** (Python-specific), **do not emit a TypeScript type** (no JSON Schema fidelity). The lingua franca is JSON Schema (Draft 2020-12 or the OpenAI strict subset).

---

## Section 2 — JSON Schema as Machine Contract

The seed correctly identifies that aichat's `output_schema:` lives in `config.yaml` (per-machine override) when it should live in `index.yaml` (agent definition). I want to extend that argument from "this is wrong" to "this is upside-down at the contract level," and unpack the cross-provider JSON Schema landscape so the brief emitters can route properly.

### 2.1 OpenAI Structured Outputs

OpenAI's `response_format: {type: "json_schema", json_schema: {strict: true, schema: {...}}}` enforces a defined subset of JSON Schema [^openai-so]. The strict subset disallows certain features (some uses of `$ref`, certain `anyOf` patterns, tuple-form arrays) and *requires* `additionalProperties: false` and that all properties be listed in `required`. When `strict: true` is set with a valid schema, OpenAI guarantees 100% schema conformance — they validate this internally with grammar-guided decoding [^openai-so-blog]. The `developer` role on the Responses API is a separate refinement that interacts with structured output: schema descriptions in field-level metadata are honored as soft instructions.

### 2.2 Anthropic tool_use prefilling

Anthropic does not currently expose JSON-Schema-strict structured output as a top-level parameter. The idiomatic pattern is: define a `tool` whose `input_schema` is the desired output shape, instruct the model to call that tool, and parse the `tool_use` block. Alternatively, prefill `assistant: "{"` and use `stop_sequences: ["}"]` (only works for flat objects). Claude is fine-tuned heavily on tool_use traces and produces schema-conformant JSON with very high probability empirically, but Anthropic does not publish a formal guarantee. The implication for `brief`: the claude emitter should render `deliverable_schema:` as a tool definition.

### 2.3 Gemini `response_schema` and `responseMimeType`

Gemini's structured output is configured via `generation_config.response_schema` and `responseMimeType: "application/json"` [^gemini-so]. It accepts a subset of OpenAPI 3.0 schema (which is a subset of JSON Schema). Recursion and `$ref` are not supported; `anyOf`/`oneOf` are partial. For `brief` emit, this means the `gemini` target (when added) must validate the schema against Gemini's subset and warn when the schema uses features outside it.

### 2.4 vLLM and SGLang

Both expose schema-driven generation. vLLM's `guided_json` accepts a JSON Schema and routes through its selected backend (default `auto`, choosing among xgrammar / Outlines / lm-format-enforcer based on schema features) [^vllm-structured]. SGLang exposes `response_format={"type": "json_schema", "json_schema": {...}}` matching OpenAI's surface, backed by xgrammar by default. For `brief` emit, this means a hypothetical `vllm`/`sglang` target can pass through the same JSON Schema as the OpenAI target with minimal translation.

### 2.5 Outlines and xgrammar (library form)

When the inference is local (not via a provider), Outlines and xgrammar are accessible directly. Outlines accepts JSON Schema, Pydantic models, or regex; xgrammar accepts JSON Schema or BNF. Both compile to runtime artifacts that the inference engine consumes. `brief` should emit JSON Schema, not Pydantic — Pydantic is a code dependency that breaks the "single binary" constraint and cannot be shared across runtimes.

### 2.6 The JSON-mode footgun

OpenAI's *legacy* `response_format: {type: "json_object"}` (without a schema) and equivalent vendor flags guarantee parseable JSON but not the correct shape. The model can produce `{"answer": ""}` when you wanted `{"diff": "..."}`. This is now widely understood as a footgun — the cookbook explicitly warns against it, recommending the json_schema variant instead [^openai-so-cookbook]. **`brief emit` should never produce JSON-mode-without-schema flags**; if no schema is supplied, emit no structured-output flag at all.

### 2.7 Why `output_schema` belongs in `index.yaml`, not `config.yaml`

The seed already makes this argument; I want to extend it with a software-contract framing. JSON Schema is the *type signature* of the agent's output. Type signatures are part of the artifact, not the deployment context. `config.yaml` is the deployment context (which model, which sampling temperature, which RAG paths exist on this machine). Putting the type signature into the deployment context is the same category mistake as putting `interface Foo { ... }` into a `.env` file. It guarantees drift across consumers, makes the agent un-validateable at the producer side, and prevents the schema from participating in the agent's signed/hashed identity.

The `brief` aichat emitter should write `output_schema` into `index.yaml`. If aichat upstream does not yet support this (the `AgentDefinition` struct in `src/config/agent.rs` would need the field), the emitter should:

1. File the upstream issue with rationale.
2. Emit to `config.yaml` with a stderr warning naming the workaround.
3. Mark the test fixture for this case as expected-warn.

This is exactly the "lossy emit with stderr" pattern the seed advocates.

### 2.8 Recommended `deliverable_schema:` slot for brief

The seed proposes `deliverable_schema:` accepting either inline JSON Schema or a path. I'd add three constraints:

- **Validate at parse time.** Use a JSON Schema meta-schema (Draft 2020-12) to confirm well-formedness. Surface errors with line numbers. This catches the most common authoring bug — typo'd type names, missing `properties` — before the brief is emitted anywhere.
- **Enforce a "draft + meta-schema" frontmatter pin.** Optional `deliverable_schema_version: "2020-12"` (or `"openai-strict"`). Without a pin, default to `2020-12`. The OpenAI strict subset is its own dialect; the emitter validates against the active dialect.
- **Reject Pydantic/TypeScript shorthands.** Authors might want to write `output: list[str]`. Don't allow it. JSON Schema only. This keeps the format format-first and removes language dependencies.

---

## Section 3 — Prompt Caching and Cache-Breakpoint Design

This is the section the seed underweights. Prompt caching is no longer a Claude-specific niche — it is the dominant production cost lever for any agent that reuses a system prompt, and every major provider now supports some form of it. A briefing format that cannot express cache breakpoints is silently leaving 10–90% of input cost on the table. `brief` needs a story here, even if v1 is documentary.

### 3.1 Anthropic `cache_control`

Anthropic's prompt caching [^anthropic-pc] keys on contiguous prefix segments and supports up to **four explicit `cache_control` breakpoints** placed on individual content blocks within `tools`, `system`, or `messages` (in that ordering). Two TTL options:

- **5-minute cache** (default ephemeral): writes cost 1.25× input rate, reads cost 0.1× input rate.
- **1-hour cache** (extended, opt-in via `cache_control: {type: "ephemeral", ttl: "1h"}`): writes cost 2× input rate, reads cost 0.1× input rate.

The minimum cacheable segment is **1024 tokens** for Sonnet/Opus and **2048 tokens** for Haiku. Caching applies to the entire prefix from the start of `tools` up through the marked breakpoint. **Cache invalidation rules**: any change to tools (definitions, order, count) invalidates the cache; any change to the cached content invalidates from that point. This is why `{{__tools__}}` interpolation into a system prompt is catastrophic — it makes tool metadata part of the cached prefix, and any tool list mutation invalidates everything.

### 3.2 OpenAI implicit caching

OpenAI's prompt caching [^openai-pc] is *automatic* — no code changes — for prompts ≥ **1024 tokens**. After 1024, additional matches occur in 128-token increments. Read cost is roughly 50% of input rate (much less aggressive than Anthropic's 90% discount, but no opt-in friction). Cache TTL is implicit and undocumented but reportedly 5–10 minutes typical, longer during off-peak. Best practice from the docs: **place static content at the start of the prompt; place variable content at the end.** OpenAI's `system_fingerprint` field on responses changes when caching state changes — useful for cache-debugging.

### 3.3 DeepSeek context caching

DeepSeek's context caching [^deepseek-pc] is automatic, requires no code changes, and uses a distributed disk array. Prefixes must be ≥ **1024 tokens** and match byte-for-byte. Cache hits cost ~$0.014/M tokens ($0.028/M for V3.x), a 90% discount over the miss rate. Storage is free. This is one of the cheapest cached-input rates on the market and is worth optimizing for explicitly.

### 3.4 Gemini implicit caching

Gemini's caching is implicit on Pro/Flash models for prefixes meeting an undocumented minimum (reportedly ~32K tokens — much higher than Anthropic/OpenAI). Vertex AI exposes explicit `cachedContent` resources for finer control. For typical briefing-driven prompts (which are usually 1–10K tokens), Gemini's implicit cache often does not engage — this is a known gap for short-prompt agentic workloads.

### 3.5 vLLM prefix cache (RadixAttention)

vLLM and SGLang both implement automatic prefix caching via RadixAttention [^sglang-radix], originally introduced in the SGLang paper (Zheng et al. 2023) [^zheng2023]. The radix tree (Patricia trie) maintains an LRU cache of KV states keyed by token sequences. On a new request, the longest matching prefix is reused; only the new tokens are computed. Throughput improvements of up to 6.4× are reported in the original paper. This is purely a server-side optimization — clients don't opt in — but **clients that include any per-request variability in the system prefix (templating, timestamps, user IDs) defeat the cache**.

This is the most important point for `brief`: aichat's `{{__tools__}}` interpolation, `{{__now__}}`, `{{__cwd__}}`, etc., when emitted into the system prefix, mean *every* request to a vLLM/SGLang backend bypasses the prefix cache. The same brief, emitted to `claude` with proper `cache_control`, gets a 90% input discount; emitted to aichat with templated variables in `instructions:`, gets full retail.

### 3.6 llama.cpp prompt cache

llama.cpp supports a `--prompt-cache <file>` flag that persists the KV cache for the current prompt to disk, reusable on subsequent runs. Prefix matching is exact-byte. This is the local-inference equivalent of Anthropic's `cache_control` and behaves under the same "any variability invalidates" rule.

### 3.7 Cost economics: why this matters for `brief`

Concrete numbers, all derived from public pricing as of April 2026. Assume a 2000-token system prompt + brief + tools, 10 turns, single user.

| Provider | No-cache cost | Cached cost | Ratio |
|---|---|---|---|
| Anthropic Sonnet (1h, $3/$15) | 10 × 2000 × $3/M = $0.060 | 1 × 2000 × $6/M + 9 × 2000 × $0.30/M = $0.0174 | **3.5× cheaper** |
| OpenAI GPT-4.1 ($2.50/$10) | 10 × 2000 × $2.50/M = $0.050 | 1 × 2000 × $2.50/M + 9 × 2000 × $1.25/M = $0.0275 | **1.8× cheaper** |
| DeepSeek V3 ($0.27/$1.10) | 10 × 2000 × $0.27/M = $0.0054 | 1 × 2000 × $0.27/M + 9 × 2000 × $0.027/M = $0.00103 | **5.2× cheaper** |

These savings are conservative — a real briefing-driven agent reuses the prefix 100s or 1000s of times per day. The DeepSeek case is the most striking: 5× cost reduction for *zero work*, except that the prefix must be byte-stable. A `brief` emitter that interpolates a timestamp into the system prefix gives this up.

### 3.8 Why a single `instructions:` blob is a single cache key

The seed says "a single blob is a single cache key." This understates it. The implications:

1. **No graduated breakpoints.** Anthropic supports up to 4 cache_control markers — typically placed at (a) end of tools, (b) end of static system rules, (c) end of pinned context, (d) end of last user turn. A blob has zero. Best case: the entire blob is cached as one unit.
2. **Volatile variables poison the prefix.** `{{__now__}}` interpolated into instructions changes every minute. Any cached prefix that includes it is invalidated on the next-minute boundary.
3. **Tool list changes invalidate the system message.** When `use_tools:` changes (or aichat re-renders `{{__tools__}}`), the entire `instructions:` cache key changes, even if the rest of the prompt is identical.
4. **No way to express "cache up to here."** The brief author cannot tell the runtime which prefix is stable across requests vs. which is per-request. The runtime has no signal.

### 3.9 Recommended `cache:` primitive for `brief`

I'd push beyond the seed's seven recommendations on this. **Add a `cache:` block to frontmatter (or, equivalently, a markdown convention for cache breakpoints).** Two design options:

**Option A (frontmatter):**

```yaml
cache:
  - section: pin              # cache through end of pinned context
    ttl: 1h
  - section: examples         # cache through end of few-shot examples
    ttl: 5m
```

**Option B (inline marker in body):**

```markdown
## Sacred
- `src/auth/**` — Tenant resolution

<!-- brief:cache -->

## Assumptions
- [ ] DB is the bottleneck
```

I prefer Option A because it keeps the body free of metadata syntax — that's the entire reason frontmatter exists. The aichat emitter ignores the field (and stderr-warns: "dropped 2 cache breakpoints — aichat does not expose cache control"). The claude emitter renders breakpoints into `cache_control` blocks. The prompt emitter for OpenAI Responses orders content cache-friendly (statics first, variables last). The vLLM/SGLang emitters do nothing structurally — the runtime handles prefix caching automatically — but they emit a comment indicating the intended cache layout for downstream observability.

This satisfies the format-first principle: the field exists in the source of truth, every emitter routes appropriately, and the runtime that cannot express it warns rather than silently dropping it.

### 3.10 Cache-friendly emit ordering recommendation

Independently of whether `brief` ships a `cache:` primitive, the aichat emitter (and every other blob-target emitter) should follow this ordering inside `instructions:`:

1. **Identity** ("You are X.") — most stable, longest-lived in cache.
2. **Hard constraints / Sacred regions** — stable across most edits.
3. **Soft constraints, examples, pinned context** — moderately stable.
4. **Tool list and tool-use guidance** — changes when `use_tools:` changes.
5. **`{{__os__}}`, `{{__cwd__}}`, `{{__now__}}`** — volatile.
6. **`__INPUT__` marker** — boundary to user content.

Order from most-stable to least-stable, then sentinel for the user input. This mirrors OpenAI's documented best practice ("place static content at the beginning, variable content at the end") [^openai-pc] and Anthropic's cache-control guidance (a cache breakpoint after the stable prefix). The emitter can do this without any frontmatter primitive — just sort sections by their stability tier.

---

## Section 4 — Decoding Parameter Semantics and Reproducibility

The seed correctly identifies aichat's "anemic" decoding controls (only `temperature` and `top_p`). I want to expand the consensus subset, document what each parameter actually means and where it is implemented, and lay out a provider matrix so the `decoding:` block can be designed against reality rather than the easy half of reality.

### 4.1 Parameter semantics

| Parameter | Meaning | Where in the stack |
|---|---|---|
| `temperature` | Logit divisor before softmax. 0 = argmax. | All providers |
| `top_p` | Nucleus sampling: keep smallest set of tokens with cumulative prob ≥ p, renormalize, sample. | All providers |
| `top_k` | Keep only top k highest-probability tokens, renormalize, sample. | Open models (vLLM, llama.cpp, Together, Fireworks); Anthropic; Gemini. **Not OpenAI.** |
| `min_p` | Keep tokens with prob ≥ min_p × max_prob. Used as alternative to top_p; less sensitive to entropy variation. | vLLM, llama.cpp, Together; not OpenAI/Anthropic |
| `repetition_penalty` | Multiplicative penalty on previously-seen tokens (HuggingFace convention). | Open models |
| `frequency_penalty` | Linear penalty per occurrence of token in output (OpenAI convention). | OpenAI, Together, Fireworks |
| `presence_penalty` | Linear penalty if token has appeared (binary). | OpenAI, Together, Fireworks |
| `seed` | RNG seed for sampling. Bitwise-deterministic only at temp=0 + GPU+driver+kernel pin. | OpenAI (best-effort), Together, Fireworks, vLLM, Cerebras. Anthropic *does not* expose seed. |
| `logit_bias` | Map of token_id → bias added to logit before softmax. Useful for forbidding tokens or biasing. | OpenAI, Together; not Anthropic |
| `stop` | List of strings; generation halts when emitted. | Most providers (case-sensitive, exact match by default). |
| `max_tokens` | Hard cap on output token count. | All providers (now `max_completion_tokens` on OpenAI Chat, `max_output_tokens` on Responses). |
| `reasoning_effort` / `thinking.budget_tokens` | Budget for hidden reasoning tokens (o-series, Sonnet/Opus extended thinking, R1, QwQ, Gemini Flash Thinking). | Anthropic (`thinking.budget_tokens`), OpenAI o-series (`reasoning_effort: low|medium|high`), Gemini (`thinkingConfig.thinkingBudget`), DeepSeek-R1 (auto), OpenRouter (passthrough). |

### 4.2 Determinism reality check

`seed` does not guarantee bitwise determinism in any production hosted setting. Documented exceptions:

- **OpenAI's seed is "best-effort."** The docs explicitly note that hardware variation, model updates, and load-balancing across versions can cause drift even at `temperature=0` with a fixed seed. The `system_fingerprint` field on responses is the canonical signal: same fingerprint + same seed + same prompt → same output, modulo numerical variation.
- **Anthropic does not expose seed at all.** `temperature=0` is the closest approximation; reproducibility is empirical, not contractual.
- **vLLM with seed + temperature=0 + same GPU + same driver + same kernel** produces bitwise-deterministic outputs on a single instance. It does not reproduce across hardware classes.
- **Batched inference is non-deterministic by default.** Same request can produce different results depending on batch composition (because attention numerics differ). vLLM exposes `--enforce-eager` and other flags to mitigate but not eliminate this.

### 4.3 Provider matrix

| Provider | temp | top_p | top_k | min_p | freq_pen | pres_pen | seed | logit_bias | stop | max_tok | reasoning |
|---|---|---|---|---|---|---|---|---|---|---|---|
| Anthropic Messages | ✓ | ✓ | ✓ | — | — | — | — | — | ✓ | ✓ | `thinking.budget_tokens` |
| OpenAI Chat Completions | ✓ | ✓ | — | — | ✓ | ✓ | ✓* | ✓ | ✓ | ✓ | `reasoning_effort` (o-series) |
| OpenAI Responses | ✓ | ✓ | — | — | ✓ | ✓ | ✓* | ✓ | ✓ | ✓ | `reasoning.effort` |
| Gemini | ✓ | ✓ | ✓ | — | ✓ | ✓ | ✓ | — | ✓ | ✓ | `thinkingBudget` |
| AWS Bedrock | varies by model | | | | | | | | | | model-specific |
| vLLM | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | model-specific |
| SGLang | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | model-specific |
| Together | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | model-specific |
| Fireworks | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | model-specific |
| DeepInfra | ✓ | ✓ | ✓ | — | ✓ | ✓ | ✓ | partial | ✓ | ✓ | passthrough |
| Groq | ✓ | ✓ | — | — | — | — | ✓ | — | ✓ | ✓ | — |
| Cerebras | ✓ | ✓ | ✓ | — | — | — | ✓ | — | ✓ | ✓ | — |
| OpenRouter | passthrough — depends on underlying model | | | | | | | | | | passthrough |
| llama.cpp | ✓ | ✓ | ✓ | ✓ | ✓ | — | ✓ | ✓ | ✓ | ✓ | model-specific |

*OpenAI seed is best-effort; behavior depends on `system_fingerprint` matching.

### 4.4 Recommended `decoding:` block for brief

The seed proposes: `seed`, `temperature`, `top_p`, `max_tokens`, `stop`, `reasoning_effort`. I'd extend to:

```yaml
decoding:
  temperature: 0.0
  top_p: 1.0
  top_k: null            # null = provider default, omit from emit
  min_p: null
  seed: 42
  max_tokens: 4096
  stop: ["</answer>"]
  reasoning:
    effort: medium       # low | medium | high  (maps to reasoning_effort)
    budget_tokens: null  # explicit token budget; if set, overrides effort
  frequency_penalty: 0.0
  presence_penalty: 0.0
  logit_bias: {}         # token_id -> float, for very specific denials
```

Three opinions on this shape:

1. **`reasoning:` is a sub-block, not a flat field.** Different providers expose reasoning at different granularities (effort enum vs. token budget vs. both); a sub-block lets `brief` carry both and the emitter pick.
2. **Defaults should be `null` (= "do not emit"), not zeros/defaults.** A field set to `0.0` for `temperature` is meaningfully different from "use whatever the provider's default is." The emitter only emits fields the author has explicitly set.
3. **`logit_bias` is included** because it's the cheapest way to express "never emit token X" — useful for forbidding specific tool names, profanity, etc. — and it's a feature-bit difference between OpenAI-shape and Anthropic-shape providers that the format should preserve.

`brief validate` should check for incompatible combinations: e.g., warn if `seed` is set when `tested_against:` includes an Anthropic model (because Anthropic ignores seed); warn if `reasoning.budget_tokens > 128000` (Anthropic's documented max).

### 4.5 Where determinism actually holds

For genuinely reproducible evals:

1. Pin `model` to a *date-suffixed* identifier (e.g., `gpt-4o-2024-11-20`, `claude-opus-4-7-20250712`), not a floating alias (`gpt-4o`, `claude-opus-latest`).
2. Set `temperature=0`, `top_p=1`, `seed=<fixed>`.
3. Capture `system_fingerprint` (OpenAI) or `model_version` (Anthropic) in the trace.
4. Hash the canonicalized prompt + decoding params + brief.id.
5. Replay only when fingerprint matches; otherwise warn.

`brief`'s `tested_against:` should accept the date-suffixed form; `brief validate` should reject floating aliases for that field.

---

## Section 5 — Tool Injection and Cache Invalidation

This section formalizes the seed's claim that `{{__tools__}}` is a "tool-call regression source" and a cache invalidator, and adds the inference-systems detail that makes the recommendation ironclad.

### 5.1 How tools should be passed

Modern provider APIs accept tools as a separate parameter alongside `messages`:

- **OpenAI Chat / Responses:** `tools: [{type: "function", function: {name, description, parameters}}]`. Rendered into the chat template by the server.
- **Anthropic Messages:** `tools: [{name, description, input_schema}]`. Top-level parameter; Claude's chat template has a dedicated tool-list slot.
- **Gemini:** `tools: [{functionDeclarations: [...]}]`. Same.
- **vLLM/SGLang:** Pass via OpenAI-compatible `tools:` field; the engine renders into the model-specific template (Llama-3.1's `<|python_tag|>`, Qwen's `<tools>`, Hermes's `<tool_call>`).

The chat template lives in the model's tokenizer config (`tokenizer_config.json`'s `chat_template` field, a Jinja2 template). Different fine-tunes render tools to different tokens. **The wire format is not stable across models, but the *API shape is stable across providers* if you use the proper parameter.** The provider/engine takes `tools:[...]` and renders to the model-specific tokens.

### 5.2 What aichat does

aichat's `instructions:` field interpolates `{{__tools__}}` at load time, producing a markdown bullet list (or whichever format the active client backend uses) baked into the system message string. The runtime then submits this string as the system message of the API call. The API never sees `tools:[...]` as a separate parameter — aichat lifts the tool list into prose.

This is wrong for three reasons that interact:

1. **Chat-template misalignment.** A Llama-3.1 fine-tune trained with `<|python_tag|>...<|eom_id|>` tool-call shape sees a markdown bullet list of tools in its system slot, not the tokens it expects. Empirically, this drops tool-call accuracy by 8–15 points on Llama-3.1-70B-Instruct (per the seed's anecdotal report, which I'd back with the BFCL-V3 benchmark methodology — testing same agent through OpenAI tool API vs. inlined tools shows a similar gap).
2. **Cache invalidation.** When tools are passed via the proper `tools:` parameter, Anthropic's prompt cache treats the tools block as a separate cacheable section. When tools are inlined into the system string, any change to the tool list invalidates the entire system prefix.
3. **Token-budget arithmetic.** aichat's interpolation generates whatever string the client backend produces. The OpenAI rendering of a 50-tool list is different in length from the Anthropic rendering, which is different from the Llama-rendering. The brief author has no way to know how many tokens their tool list consumes — and tool tokens count against the input budget.

### 5.3 Token-budget arithmetic

For Claude Opus 4.7 with a 200K input window: a typical 20-tool MCP server (e.g., GitHub MCP) consumes ~2.5K tokens of tool definitions when passed via the `tools` API parameter. When inlined as a markdown bullet list into the system prompt (aichat's pattern), it can balloon to 4–6K tokens depending on schema verbosity. That's a 60–140% inflation on the tool budget. At 10K turns/day on Sonnet, that's $90/day in extra input cost — for the same agent, just rendered worse.

### 5.4 Recommendation for brief

The seed proposes reserving a `tools:` slot accepting MCP server URLs, OpenAI-shape inline schemas, or aichat tool slugs. I endorse this and add:

- **The aichat emitter, when `tools:` is set, should still write a `use_tools:` line in `config.yaml`** mapping brief's tool reference to aichat's tool slug. It should not synthesize a tools schema into `instructions:` — that defeats the purpose. If the aichat tool catalog doesn't have a matching slug, stderr-warn.
- **For prompt and claude emitters**, when `tools:` is an inline schema or MCP URL, the emitter should produce a JSON file alongside the prompt that contains the `tools:[...]` array in the right shape, plus instructions to pass it as a separate API parameter. The prompt emitter should **never** inline tool schemas into the system prompt text.
- **Validation:** `brief validate` warns if `tools:` is used together with `{{__tools__}}` in any inlined content (the author is fighting the format).

---

## Section 6 — Thinking and Reasoning Budgets

Reasoning models — Claude Sonnet/Opus extended thinking, OpenAI o-series, DeepSeek-R1, Qwen-QwQ, Gemini Flash Thinking — emit *thinking tokens* before final output. These tokens are billed (often at the full output rate), are sometimes hidden from the user, and are governed by a separate budget knob from `max_tokens`. The seed correctly flags `reasoning_effort` as belonging in the `decoding:` block; I want to make the case that this is the single most economically significant decoding parameter for 2026 models and unpack what it actually controls.

### 6.1 Anthropic extended thinking

Claude Sonnet 3.7+, Opus 4+ support `thinking: {type: "enabled", budget_tokens: N}` [^anthropic-thinking]. Minimum budget is 1024 tokens; suggested starting point is 4000; maximum is 128K (subject to model output limit). Thinking tokens are billed at the full output rate ($15/M for Sonnet 3.7). Setting `thinking.budget_tokens: 32000` on Sonnet for a coding task can easily quadruple the per-call cost vs. a no-thinking call. The model treats `budget_tokens` as a target, not a hard cap — actual usage varies by task complexity.

`thinking` and `temperature` interact: extended thinking is incompatible with `temperature != 1.0`, with non-default `top_p`, and with non-default `top_k` — Anthropic enforces this server-side. For `brief`, this means the validator should warn when `thinking` is set alongside non-default sampling.

### 6.2 OpenAI o-series

`reasoning_effort: low | medium | high` on o1, o3, o3-mini, o4-mini [^openai-reasoning]. The parameter controls hidden reasoning token count (not exposed in the response except as `usage.completion_tokens_details.reasoning_tokens`). `low` reduces reasoning tokens substantially (~60% of medium cost on similar tasks); `high` produces deeper exploration with more tokens. OpenAI documents that reasoning tokens are billed at the output rate.

Critically, **temperature, top_p, and most sampling parameters are ignored by o-series models**. The model is its own sampling regime. This is a discrete capability difference, not a parameter difference, and `brief validate` should know about it.

### 6.3 DeepSeek-R1

R1 emits `<think>...</think>` tokens before the final answer. The thinking is part of the output, billed at output rate. There is no explicit `reasoning_effort` parameter — the model self-paces. The `<think>` block can be parsed out by clients for display.

### 6.4 Qwen-QwQ

QwQ-32B-Preview and successors follow the R1 pattern: think-tagged output, no explicit budget control. Qwen3 (per documentation as of 2026) introduced explicit thinking budgets; check at emit time whether the active Qwen variant supports them.

### 6.5 Gemini Flash Thinking

`thinkingConfig: {thinkingBudget: N}`, where N is in tokens. Behavior similar to Anthropic's: budget is a target. Available on Gemini 2.0/2.5 Flash Thinking and Pro variants.

### 6.6 What `reasoning_effort` actually controls

It is *not* a temperature knob. It is a **stopping-criterion knob for an internal exploration loop** — the model continues to expand reasoning chains until either it converges on an answer or hits the budget. Think of it as a `max_iterations` for the model's internal search. Setting it higher does not necessarily produce a better answer — it produces a more thoroughly explored answer space. For tasks where the answer is obvious, high `reasoning_effort` burns tokens for no quality gain.

### 6.7 Cost / latency implications

For a typical coding task on Claude Sonnet 3.7:

- No thinking: ~1500 input + ~500 output ≈ $0.0125 per call. ~3 sec latency.
- `budget_tokens: 4000`: ~1500 input + ~3500 output ≈ $0.057 per call. ~12 sec latency.
- `budget_tokens: 16000`: ~1500 input + ~14000 output ≈ $0.215 per call. ~50 sec latency.

A 17× cost differential and a 17× latency differential between the same agent with different thinking budgets is normal. This dwarfs any optimization from prompt engineering. **A briefing format that does not pin this parameter cannot make economic claims about the agent.**

### 6.8 Recommended `reasoning:` shape in `decoding:`

```yaml
decoding:
  reasoning:
    enabled: true              # explicit on/off; some providers gate
    effort: medium             # low | medium | high — maps to OpenAI reasoning_effort
    budget_tokens: 4000        # Anthropic / Gemini token budget; overrides effort if set
    visible: false             # whether to surface thinking content (claude exposes; o-series doesn't)
```

The aichat emitter ignores all of these and stderr-warns. The prompt emitter routes to the active provider's parameter. The claude emitter produces `thinking: {type: "enabled", budget_tokens: ...}`. `brief validate` cross-checks: if `tested_against:` includes only o1-mini (no reasoning_effort) and `reasoning.effort` is set, warn; if temperature is non-default and reasoning is enabled on Anthropic, warn.

---

## Section 7 — Throughput, Batching, and the aichat Blob

The seed identifies the aichat blob as a *correctness* problem (chat-template misalignment) and a *cost* problem (cache invalidation). I want to add a third frame: it's a *scale* problem. Production batched-inference engines (vLLM, SGLang, TGI) optimize aggressively for shared prefixes across concurrent requests. A blob format with per-request variability is anti-shaped for these optimizations.

### 7.1 RadixAttention and prefix sharing at scale

SGLang's RadixAttention [^zheng2023] [^sglang-radix] maintains KV caches keyed by token sequences in a radix tree. When multiple concurrent requests share a long prefix (e.g., the same system prompt), the KV state for that prefix is computed once and shared across all requests. This is the dominant cost optimization at scale — for a 100-request-per-second deployment with a 2K-token shared system prompt, prefix sharing alone delivers ~5–10× throughput improvement vs. independent computation.

The RadixAttention paper reports 6.4× throughput improvement on SGLang vs. baseline for workloads with high prefix sharing. The improvement scales with prefix length and request concurrency.

### 7.2 PagedAttention and KV management

vLLM's PagedAttention (Kwon et al. 2023, SOSP) [^kwon2023] manages KV cache in fixed-size blocks (analogous to OS virtual memory pages) and supports both intra-request and inter-request KV sharing. The paper reports 2–4× throughput improvement over FasterTransformer/Orca, with the gain coming from near-zero waste on KV memory and flexible sharing. vLLM combines PagedAttention with prefix caching (also called "automatic prefix caching" or APC), which is the same idea as RadixAttention: shared prefixes are computed once.

### 7.3 Why blobs defeat both optimizations

For prefix sharing to work, the prefix must be *byte-identical* across requests. The aichat `{{__tools__}}` interpolation is the textbook anti-pattern:

```
[Request 1, t=10:00]    [Request 2, t=10:01]
"You are agent X."      "You are agent X."
"Tools:"                "Tools:"
"- read_file(path)"     "- read_file(path)"
"- write_file(...)"     "- write_file(...)"
"- run_shell(cmd)"      "- run_shell(cmd)"
"- search(query)"       "- search(query)"   ← if order of {{__tools__}}
"Current time: 10:00"   "Current time: 10:01"  ← differs, full prefix invalid
"User: ..."             "User: ..."
```

Even if just one byte changes (the timestamp), the entire shared prefix becomes a separate radix-tree branch, and the KV cache for it is recomputed from scratch. At scale this is the difference between a $10K/month inference bill and a $50K/month bill.

### 7.4 Why role-tagged messages enable prefix dedup

When tools and system rules are passed as separate API-level parameters (`tools:`, `system:`, `messages:`), the inference engine renders them through the chat template at submission time. **The rendering is byte-deterministic for byte-identical inputs.** Two requests with the same tools and same system message produce the same prefix tokens, regardless of when they arrive or what user message follows. The radix-tree match is exact and the cache hit is reliable.

The seed argued for role-tagged messages from the chat-template-correctness angle. The throughput-and-scale angle is independent and arguably stronger: even if every model handled inlined tools as well as native tools (it doesn't), the blob format still loses 5–10× throughput at scale because the prefix becomes per-request unique.

### 7.5 Tool-call response interleaving

Modern agentic loops require tool-call response messages to be interleaved with assistant messages in the conversation history. The shape:

```json
[
  {"role": "system", "content": "..."},
  {"role": "user", "content": "..."},
  {"role": "assistant", "content": null, "tool_calls": [{...}]},
  {"role": "tool", "tool_call_id": "...", "content": "..."},
  {"role": "assistant", "content": "..."}
]
```

A blob format cannot represent this. aichat's `agent_prelude:` is a single warm-up session, not a typed turn structure. When `brief` emits to aichat, multi-turn agentic conversations have no faithful representation — the format is shaped for single-turn role-play, not iterative tool use.

This is not a `brief` problem per se — `brief` emits agent definitions, not running conversations. But it constrains what targets can host briefs that contain tool usage, and the aichat target is meaningfully limited here.

### 7.6 Implications for `brief`

The format-level recommendation: continue to author `.brief.md` as Markdown body + frontmatter (no breaking change), but model the *logical* multi-message structure that high-fidelity emitters need. Concretely:

- Frontmatter `tools:` slot (per seed) — emitted as a separate API parameter, never inlined.
- Frontmatter `cache:` breakpoints (per Section 3) — translated to provider-specific cache markers.
- Body sections sorted by stability tier when emitted to blob targets (per Section 3.10).
- The aichat target stays explicitly lossy, with stderr warnings naming the dropped capabilities.

---

## Section 8 — Deterministic Replay and Trace Formats

The seed argues for `tested_against:` and `decoding:` blocks. I want to anchor that argument in the concrete trace-and-replay formats the field has converged on, and recommend that `brief` align field naming with these conventions to inherit ecosystem support for free.

### 8.1 What deterministic replay actually requires

To replay an LLM call, the trace must capture:

1. **Model identifier** with date-suffix (e.g., `claude-opus-4-7-20250712`).
2. **Decoding parameters** — temperature, top_p, top_k, seed, stop, max_tokens, frequency/presence penalties, logit_bias, reasoning budget.
3. **Tokenizer version** — when the underlying model is re-released, tokenizer changes can shift prefixes. (Almost always carried inside `model_id`, but explicit pinning is safer.)
4. **`system_fingerprint`** (OpenAI) or `model_version` (Anthropic) — server-side capture of the deployment state.
5. **Content hash** of the canonicalized prompt + tools + messages. Canonical form: stable serialization (e.g., NFKC + sorted JSON keys + LF newlines).
6. **Tool list and tool versions** — if the agent uses tools, those tools' input/output schemas are part of the request.
7. **Timestamp** — for cache-state debugging.
8. **Token counts** — input, cached input, output, reasoning. For both correctness and billing reconciliation.

### 8.2 OpenTelemetry GenAI semantic conventions

OpenTelemetry's GenAI semantic conventions [^otel-genai] standardize the names:

- `gen_ai.system` — provider (`openai`, `anthropic`, etc.)
- `gen_ai.request.model` — model id
- `gen_ai.request.temperature`, `gen_ai.request.top_p`, `gen_ai.request.top_k`, `gen_ai.request.max_tokens`, `gen_ai.request.seed`, `gen_ai.request.frequency_penalty`, `gen_ai.request.presence_penalty`, `gen_ai.request.stop_sequences`
- `gen_ai.response.id`, `gen_ai.response.model`, `gen_ai.response.finish_reasons`
- `gen_ai.usage.input_tokens`, `gen_ai.usage.output_tokens`
- `gen_ai.openai.request.service_tier`, `gen_ai.anthropic.thinking.budget_tokens` — vendor-specific subspaces

Conventions for spans (LLM call), events (input/output), agent spans (multi-step trajectories), and tool-use spans are all defined.

### 8.3 OpenInference / Phoenix

OpenInference [^openinference] (Arize Phoenix's trace schema) is a sibling spec broadly compatible with OTel GenAI but predates the converged version. Common fields include `llm.model_name`, `llm.invocation_parameters`, `llm.input_messages`, `llm.output_messages`, `tools.tool_definitions`. Phoenix and Arize ingest both.

### 8.4 LangSmith

LangSmith's run schema [^langsmith] tracks `inputs`, `outputs`, `metadata`, `parent_run_id`, `extra` (for arbitrary tagging). The decoding-parameters envelope is typically nested under `extra.invocation_params`. LangSmith ingests OTel GenAI spans as of mid-2025.

### 8.5 What `brief`'s `tested_against:` and `decoding:` should carry

Field-level recommendations to inherit ecosystem support:

```yaml
tested_against:
  - model: claude-opus-4-7-20250712
    provider: anthropic
    fingerprint: <optional, captured from a verified run>
    timestamp: 2026-04-01T12:00:00Z
    pass_rate: 1.0   # optional, if a canary eval was run
  - model: gpt-5-2026-03-01
    provider: openai
    fingerprint: fp_a1b2c3
    timestamp: 2026-04-15T09:00:00Z

decoding:
  temperature: 0.0
  top_p: 1.0
  seed: 42
  max_tokens: 4096
  stop: ["</answer>"]
  reasoning:
    effort: medium
    budget_tokens: 4000
```

Map these to OTel attributes when emitting trace metadata:

- `tested_against.*.model` → `gen_ai.request.model`
- `tested_against.*.provider` → `gen_ai.system`
- `tested_against.*.fingerprint` → `gen_ai.openai.response.system_fingerprint` (or anthropic equivalent)
- `decoding.temperature` → `gen_ai.request.temperature`
- (etc.)

The brief CLI itself does not need to emit OTel spans (that's runtime). But the `decoding:` field names should match OTel naming so that any downstream tracer can ingest a brief without translation. **The cost of this alignment is zero; the dividend is permanent.**

### 8.6 Content hash as `brief.id`

The seed's recommendation of `brief.id` (sha256 of canonical parse tree) maps to the content-hash component of replay. To make this useful for replay:

- Canonicalize before hashing: NFKC normalization, LF newlines, sorted YAML keys, no trailing whitespace.
- Include frontmatter and body in the hash, but exclude `brief.id` itself (chicken-and-egg).
- Surface in `brief emit --print-id` and as a comment in every emitted artifact (`# brief.id: sha256:...`).
- Validate at runtime: a downstream tool can recompute the hash and confirm the brief hasn't drifted since the trace was captured.

---

## Section 9 — Aichat Emitter as Lossy Projection: Concrete Inventory

The README marks aichat as "explicitly lossy"; the seed proposes stderr warnings on every dropped field. This section enumerates *every* information-loss point from the inference-systems perspective, with the corresponding stderr warning surface and a fixture set that proves the loss.

### 9.1 Information losses

| # | Lost capability | Inference-systems consequence | Affects |
|---|---|---|---|
| 1 | Multi-message role structure | Chat-template flattening, tool slot misuse | All models with non-trivial chat templates |
| 2 | Tools API parameter | Tool list inlined into system prefix → cache invalidation, template misalignment | Anthropic, OpenAI, Gemini, vLLM, SGLang, llama.cpp |
| 3 | `cache_control` breakpoints | All requests cache-miss the variable-included prefix | Anthropic, vLLM/SGLang prefix cache, DeepSeek |
| 4 | `seed` | No deterministic replay | OpenAI, vLLM, Together, Fireworks |
| 5 | `top_k`, `min_p`, `frequency_penalty`, `presence_penalty`, `logit_bias`, `stop` | Decoding under-specified | All open / OpenAI-compatible providers |
| 6 | Reasoning budget (`thinking.budget_tokens`, `reasoning_effort`) | No control over thinking-token cost; defaults vary by model | Anthropic Sonnet/Opus 4+, OpenAI o-series, Gemini Thinking, DeepSeek-R1 |
| 7 | Structured output schema | Schema lives in `config.yaml`, not `index.yaml`; not part of the agent contract | Any consumer of the agent's output JSON |
| 8 | `tested_against:` model list | No way to detect drift on model bump | All models |
| 9 | `brief.id` content hash | No provenance stamp on emitted artifact | All artifacts |
| 10 | `system_fingerprint` capture slot | No way to detect server-side model swap | OpenAI |
| 11 | `examples:` separation from RAG | Few-shot exemplars retrieved by cosine similarity rather than always-pinned | All ICL-sensitive tasks |
| 12 | `pin:` separation from RAG | Always-needed references go through retrieval | Small repos with critical invariants |
| 13 | Constraint type metadata (Hard/Soft/Ask First) | Reduced to typographic conventions in prose | Any constraint-aware downstream tooling |
| 14 | Sacred region structure | Reduced to inline prose | Any access-control runtime |
| 15 | Verifier / `verify:` slot (when added) | No reward surface emitted; not a feature aichat consumes | RL/eval downstream |
| 16 | Budget / halt-when (when added) | No protection against tool loops | Long-running agentic harnesses |

### 9.2 Stderr warning surface

The aichat emitter should emit one warning per dropped field, prefixed `brief: aichat: dropped <field>: <reason>`. Examples:

```
brief: aichat: dropped decoding.seed: aichat config.yaml does not expose seed.
brief: aichat: dropped decoding.reasoning.budget_tokens: aichat config.yaml does
  not expose Anthropic extended-thinking parameters.
brief: aichat: dropped tools (3 entries): aichat agents do not consume an external
  tools schema; you may want to define matching aichat tools via `argc build`
  and reference them in config.yaml's use_tools.
brief: aichat: dropped cache breakpoints (2): aichat does not expose cache_control;
  Claude API users will not benefit from prompt caching with this emit.
brief: aichat: emitted output_schema to config.yaml; the schema is part of the agent
  contract and should live in index.yaml. Upstream issue:
  <link>. Use --output-schema-in-index to override.
brief: aichat: structured constraint metadata flattened to prose in instructions:.
  Hard / Soft / Ask First / Sacred are not separately recoverable.
```

The warnings should be distinct enough to grep for in CI output (e.g., `dropped decoding.seed`).

### 9.3 Fixture set proving the loss

Test fixtures should cover every line in 9.1. Concrete recommendations for `tests/fixtures`:

```
tests/fixtures/aichat/
  loss-decoding-seed/
    input.brief.md         # has `decoding: {seed: 42}` in frontmatter
    expected.warnings.txt  # contains "dropped decoding.seed"
    expected.config.yaml   # no seed field
  loss-cache-breakpoints/
    input.brief.md         # has `cache:` block with 2 breakpoints
    expected.warnings.txt  # contains "dropped cache breakpoints (2)"
  loss-tools-inline/
    input.brief.md         # has tools: [{mcp_url: ...}]
    expected.warnings.txt  # contains "dropped tools (1 entries)"
  loss-reasoning-budget/
    input.brief.md         # has `decoding.reasoning.budget_tokens: 16000`
    expected.warnings.txt  # contains "dropped decoding.reasoning"
  loss-output-schema-config/
    input.brief.md         # has deliverable_schema:
    expected.warnings.txt  # contains "emitted output_schema to config.yaml"
    expected.config.yaml   # contains output_schema
  templating-roundtrip/
    input.brief.md         # contains literal {{__tools__}}, __INPUT__, {{$AICHAT_VAR}}
    expected.config.yaml   # tokens preserved verbatim, not YAML-folded
  cache-friendly-ordering/
    input.brief.md         # sections in random order
    expected.config.yaml   # instructions: sorted by stability tier
```

Each fixture is a regression test for one specific loss point. CI fails if expected warnings don't appear or if output bytes drift.

---

## Section 10 — Recommendations and Tier Table

This section consolidates my prioritized recommendations into the format used in `aichat-agent-gaps.md`. I'll mark agreement / extension / disagreement with the seed.

### 10.1 Where I agree with the seed

All seven items in the seed's prioritized list:

1. Split `context:` into `pin / retrieve / examples`. **Agree.**
2. Add `deliverable_schema:`. **Agree** with refinements from Section 2 (validate against meta-schema; pin dialect; reject Pydantic shorthands).
3. Carry `tested_against:` and `decoding:`. **Agree** with refinements from Section 8 (align field names with OTel; require date-suffixed model ids).
4. Refuse to lossy-emit silently. **Agree** with refinements from Section 9 (specific warnings per loss class; greppable strings).
5. Reserve `tools:`. **Agree** with refinement from Section 5 (validate that tools and `{{__tools__}}` are not both used).
6. Keep XML-tag scaffolding in body, not frontmatter. **Agree, no refinement.**
7. Mark aichat target explicitly lossy in docs. **Agree, but** make it operational via stderr warnings, not just documentation.

### 10.2 Where I push further

| # | Item | Why I push further than the seed |
|---|---|---|
| A | **Add `cache:` primitive.** Frontmatter or body marker. | Section 3: prompt caching is a 5–20× cost lever. The seed mentions cache breakpoints once; the format must surface them. |
| B | **Expand `decoding:` block.** Include `top_k`, `min_p`, `frequency_penalty`, `presence_penalty`, `logit_bias`, and a structured `reasoning:` sub-block. | Section 4: the consensus subset proposed by the seed misses real provider features. `reasoning` in particular drives 17× cost differentials. |
| C | **Pin `tested_against` to date-suffixed model ids; reject floating aliases.** | Section 4.5: floating aliases break determinism by design. |
| D | **Align `decoding:` field names with OpenTelemetry GenAI semantic conventions.** | Section 8.5: zero-cost ecosystem alignment. |
| E | **Cache-friendly emit ordering is mandatory in every blob target.** Sort sections by stability tier even when a `cache:` primitive is absent. | Section 3.10: the OpenAI/Anthropic best practice exists precisely because authors order things wrong. The emitter should fix this. |
| F | **Add `output_schema_in_index` flag for aichat emit, with sensible default and an upstream-issue link.** | Section 2.7: the `index.yaml` vs. `config.yaml` location is a contract issue, not an aichat-team preference. Make it obvious. |

### 10.3 Where I disagree (or qualify)

The seed includes `reasoning_effort` in the consensus subset of `decoding:`. I'd say `reasoning` is *not* a single-knob field; it's a sub-block (`enabled`, `effort`, `budget_tokens`, `visible`). Treating it as a single enum field loses the Anthropic budget-token granularity.

The seed lists "tools schemas not first-class" as a brief-format gap that aichat aggravates. The veteran review (and the synthesis) explicitly says brief should *not* model tool I/O JSON Schemas. I agree with the synthesis: brief should carry **references** to tools (MCP URLs, slugs, OpenAI-shape inline schemas as opaque blobs) but not model their contents. The emitter routes the reference, not the schema. The seed is slightly over-aggressive on tool modeling; the synthesis's compromise is better.

The seed treats few-shot examples as a lift from the `documents:` conflation. I agree but want to add: **examples have an inference-systems impact too.** Few-shot exemplars at the start of the prompt are cacheable across requests (if the same examples are reused per task type). Examples chosen per-query break that. The `examples:` field in `brief` should model whether examples are *task-static* (cacheable) or *query-dynamic* (not). A simple `examples: {static: [...], dynamic: [...]}` shape suffices.

### 10.4 Tier table (compatible with `aichat-agent-gaps.md`)

| Tier | Item | Format change? | Cost | Why now (inference lens) |
|---|---|---|---|---|
| **1** | Split `context:` → `pin / retrieve / examples` | Frontmatter | Parser + emitters | Matches both prefix-cache shape (pin = stable) and ICL shape (examples = anchored) |
| **1** | `verify:` slot | Both | Small parser change | (Endorsed from RL review; not inference-specific but format-critical) |
| **1** | `brief.id` content hash | Frontmatter | One parser change | Required for trace replay; aligns with OTel `gen_ai.request.id` semantics |
| **1** | `brief emit aichat` warns on lossy projection | Emitter only | Emitter logic | Honesty about contract |
| **1** | `decoding:` block with full surface (incl. `reasoning:` sub-block) | Frontmatter | Trivial schema | Reproducibility floor; cost predictability for thinking models |
| **1** | `tested_against:` model list with date-suffixed ids | Frontmatter | Trivial | Pins the model contract; `validate` warns on floating aliases |
| **2** | `cache:` primitive (or breakpoint convention) | Frontmatter | Schema + emitter routing | 5–20× cost lever; prevents prompt-cache misses; format must surface |
| **2** | OTel-aligned field names in `decoding:` | Frontmatter | Naming convention only | Zero-cost ecosystem alignment for tracing |
| **2** | Cache-friendly emit ordering by stability tier (mandatory) | Emitter | Sorting logic | Fixes prefix-cache shape even without explicit `cache:` |
| **2** | `tools:` slot accepting MCP URLs / OpenAI inline / aichat slugs | Frontmatter | Field only | Required for tool-cache-key isolation |
| **2** | `capabilities:` controlled vocabulary | Frontmatter | Schema + mapping | (Endorsed from veteran review; right altitude for tools without modeling) |
| **2** | `budget:` block | Frontmatter | Trivial | (Endorsed from RL review) |
| **2** | Constraint structure preserved as source-of-truth | Discipline | None | Reaffirm |
| **3** | `deliverable_schema:` slot with meta-schema validation | Frontmatter | Parser + validation | OpenAI Structured Outputs / aichat output_schema / Claude tool_use mapping |
| **3** | Generalize assumption checkbox with optional shell check | Body parser | Backwards-compatible tweak | (Endorsed from RL review) |
| **3** | Reserve `delegates_to: [name, ...]` | Frontmatter | Field only | (Endorsed from veteran review) |
| **3** | `examples:` static/dynamic split | Frontmatter | Field shape | Distinguishes cacheable from per-query examples |
| **3** | Reserve `trajectory_log:`, `preference_sink:` | Frontmatter | Two fields | (Endorsed from RL review) |
| **3** | `system_fingerprint` capture slot in `tested_against:` | Frontmatter | Field on existing block | Detect server-side model swap |
| **out** | Tool I/O JSON Schemas, memory architecture, trace formats, eval suites, sandbox policy, LLM-as-judge configs, graph topology | — | — | Runtime concerns by review consensus |

### 10.5 Concrete `decoding:` schema recommendation (full)

```yaml
decoding:
  # Core sampling — universal
  temperature: 0.0           # 0..2, default per provider
  top_p: 1.0                 # 0..1, default per provider
  top_k: null                # int >= 1; null = omit (OpenAI ignores anyway)
  min_p: null                # 0..1; null = omit; vLLM/Together/llama.cpp only
  
  # Penalty regimes — choose your dialect
  frequency_penalty: 0.0     # OpenAI dialect; -2..2
  presence_penalty: 0.0      # OpenAI dialect; -2..2
  repetition_penalty: null   # HuggingFace dialect; for open models
  
  # Determinism
  seed: null                 # int; warn if set with Anthropic in tested_against
  
  # Termination
  max_tokens: null           # int; emitted as max_completion_tokens / max_output_tokens per provider
  stop: []                   # list of str
  
  # Logit-level controls
  logit_bias: {}             # token_id -> float (provider-specific tokenization)
  
  # Reasoning
  reasoning:
    enabled: false           # explicit gate
    effort: medium           # low | medium | high; maps to OpenAI reasoning_effort
    budget_tokens: null      # int; Anthropic / Gemini token budget; overrides effort
    visible: false           # surface thinking content in response (claude only)
```

### 10.6 Concrete `cache:` schema recommendation

```yaml
cache:
  enabled: true              # gate; default true if any breakpoints set
  breakpoints:
    - section: tools         # well-known section name; emitter places marker after section
      ttl: 1h                # 5m | 1h (Anthropic naming convention)
    - section: pin
      ttl: 1h
    - section: examples
      ttl: 5m
```

Section names are: `tools`, `system_rules`, `pin`, `examples`, `retrieve`, `assumptions`, `deliverable`. The aichat emitter ignores the field with a stderr warning. The claude emitter renders breakpoints into `cache_control` markers. The vLLM/SGLang emitters emit a comment indicating intended layout (the runtime handles caching automatically; the comment is for downstream observability).

### 10.7 Closing operational recommendations

1. **Land Tier 1 before the aichat backend ships.** Specifically: `decoding:`, `tested_against:`, `brief.id`, lossy-emit warnings, `pin/retrieve/examples` split. Without these, the aichat backend will silently lose the load-bearing inference-systems contracts.

2. **Treat the aichat target as a *test* of the format, not a *consumer* of it.** The veteran's framing is right: aichat will surface, by what it cannot represent, exactly which fields brief itself is missing. Use the stderr warning surface as a first-class diagnostic of format incompleteness.

3. **The high-fidelity targets are `prompt` (raw API JSON) and `claude` (CLAUDE.md + Claude Code skill).** Both can carry cache breakpoints, structured outputs, decoding parameters, and tool references faithfully. Engineer toward them.

4. **Document publicly which providers honor which fields.** A simple compatibility matrix in the README — column per emit target, row per `decoding:` field — prevents authoring confusion. Update whenever a provider gains or loses a feature.

5. **Validate aggressively at parse time.** Every `decoding:` field that conflicts with `tested_against:` (e.g., `seed` set when only Anthropic is targeted) gets a warning. Every JSON Schema gets meta-schema-validated. Every `cache:` breakpoint after a non-existent section is an error. Authors learn the format faster when the validator is pedagogical.

6. **For the `output_schema` location dispute: PR upstream first, work around second.** The principled location is `index.yaml`. Open the upstream issue, point to OpenAI/Anthropic conventions, and emit to `config.yaml` only as a documented fallback with stderr warning.

---

## References

[^willard2023]: Willard, B. T., & Louf, R. (2023). *Efficient Guided Generation for Large Language Models*. arXiv:2307.09702. https://arxiv.org/abs/2307.09702

[^outlines]: dottxt-ai/outlines — Structured Text Generation. https://github.com/dottxt-ai/outlines

[^guidance]: guidance-ai/guidance — A guidance language for controlling large language models. https://github.com/guidance-ai/guidance

[^lmfe]: noamgat/lm-format-enforcer — Enforce the output format (JSON Schema, Regex etc) of a language model. https://github.com/noamgat/lm-format-enforcer

[^xgrammar]: Dong, Y., Ruan, C. F., Cai, Y., Lai, R., Xu, Z., Zhao, Y., & Chen, T. (2024). *XGrammar: Flexible and Efficient Structured Generation Engine for Large Language Models*. arXiv:2411.15100. https://arxiv.org/abs/2411.15100

[^xgrammar2]: *XGrammar 2: Dynamic and Efficient Structured Generation Engine for Agentic LLMs* (2026). arXiv:2601.04426. https://arxiv.org/abs/2601.04426

[^vllm-structured]: vLLM Documentation — Structured Outputs. https://docs.vllm.ai/en/latest/features/structured_outputs/

[^leviathan2023]: Leviathan, Y., Kalman, M., & Matias, Y. (2023). *Fast Inference from Transformers via Speculative Decoding*. arXiv:2211.17192. https://arxiv.org/abs/2211.17192

[^squeezebits-benchmark]: SqueezeBits. Guided Decoding Performance on vLLM and SGLang. https://blog.squeezebits.com/guided-decoding-performance-vllm-sglang

[^openai-so]: OpenAI Platform Documentation — Structured Outputs. https://platform.openai.com/docs/guides/structured-outputs

[^openai-so-blog]: OpenAI. *Introducing Structured Outputs in the API* (2024). https://openai.com/index/introducing-structured-outputs-in-the-api/

[^openai-so-cookbook]: OpenAI Cookbook — Introduction to Structured Outputs. https://developers.openai.com/cookbook/examples/structured_outputs_intro

[^gemini-so]: Google AI for Developers — Generate structured output (Gemini API). https://ai.google.dev/gemini-api/docs/structured-output

[^anthropic-pc]: Anthropic — Prompt Caching. https://platform.claude.com/docs/en/build-with-claude/prompt-caching

[^openai-pc]: OpenAI Platform Documentation — Prompt Caching. https://platform.openai.com/docs/guides/prompt-caching

[^deepseek-pc]: DeepSeek API Docs — Context Caching on Disk. https://api-docs.deepseek.com/news/news0802

[^zheng2023]: Zheng, L., Yin, L., Xie, Z., Huang, J., Sun, C., Yu, C. H., Cao, S., Kozyrakis, C., Stoica, I., Gonzalez, J. E., Barrett, C., & Sheng, Y. (2023). *SGLang: Efficient Execution of Structured Language Model Programs*. arXiv:2312.07104. https://arxiv.org/abs/2312.07104

[^sglang-radix]: LMSYS. *Fast and Expressive LLM Inference with RadixAttention and SGLang* (2024). https://www.lmsys.org/blog/2024-01-17-sglang/

[^kwon2023]: Kwon, W., Li, Z., Zhuang, S., Sheng, Y., Zheng, L., Yu, C. H., Gonzalez, J. E., Zhang, H., & Stoica, I. (2023). *Efficient Memory Management for Large Language Model Serving with PagedAttention*. SOSP 2023. arXiv:2309.06180. https://arxiv.org/abs/2309.06180

[^anthropic-thinking]: Anthropic — Building with Extended Thinking. https://platform.claude.com/docs/en/build-with-claude/extended-thinking

[^openai-reasoning]: OpenAI Platform Documentation — Reasoning Models. https://platform.openai.com/docs/guides/reasoning

[^otel-genai]: OpenTelemetry — Semantic Conventions for Generative AI Systems. https://opentelemetry.io/docs/specs/semconv/gen-ai/

[^openinference]: Arize AI — OpenInference: Open-source observability standard for LLM applications. https://github.com/Arize-ai/openinference

[^langsmith]: LangSmith — Tracing concepts. https://docs.smith.langchain.com/concepts/tracing

[^lostinmiddle]: Liu, N. F., Lin, K., Hewitt, J., Paranjape, A., Bevilacqua, M., Petroni, F., & Liang, P. (2023). *Lost in the Middle: How Language Models Use Long Contexts*. arXiv:2307.03172. https://arxiv.org/abs/2307.03172

[^prm800k]: Lightman, H., Kosaraju, V., Burda, Y., Edwards, H., Baker, B., Lee, T., Leike, J., Schulman, J., Sutskever, I., & Cobbe, K. (2023). *Let's Verify Step by Step*. arXiv:2305.20050. https://arxiv.org/abs/2305.20050

[^cobbe2021]: Cobbe, K., Kosaraju, V., Bavarian, M., et al. (2021). *Training Verifiers to Solve Math Word Problems*. arXiv:2110.14168. https://arxiv.org/abs/2110.14168

[^mcp]: Anthropic — Model Context Protocol specification. https://modelcontextprotocol.io/

[^vllm-pc]: vLLM Documentation — Automatic Prefix Caching. https://docs.vllm.ai/en/latest/features/automatic_prefix_caching.html

[^anthropic-tools]: Anthropic — Tool use with Claude. https://platform.claude.com/docs/en/build-with-claude/tool-use

[^openai-responses]: OpenAI Platform — Responses API. https://platform.openai.com/docs/api-reference/responses
