# Compiled Intent: What If Brief Could Emit Directly Into Latent Space?

**Date:** 2026-04-07
**Author:** Claude Opus 4.6
**Type:** Theoretical extrapolation / research synthesis
**Scope:** Take the `brief` concept to its maximum theoretical distance. What would it mean to "compile" a `.brief.md` into something closer to how models actually consume instructions — embeddings, continuous vectors, latent representations?

---

## The Premise

Brief currently operates at the **text layer**. A human writes structured intent in Markdown. An emitter translates it into a different text format (CLAUDE.md, XML tags, system prompt). The model tokenizes that text, embeds it, and processes it through attention layers to extract the intent.

Every step of that pipeline is lossy:

1. **Human intent → text**: Language is a bottleneck. "Do not modify the authentication module" is an approximation of the constraint the human actually holds in their head — a constraint that includes spatial intuition about the codebase, risk models about security, and implicit context about why this region is sacred.

2. **Text → tokens**: Tokenization is an arbitrary segmentation. "WebSocket" might be one token or two depending on the tokenizer. The semantic density per token is uneven.

3. **Tokens → behavior**: The model's interpretation of "NEVER modify" depends on where it appears in context, what surrounds it, attention dynamics, and the model's training distribution. arxiv 2411.10541 showed up to 40% performance variation just from *formatting* the same content differently.

The question: what if brief could skip the text layer entirely and emit directly into the representation the model actually operates on?

---

## The Landscape: What Exists Today

### Soft Prompts and Prefix Tuning

The most mature research in "compiled instructions" is **soft prompt tuning** (Lester et al., 2021) and **prefix tuning** (Li & Liang, 2021). Instead of prepending text tokens to the input, you prepend *learned continuous vectors* that occupy the same embedding space as token embeddings but are not constrained to correspond to any real token.

Key properties:
- A soft prompt is typically 10-100 vectors, each matching the model's hidden dimension (e.g., 4096 floats for a 4096-dim model).
- These vectors are learned via gradient descent on a task-specific objective.
- The frozen model treats them as "virtual tokens" — subsequent real tokens attend to them exactly as they would attend to real token embeddings.
- Storage: a 100-vector soft prompt for a 4096-dim model is ~400KB. A text prompt for the same task might be 2KB of text but occupy the same context window.

**What this means for brief**: A `.brief.md` with hard constraints, sacred regions, and a deliverable could theoretically be *compiled* into ~50-100 learned vectors that encode the same behavioral directives — but in the model's native representation space rather than in natural language.

### The Vector Prompt Manifesto (March 2026)

[arxiv 2603.04292](https://arxiv.org/abs/2603.04292) ("Position: Vector Prompt Interfaces Should Be Exposed to Enable Customization of Large Language Models") is the most directly relevant recent paper. Its core argument:

> Text-based prompts are discrete, semantically grounded artifacts whose influence on model behavior is mediated through linguistic interpretation, making them inherently brittle under iterative optimization and difficult to scale across heterogeneous tasks.

Their diagnostic evidence shows:
- **Vector prompt tuning continues to improve** with increasing supervision, while **text-based prompt optimization saturates early**.
- Vector prompts exhibit **dense, global attention patterns** — they function as persistent control signals addressable by all downstream tokens, not as transient natural language to be "read" and potentially forgotten.
- Text prompts are linguistically constrained (must be valid language), while vector prompts can express configurations in continuous space that no text string could represent.

This paper essentially argues that the text prompt interface is an artificial bottleneck — that models could be more precisely controlled if users could submit vectors directly alongside (or instead of) text.

### Instruction-Conditioned Embeddings

A parallel development: embedding models that take instructions as conditioning signals.

- **INSTRUCTOR** (Su et al., 2022): A single embedding model that generates different vector representations of the same text depending on a task instruction.
- **Gemini Embedding 2** (Google, March 2026): First natively multimodal embedding model — maps text, images, video, audio, and documents into a single vector space, conditioned on task prompts.
- **PonTE** (2025): Prompt-based conditional text embeddings from causal LLMs — the prompt *reshapes* the embedding space.

The pattern: embeddings are no longer fixed representations of content. They are **intent-conditioned projections** — the same content projected differently depending on what you're trying to do with it.

### Multimodal World Models

The frontier of representation learning has moved to **unified latent spaces** that fuse multiple modalities:

- **Emu3.5, GenRL, WMLM**: Shared attention backbones with modality-tagged tokens in a common latent space.
- **WAVE**: First model producing unified embeddings for text, audio, video, and audio-visual inputs.
- **Motus**: Unified latent action world model — encodes both perception *and* planned actions in the same representation space.

These systems don't just *encode* multimodal input — they reason, plan, and generate actions within the latent space. The latent representation is not a waypoint; it's the substrate of computation.

---

## The Theoretical Extrapolation

### Level 0: Where Brief Is Today

```
.brief.md  →  emitter  →  text (CLAUDE.md, XML, system prompt)  →  tokenizer  →  model
```

Human intent encoded as structured text. Model consumes text. Lossy at every boundary.

### Level 1: Compiled Soft Prompts

```
.brief.md  →  compiler  →  soft prompt vectors  →  model (prefix)
```

Brief's structured fields map to learned vector prefixes:
- **Goal** → 5-10 vectors encoding task direction
- **Hard constraints** → vectors trained to suppress specific behaviors (the NEVER/MUST signal, but in continuous space where it cannot be "forgotten" through attention decay)
- **Sacred regions** → vectors that encode file-path awareness and modification suppression (requires the model to have been trained with code-structural grounding)
- **Soft constraints** → vectors with weaker activation, influencing but not suppressing
- **Deliverable** → vectors encoding success criteria

The "compiler" would be a small neural network (a prompt encoder) trained on (brief, desired_behavior) pairs. You'd fine-tune it on examples: "given this brief, the model should do X and not do Y."

**What this buys you**:
- No context window cost. Soft prompts are prepended outside the token budget.
- No attention decay. Soft prompt vectors occupy the prefix position and receive global attention — they don't "fade" like instructions buried in a long system prompt.
- Constraint encoding is denser. The information in "NEVER modify src/auth/**" could be encoded in 2-3 vectors instead of ~15 tokens.

**What this costs you**:
- Model-specific. Soft prompts are not portable across models (PromptBridge shows 30%+ accuracy drops on transfer).
- Not interpretable. You cannot read a soft prompt to verify it encodes what you intended. You lose the human-readable artifact.
- Requires training infrastructure. Compiling a brief → vectors requires gradient descent, not string formatting.

### Level 2: Intent Embeddings in a Shared Space

```
.brief.md  →  intent encoder  →  intent vector  →  multimodal model (conditioning signal)
```

Instead of many soft prompt vectors, a single high-dimensional **intent embedding** encodes the entire brief. This intent vector conditions the model's behavior the way Gemini Embedding 2 conditions search — the same model, but projecting its computation through the lens of "what this brief says."

This requires an intent encoder trained to map structured briefs into the model's conditioning space. The brief's fields become regions of a learned manifold:
- Nearby intent vectors produce similar agent behavior.
- The constraint taxonomy (hard/soft/ask-first) maps to distances from decision boundaries in the intent space.
- Sacred regions are encoded as exclusion zones in the action-generation subspace.

**The key insight**: at this level, the brief is not "instructions the model reads." It's a **coordinate in intent space** that configures the model's computation. The difference is analogous to the difference between telling someone "go north" (text) versus physically rotating them to face north (vector).

### Level 3: Compiled Constraint Manifolds

```
.brief.md  →  constraint compiler  →  constraint manifold  →  model (latent space constraint)
```

At this level, brief doesn't encode *instructions* — it encodes **the shape of the permissible action space**.

Drawing on latent constraint research (Engel et al., 2017; recent LSC-GNN work), the brief would compile into a constraint manifold in the model's latent space — a geometric object that defines which outputs are valid and which are forbidden.

- **Hard constraints** define hard boundaries in the manifold. The model's generation process is geometrically unable to produce outputs that violate them — not because it was told not to, but because those outputs lie outside the reachable space.
- **Soft constraints** define attractor regions — the model's generation is pulled toward preferred behaviors without hard exclusion.
- **Sacred regions** map to **invariance constraints** — regions of the code representation that must remain fixed under the model's proposed transformations.

This is the theoretical maximum: **constraints that are not instructions but geometry**. The model doesn't "comply" with constraints because it was told to. It can't violate them because the computation is shaped to make violation unreachable.

**This is what enforcement looks like at the limit.** Brief's design-decisions.md describes a capability ladder: advisory text → CI enforcement → hook enforcement. The next rungs would be: soft prompt conditioning → intent space projection → constraint manifold shaping.

### Level 4: The Multimodal Unified Brief

```
.brief.md + codebase + architecture diagram + test results + conversation history
     ↓
  multimodal intent encoder
     ↓
  single unified intent vector in shared latent space
     ↓
  world model (plans, reasons, and acts within the latent space)
```

The maximum theoretical distance. Brief's `.brief.md` is one modality of intent. But the human's *actual* intent also includes:

- The codebase itself (the spatial/structural context)
- Architecture diagrams (visual modality)
- Test results (empirical constraints)
- Conversation history (evolving intent)
- The human's past behavior patterns (implicit preferences)

A natively multimodal world model (in the vein of Motus, Emu3.5) would fuse all of these into a unified latent representation. The `.brief.md` is the explicit, authored component of intent — but it's composed with all the implicit context into a single representation that the world model uses for planning and action generation.

At this level, the distinction between "prompt" and "context" and "constraint" dissolves. They're all projections into the same latent space. The brief is not a document the model reads — it's a **perturbation of the world model's state** that reshapes its planning manifold.

---

## Would It Look Like Embeddings?

Yes and no. Let me be precise about what "embeddings" means at each level:

### What it IS (probably)

The compiled brief would most likely take the form of **learned continuous vectors in the model's hidden dimension space** — i.e., the same mathematical object as soft prompts. This is because:

1. The model's attention mechanism is the only interface through which external signals can influence computation. Vectors in the hidden dimension space are the native currency of that interface.
2. All information that enters a transformer must eventually become vectors in this space. Text gets there via tokenization + embedding lookup. Images get there via vision encoders. A compiled brief would get there via a learned encoder.
3. The vector representation allows continuous interpolation — you could smoothly vary the "strength" of a constraint by moving along a dimension, which is impossible with discrete text.

### What it IS NOT (probably)

The compiled brief would probably **not** look like conventional embedding vectors (e.g., sentence embeddings from OpenAI's text-embedding-3 or Gemini Embedding 2). Those are designed for **similarity search** — they encode semantic proximity. A compiled brief needs to encode **behavioral directives** — it needs to change what the model *does*, not describe what something *means*.

The distinction:
- **Retrieval embedding**: encodes "what is this about?" → similarity in meaning space
- **Compiled brief**: encodes "what should you do and not do?" → configuration of action space

These are different manifolds with different training objectives. A retrieval embedding of "do not modify src/auth" is close to other sentences about auth modification. A compiled brief vector for that constraint needs to be close to *the computational pattern that suppresses auth modifications* — a fundamentally different kind of proximity.

---

## The Portability Problem

The deepest theoretical challenge: **soft prompts are model-specific**. A compiled brief for Claude would not work for GPT, and might not even transfer between Claude model versions.

This is brief's central tension taken to its limit. Today, `brief emit claude` produces Claude-optimized text and `brief emit prompt` produces generic text. At the compiled level, you'd need `brief compile claude-opus-4-6` and `brief compile gpt-5` — different vector sets for different models, trained on different objectives.

Research on cross-model prompt transfer (PromptBridge, CrossPT) shows 30%+ accuracy drops on naive transfer, with partial recovery through learned projection networks. The theoretical resolution:

1. **Model-specific compilers**: Brief maintains a compiler per model family, trained to map structured intent → that model's vector space. This is the pragmatic path.
2. **Universal intent space**: A model-agnostic intent representation that each model has a learned adapter for. Analogous to how LLVM IR is architecture-independent but compiled to architecture-specific machine code. No evidence this is achievable yet.
3. **Convergent representations**: As models get larger, their internal representations may converge (the "platonic representation hypothesis"). If frontier models converge on similar latent geometries, compiled briefs might become approximately portable. Speculative but not impossible.

---

## What This Means for Brief (Practically)

None of this is buildable today as a CLI tool. But it reshapes how to think about brief's architecture:

### The `.brief.md` format is the right abstraction layer

The structured fields (goal, constraints with taxonomy, sacred regions, assumptions, deliverable) are not just convenient authoring affordances — they're the **semantic primitives** that any compilation target would need. Whether you emit to XML text, soft prompt vectors, or constraint manifolds, you need to know: what's the goal? What's forbidden? What's preferred? What must not change?

The `.brief.md` format is an **intermediate representation** in the compiler sense. It's closer to LLVM IR than to a source language. It's structured enough to compile to any target, human-readable enough to author directly.

### The emitter architecture anticipates this

Brief's multi-target emit design (`claude`, `prompt`, `xml`, `json`, `skill`) is already the right pattern. A hypothetical `brief compile soft-prompt --model claude-opus` would be another emit target — one that outputs a binary vector file instead of a text string, but consuming the same `&Brief` struct.

### The Phase 2 constraint reframing is an incremental step toward compilation

Reframing "Hard" as "NEVER/MUST" is moving brief's output from *human categorization language* toward *model-effective behavioral signals*. This is the same direction as compilation — making the emitted representation more aligned with how the model actually processes behavioral directives. Text reframing is a 10% step toward compiled vectors that encode behavioral directives directly.

### The enforcement ladder extends naturally

```
Advisory text (Phase 1)
  → CI enforcement (Phase 2)
    → Hook enforcement (Phase 2)
      → Soft prompt conditioning (theoretical)
        → Intent space projection (theoretical)
          → Constraint manifold shaping (theoretical limit)
```

Each step makes constraint compliance less probabilistic and more deterministic. The text-to-vector boundary is where compliance stops being "the model was told" and starts being "the model was configured."

---

## The Philosophical Endpoint

At the maximum theoretical distance, brief dissolves into something that doesn't have a name yet. It's not a prompt. It's not a program. It's not a configuration file. It's a **shape in intent space** — a geometric object that defines what an agent should do, encoded in the native representation of the computational substrate that will execute it.

The `.brief.md` file — the Markdown with YAML frontmatter that a human writes in 60 seconds — is the source language. The compiled output is a point (or region, or manifold) in a high-dimensional space that no human can read but that perfectly encodes the human's intent in a form the model can execute without interpretation loss.

The irony: the maximum theoretical endpoint of machine learning applied to human→agent communication is to make the communication *invisible*. The human writes a brief. The machine executes intent. The intermediate representation — whether text, XML, vectors, or manifold geometry — is an implementation detail.

Brief's bet is that the structured intermediate representation — the `.brief.md` — will remain the right authoring interface regardless of what the compilation target becomes. And the research suggests this bet is correct: every approach from soft prompts to world models needs the same semantic primitives that brief already captures. The format is the product; the targets are plugins. That design decision scales all the way to the theoretical limit.

---

## Sources

- [Position: Vector Prompt Interfaces Should Be Exposed (arxiv 2603.04292)](https://arxiv.org/abs/2603.04292)
- [Prefix-Tuning: Optimizing Continuous Prompts for Generation (arxiv 2101.00190)](https://arxiv.org/abs/2101.00190)
- [Does Prompt Formatting Have Any Impact on LLM Performance? (arxiv 2411.10541)](https://arxiv.org/abs/2411.10541)
- [Gemini Embedding 2 — Google's First Natively Multimodal Embedding Model](https://blog.google/innovation-and-ai/models-and-research/gemini-models/gemini-embedding-2/)
- [Amazon Nova Multimodal Embeddings](https://aws.amazon.com/blogs/aws/amazon-nova-multimodal-embeddings-now-available-in-amazon-bedrock/)
- [WAVE: Unified Audio-Visual Embeddings](https://openreview.net/pdf?id=MiV3WXDYJb)
- [Soft Prompt Methods in Large Models — Emergent Mind](https://www.emergentmind.com/topics/soft-prompt-methods)
- [PromptBridge: Cross-Model Prompt Transfer (arxiv 2512.01420)](https://arxiv.org/pdf/2512.01420)
- [CrossPT: Cross-Task Transferability through Multi-Task Prompt Tuning](https://arxiv.org/html/2509.14253)
- [Latent Constraints: Learning to Generate Conditionally (arxiv 1711.05772)](https://arxiv.org/abs/1711.05772)
- [Soft Injection of Task Embeddings Outperforms Prompt-Based ICL (arxiv 2507.20906)](https://arxiv.org/html/2507.20906v1)
- [Embedding-to-Prefix: Parameter-Efficient Personalization (arxiv 2505.17051)](https://arxiv.org/html/2505.17051v1)
- [Survey on Prompt Tuning (arxiv 2507.06085)](https://arxiv.org/html/2507.06085v2)
- [Understanding World or Predicting Future? Survey of World Models (ACM CSUR 2025)](https://dl.acm.org/doi/10.1145/3746449)
- [Intent Embed: Capturing User Intention in Dense Vector Representations](https://medium.com/@plbiojout/intent-embed-capturing-user-intention-in-dense-vector-representations-for-enhanced-llm-interaction-f8271cd61871)
- [INSTRUCTOR: One Embedder, Any Task (arxiv 2212.09741)](https://arxiv.org/abs/2212.09741)
- [Generative Modelling in Latent Space — Sander Dieleman (2025)](https://sander.ai/2025/04/15/latents.html)
- [Embodied Intelligence: Multimodal Perception, World Modeling, and Structured Strategies (Frontiers 2025)](https://www.frontiersin.org/journals/robotics-and-ai/articles/10.3389/frobt.2025.1668910/full)
