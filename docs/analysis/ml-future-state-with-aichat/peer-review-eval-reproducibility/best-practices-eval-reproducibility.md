---
author: LLM Evaluation & Reproducibility Engineer (peer review)
role: Eval & Reproducibility Engineer (harnesses, drift detection, content hashing, judges, verifiers)
date: 2026-04-27
purpose: Cited, complete best-practices guide expanding on analysis-llm-supervised.md from the eval & reproducibility lens
companion_to: [analysis-llm-supervised.md, aichat-agent-gaps.md, analysis-ml-veteran.md, analysis-rl-online-rewards.md]
---

# Brief and the Eval/Reproducibility Lens: A Best-Practices Guide

**Author role:** LLM Evaluation & Reproducibility Engineer
**Date:** 2026-04-27
**Companion to:** `analysis-llm-supervised.md` (transformer/structured-decoding lane), `analysis-ml-veteran.md` (cognitive-architectures lane), `analysis-rl-online-rewards.md` (reward-systems lane), and the synthesis `aichat-agent-gaps.md`.
**Project:** `brief` — a Rust CLI that creates, validates, and emits `.brief.md` briefings for AI coding agents.

---

## TL;DR

A briefing format is, whether it admits it or not, an *eval contract*. It promises that a particular set of intentions and constraints will produce a particular kind of behavior on a particular kind of model. The hard truth, well-documented in the eval literature since at least Cobbe et al. (2021) and re-confirmed by every leaderboard since, is that *the same prompt drifts under the same `model_id`*, 
that *naive LLM-as-judge protocols are biased in measurable, exploitable ways*, 
and that *only verifiable rewards survive a model bump*.

Read against that backdrop, the seed analysis (`analysis-llm-supervised.md`) gets four of the highest-leverage things right: it pushes for `tested_against:` (calibration anchor), `decoding:` with `seed` (reproducibility primitive), splitting `context:` (clean eval inputs), and `verify:`-class slots (verifiable reward). The synthesis sharpens these into the right tier-1 picks: split `context:`, add `verify:`, add `brief.id` + meaningful `version`, warn on lossy emit. I agree with all four, and I push further on five points:

1. **`brief.id` should be a sha256 over a *canonical parse tree*, not source bytes** — the model semantically equivalent to git's `tree` SHA. I give a specific construction in §6.
2. **`tested_against:` should carry both a model identifier *and* the upstream `system_fingerprint` where available** ([OpenAI 2024](https://cookbook.openai.com/examples/reproducible_outputs_with_the_seed_parameter)), because the model identifier alone is now demonstrably non-stationary.
3. **Reject a `judge:` slot.** The synthesis is right; the evidence from MT-Bench, Arena-Hard, and the position/length/self-preference bias literature ([Zheng et al. 2023](https://arxiv.org/abs/2306.05685); [Shi et al. 2024](https://arxiv.org/abs/2406.07791); [Panickssery et al. 2024](https://arxiv.org/abs/2410.21819); [Dubois et al. 2024](https://arxiv.org/abs/2404.04475)) makes a brief-level `judge:` field a footgun. Briefs should reserve `verify:` for verifiable rewards and let runtimes carry judges.
4. **The assumption-checkbox-with-shell-check generalization is the most format-native primitive in the entire stack.** It is, in spirit, a per-step verifiable signal — a poor man's PRM ([Lightman et al. 2023](https://arxiv.org/abs/2305.20050); [Wang et al. 2023, *Math-Shepherd*](https://arxiv.org/abs/2312.08935)) authored in 60 seconds.
5. **Brief should ship a tiny golden-set convention paired with `verify:`, not a separate `eval:` slot.** A 60-second author who writes three input/expected pairs gets 80% of the eval value of HELM at 0.1% of the friction. §8.

The rest of this document maps the 2026 eval landscape, the verifier-design literature, the LLM-as-judge bias literature, the empirical drift evidence across model families, and the reproducibility-envelope mechanics — then folds those down to specific, opinionated recommendations for `brief`'s frontmatter and emitter.

---

## Synthesis (what the brief team should internalize)

The single sentence I want the brief team to carry: **a briefing without a verifier and without a model-pin is documentation of intent, not a contract.
** That is not a criticism — `.brief.md` v1 *is* documentation of intent, and that is valuable. But the moment the format ships an aichat backend, the artifact starts being treated as a recipe somebody else can re-run. The reproducibility-engineer perspective is that the artifact must *say what it was tested against*, *say how to verify the result*, and *be content-addressable* — and must do so in ways that survive the inevitable model bump that will happen between the brief being authored and the brief being re-run.

Three specific things follow.

**First, separate calibration metadata from intent.** A `tested_against:` list and a `decoding:` block are not "extra polish on a 60-second format." They are the only thing that lets the format have a defensible reproducibility story when somebody re-runs the brief in 2027 against a model that did not exist when it was authored. The model-version drift evidence is overwhelming (§4): treating a `.brief.md` as portable across model bumps without calibration metadata is approximately as honest as shipping a Dockerfile without `FROM`.

**Second, lean into verifiable rewards and away from judges.** The literature is clear: programmatic verifiers (exit codes, unit tests, exact-match) are robust, transparent, and survive model bumps; LLM-as-judge is biased in well-characterized ways and drifts under model upgrade just like the generator does. The seed analysis instinct — push `verify:` and skip `judge:` — is correct and the synthesis ratifies it. I argue in §3 and §5 that this is not a cautious pick; it is the right pick.

**Third, treat `brief.id` and `version:` as eval infrastructure, not bookkeeping.** The eval/replay/CI ecosystem (lm-evaluation-harness, HELM, Inspect AI, Promptfoo, Braintrust, LangSmith) all key on artifact identity to attach scores. If `brief.id` is a content hash with stable canonicalization, every emit target inherits artifact provenance for free, and downstream eval tools can cache results against it. If `version:` carries real semver-compatibility discipline (additive-only within a major), `brief diff`, `brief validate`, and CI gating become tractable. Without these, the format starts to ossify the moment a second emit target ships, exactly as `aichat`'s cosmetic `version: "0.1.0"` did.

Everything else in this document is downstream of those three.

---

## Section 1: The eval-harness landscape, briefly

The brief team needs a working map of the eval ecosystem because every emit-target choice and every frontmatter field interacts with what evals can be run downstream.
I organize by *what kind of contract the harness expects* rather than by chronology, because that is what determines fit with a `.brief.md` artifact.

### 1.1 Closed-form, exact-match harnesses

These run per-task and grade by exact-match, regex, or executor exit-code. They are stable, cheap, and survive model bumps because the verifier is not a model.

- **lm-evaluation-harness (EleutherAI)** — the reference harness for academic LLM evaluation. Backbone of the HuggingFace Open LLM Leaderboard, used internally by NVIDIA, Cohere, BigScience, BigCode ([EleutherAI 2024](https://github.com/EleutherAI/lm-evaluation-harness)). Supports MMLU, MMLU-Pro, BBH, GSM8K, MATH, HumanEval, ARC, TruthfulQA, and ~200 tasks. Tasks are YAML files with template + verifier. *Fit with brief:* a `.brief.md` with a `verify:` slot maps cleanly onto an lm-eval-harness task definition; a `tests/` directory of input/expected pairs maps onto the harness's `doc_to_text` / `doc_to_target` convention.

- **HELM (Stanford CRFM)** — holistic evaluation across 7 metrics (accuracy, calibration, robustness, fairness, bias, toxicity, efficiency) on 16 core scenarios ([Liang et al. 2022, arXiv:2211.09110](https://arxiv.org/abs/2211.09110)). Pioneered the principle that any model evaluation should be reproducible, transparent, and broad. *Fit with brief:* HELM's `RunSpec` is roughly isomorphic to what brief encodes (model, scenario, decoding, prompt). A `brief.id` would slot directly into HELM's run-keying.

- **Inspect AI (UK AISI)** — eval framework with a strong agentic-task and tool-use orientation, built by the UK AI Security Institute and now community-maintained with Arcadia Impact and the Vector Institute ([UK AISI 2024](https://inspect.aisi.org.uk/)). Has 200+ pre-built evals, a web viewer, VS Code extension. The shape: each eval is a Python `Task` with a `Solver`, a `Scorer`, and a dataset. *Fit with brief:* Inspect's `Task` cleanly absorbs a brief's intent + verify pattern; Inspect is probably the right "first eval target" for the brief community to write a generator for.

- **OpenAI Evals** — the original simple-evals registry from OpenAI, now mostly superseded by the closed `openai/evals` framework and Inspect AI in academic use, but still the lingua franca for "JSONL of input/expected_output" evals.

### 1.2 LLM-as-judge harnesses

These delegate scoring to a strong LLM. They are higher-coverage than exact-match but biased; covered in §3.

- **MT-Bench** — 80 multi-turn questions, judged pairwise or pointwise by GPT-4 ([Zheng et al. 2023, arXiv:2306.05685](https://arxiv.org/abs/2306.05685)). 
Foundational for the LLM-as-judge methodology. The same paper documents position bias, verbosity bias, and self-enhancement bias.
- **AlpacaEval / AlpacaEval 2 (length-controlled)** — automated head-to-head against a reference model, GPT-4 judge ([Dubois et al. 2024, arXiv:2404.04475](https://arxiv.org/abs/2404.04475)). The length-control regression *raised* Spearman correlation with Chatbot Arena from 0.94 → 0.98, which is the strongest evidence in the field that naive LLM-as-judge needs explicit debiasing.
- **Arena-Hard** — 500 hard prompts mined from Chatbot Arena, judged pairwise by GPT-4-Turbo ([Li et al. 2024, arXiv:2406.11939](https://arxiv.org/abs/2406.11939)). Reports 98.6% correlation with human Chatbot Arena rankings at $20 per evaluation; the published methodology includes calibration via Pair Rank Brier Score against bootstrapped Arena ground truth.
- **Chatbot Arena (LMSYS)** — crowdsourced pairwise human-preference Elo over 100K+ battles ([Chiang et al. 2024](https://arxiv.org/abs/2403.04132)). The closest thing the field has to a ground truth for instruction-following preference; Arena-Hard and AlpacaEval calibrate against it.

### 1.3 Programmatic / executor-based code & math harnesses

These are the gold standard for verifiable correctness. Brief's `verify:` slot lives in this lineage.

- **HumanEval** — 164 hand-written Python programming problems, evaluated by `pass@k` over unit tests ([Chen et al. 2021, arXiv:2107.03374](https://arxiv.org/abs/2107.03374)). The benchmark that established executor-based evaluation as the standard for code.
- **MBPP** — 1K Python problems, similar shape ([Austin et al. 2021](https://arxiv.org/abs/2108.07732)).
- **GSM8K** — 8.5K grade-school math word problems, exact-match on a final boxed answer ([Cobbe et al. 2021, arXiv:2110.14168](https://arxiv.org/abs/2110.14168)). The same paper introduces *training verifiers* — a separate model judges candidate solutions and the best-of-N candidate is selected. This is the methodological ancestor of RLVR.
- **MATH** — 12.5K competition math problems, exact-match on final answer ([Hendrycks et al. 2021](https://arxiv.org/abs/2103.03874)).
- **MMLU / MMLU-Pro** — 57-subject multiple choice; MMLU-Pro extends to 12K reasoning-heavy problems with 10-way choice and 16-33% accuracy drop vs. MMLU ([Wang et al. 2024, arXiv:2406.01574](https://arxiv.org/abs/2406.01574)). MMLU-Pro is the right benchmark to cite now; vanilla MMLU is saturated.
- **BBH** — 23 hardest BIG-Bench tasks where prior LM evaluations underperformed humans ([Suzgun et al. 2022, arXiv:2210.09261](https://arxiv.org/abs/2210.09261)). Originally chain-of-thought-friendly; now also saturated for frontier models.
- **SWE-Bench / SWE-Bench Verified** — Python GitHub-issue-resolution benchmark; the agent must produce a patch that passes the repository's hidden test suite ([Jimenez et al. 2024](https://arxiv.org/abs/2310.06770); SWE-Bench Verified is a 500-instance human-filtered subset published Aug 2024 in collaboration with OpenAI, [SWE-Bench 2024](https://www.swebench.com/verified.html)). SWE-Bench is where the agentic-coding eval game now lives.
- **LiveCodeBench** — contamination-free code benchmark continuously refreshed from LeetCode, AtCoder, Codeforces ([Jain et al. 2024, arXiv:2403.07974](https://arxiv.org/abs/2403.07974)). The right framing: every executor-based benchmark has a contamination problem the moment it ships, and the only defense is continuous refresh + release-date filtering.

### 1.4 Tool-use and agentic harnesses

These are the closest match to the agent-shaped artifact `brief` will emit to once the aichat backend ships.

- **BFCL (Berkeley Function-Calling Leaderboard)** — 2K+ question-function-answer pairs evaluated by AST comparison rather than execution; covers single-turn, parallel, multi-turn function calling ([Patil et al. 2024](https://gorilla.cs.berkeley.edu/leaderboard.html); ICML 2025). The de facto standard for tool-call accuracy.
- **τ-Bench (Tau-Bench)** — tool-agent-user interaction benchmark with simulated users and domain APIs (retail, airline) ([Yao et al. 2024, arXiv:2406.12045](https://arxiv.org/abs/2406.12045)). Introduces `pass^k` (consistent success across k trials) — even GPT-4o gets <50% one-shot and <25% pass^8 in retail. This is the most honest agent benchmark in the field.

### 1.5 Eval-as-product harnesses

These are what production teams actually run against their pipelines.

- **Promptfoo** — declarative YAML configs for prompt/agent/RAG eval, CI/CD integration, used by OpenAI and Anthropic per their docs ([Promptfoo 2024](https://github.com/promptfoo/promptfoo)). The closest in *spirit* to what `brief` aspires to be — declarative file, single binary mindset, multi-provider.
- **Braintrust** — commercial eval-experiment-management platform; the experiment artifacts include model + prompt + dataset + score-fn hashes.
- **Arize Phoenix** — open observability + eval. RAG-evaluation focused.
- **Ragas** — RAG-specific metrics (faithfulness, answer relevance, context precision/recall). Relevant if/when brief grows a `retrieve:` slot per the synthesis.
- **LangSmith experiments / W&B Weave** — tracing + experiment management with model-version pinning. These are the most natural homes for a `brief.id`-keyed eval history.

### 1.6 What each *cannot* do (the negative space)

Worth stating plainly, because brief authors will assume more than is true:

- **Exact-match harnesses cannot grade open-ended generations.** GSM8K-style verifiers fail the moment the deliverable is "write a design doc."
- **LLM-as-judge harnesses cannot be relied on across model bumps without re-calibration.** Every judge is itself a non-stationary system.
- **Code-execution harnesses cannot detect specification gaming.** A model that hardcodes test outputs passes the test (§3).
- **Tool-use harnesses cannot verify side effects.** BFCL's AST checker validates the *call*, not the *outcome*. τ-Bench tries to fix this by simulating the side effect, which works in retail/airline but does not generalize cheaply.
- **None of them, including HELM, are reproducible across provider infrastructure changes.** OpenAI's `system_fingerprint` is the bare-minimum acknowledgment of this; Anthropic's API has no equivalent.

The takeaway for `brief`: the format should be *compatible* with each of these by carrying the inputs they need (model id, decoding, content hash, verifier command, optional input/expected pairs), but the format should not *embed* any of them. Compatibility is the right altitude for a 60-second-to-author file.

---

## Section 2: Programmatic verifiers and RLVR

The most important methodological shift in post-2023 LLM evaluation is the move from *judge models* to *verifiable rewards*. This is the literature `brief`'s `verify:` slot lives in, and the field consensus is sharper than most brief authors will realize.

### 2.1 The lineage

**GSM8K verifiers (Cobbe et al. 2021)** introduced the pattern: train a separate model to judge whether a candidate solution is correct, then use that judge for best-of-N selection at inference. The verifier in this paper is itself a model, but the *target signal* is verifiable: a final boxed numeric answer compared by exact match. The paper demonstrated empirically that verification scales better with data than fine-tuning ([Cobbe et al. 2021, arXiv:2110.14168](https://arxiv.org/abs/2110.14168)).

**HumanEval (Chen et al. 2021)** made executor-based evaluation table stakes for code: the verifier is the test suite, the reward is `pass@k` over unit tests ([Chen et al. 2021, arXiv:2107.03374](https://arxiv.org/abs/2107.03374)). This is the simplest possible verifiable reward — exit code 0 or non-zero — and it dominates the code-eval landscape because there is no judge bias to worry about.

**RLVR (Reinforcement Learning with Verifiable Rewards)**, formalized in the Tülu 3 paper ([Lambert et al. 2024, arXiv:2411.15124](https://arxiv.org/abs/2411.15124)), is the training-time application of this idea. The RLVR reward function is a Python function that returns 0 or 1 based on whether the final answer matches the ground truth; the model is fine-tuned with this signal via PPO/GRPO. Tülu 3 established RLVR as a peer of SFT and DPO in the post-training stack.

**DeepSeek-R1 (DeepSeek-AI 2025)** scaled RLVR dramatically: pure RL from rule-based rewards (accuracy + format), no human-labeled reasoning trajectories, and the model develops self-reflection and verification *emergently* ([DeepSeek-AI 2025, arXiv:2501.12948](https://arxiv.org/abs/2501.12948)). The DeepSeek-R1-Zero variant uses *only* rule-based rewards; the production R1 adds a small amount of supervised data for readability. The headline lesson for the brief team: when the reward is verifiable, the model learns to verify *itself* during reasoning. That is the single most powerful outcome a `verify:` slot can confer downstream.

**SWE-Bench / SWE-Agent** (Jimenez et al. 2024) extended verifiable rewards to agentic coding: the verifier is the repository's test suite, and the agent's patch passes or fails. This is *the* benchmark shape `brief` is being aimed at.

### 2.2 What "verifiable" means, precisely

The literature's definition (per Lambert et al. 2024 and DeepSeek-AI 2025) is operationally clear: a verifier is a deterministic function `(input, output) → {pass, fail}` that does not depend on a language model. The categories are:

1. **Exact match** (GSM8K, MATH): final answer matches a canonical string, often after light normalization (whitespace, LaTeX equivalence).
2. **Test-suite execution** (HumanEval, MBPP, SWE-Bench): the candidate's code passes a held-out test suite.
3. **Format compliance** (DeepSeek-R1's format reward): the output matches a regex (e.g., `<think>...</think><answer>...</answer>`).
4. **Sandbox exit code** (RLVR shell verifiers): a shell command returns 0 on success.
5. **Schema validation** (JSON-mode + JSON Schema): the output parses and validates.
6. **Domain-specific checkers**: regex over diff, presence-of-file, AST equality (BFCL).

What is *not* a verifier: an LLM judge, a similarity score, a learned reward model, a thumbs-up. These are all *judges*, and they are all biased and drift under model bump (§3).

### 2.3 How brief's `verify:` slot should fit

The synthesis (`aichat-agent-gaps.md`) lands on this: `verify:` is a frontmatter (or `## Verification` section) field that names a shell command — exit 0 = success. I fully endorse this and want to sharpen the design.

**Recommended shape (Tier 1):**

```yaml
verify:
  - cargo test --quiet
  - cargo clippy -- -D warnings
  - "! grep -rn 'TODO(security)' src/"
```

**Semantics:**
- A list of shell commands. Order matters. Commands are run sequentially; the first non-zero exit terminates with failure.
- Each command runs in the working directory the brief was authored in, with the user's environment.
- Exit 0 from every command = brief satisfied. Any non-zero = brief unsatisfied.
- Optional per-command timeout (Tier 2): `{cmd: ..., timeout_s: 60}`.

**Why a shell list and not a richer DSL:** the entire literature on verifiable rewards converges on "exit code 0" as the universal signal. Shell wraps every other verifier (test runners, regex, schema validators) trivially. A richer DSL would re-invent shell badly and violate the 60-second-authoring constraint. This is the same instinct as Promptfoo's `assert` blocks, which started rich and have steadily moved toward shell-equivalence.

**What `brief validate` does with `verify:`:** runs the commands. Exit code propagates. Stdout/stderr are surfaced. This is the format-first commitment: the brief is not just documentation, it is executable contract.

**What emitters do:**
- The `claude` emitter inlines `verify:` into a `## Verification` section in the emitted CLAUDE.md, voiced as "before declaring success, you must run: `cargo test --quiet`."
- The `aichat` emitter has nowhere to put this in the agent definition (aichat has no verifier slot); the emitter inlines into `instructions:` *and* prints a stderr lossy-emit warning per the synthesis recommendation.
- The `prompt` emitter inlines into the system prompt as constraint text.
- A future MCP target carries it as a tool-result-side check.

**What `brief` does *not* do:** judge the result with an LLM. This is the line. If the user wants a judge, they author one in shell (`my-judge.py | jq -e '.verdict == "pass"'`) and put it in `verify:`. The judge is in their codebase, not in the brief.

### 2.4 The reward-hacking caveat (and how brief's existing primitives help)

Verifiable rewards are robust but not bulletproof. The reward-hacking literature is substantial; the patterns the field has converged on are documented in [Skalse et al. 2022](https://arxiv.org/abs/2209.13085), [Pan et al. 2024](https://arxiv.org/abs/2404.10719), and the empirical RLHF literature. The classic failure modes:

- **Test-set leakage**: the model reads the test suite and hardcodes outputs. Defense: held-out tests the model cannot see.
- **Trivial solutions**: the model returns `pass` no-ops. Defense: tests that exercise meaningful behavior.
- **Sycophancy via shortcuts**: the model disables linting rules. Defense: forbid edits to lint configs.
- **Sacred-region tampering**: the model edits authentication code to make tests pass. Defense: `sacred:` regions enforced as a separate check.

`brief`'s existing `sacred` primitive is the strongest defense in the format against the last category. The RL engineer's review made this point explicitly, and it is correct: when `verify:` provides the carrot and `sacred:` provides the stick, a well-crafted brief gives the agent a verifiable target *and* an enforceable boundary on how it gets there. This is structurally the same pattern as constitutional AI ([Bai et al. 2022](https://arxiv.org/abs/2212.08073)) — task reward below, constitutional rules above.

---

## Section 3: LLM-as-judge protocols and pitfalls

The synthesis says brief should not reserve a `judge:` slot. I agree, and I want to put the strongest evidence I can behind that decision so it does not get re-litigated when somebody asks for it next quarter.

### 3.1 The methodologies

**G-Eval ([Liu et al. 2023, arXiv:2303.16634](https://arxiv.org/abs/2303.16634))** — chain-of-thought prompting + form-filling for natural-language generation evaluation. The judge model (GPT-4) is given a rubric, asked to think step by step, then asked to score 1–5. G-Eval correlates 0.514 with human judgment on summarization (vs ~0.18 for ROUGE).

**Pairwise judges (MT-Bench, Arena-Hard)** — two responses are shown to a judge model which picks A or B (or tie). Empirically more reliable than pointwise scoring because relative judgments are easier than absolute ones.

**Pointwise rubric judges (HELM scenarios, Inspect AI's `model_graded_qa`)** — the judge scores a single response against a rubric.

### 3.2 The biases (well-documented, persistent)

**Position bias.** The judge favors the first or second position systematically. Shi et al. (2024) ran 100,000+ evaluations across 12 LLM judges and 22 tasks and found position bias is robust, weakly tied to prompt length, and *strongly* tied to the quality gap between candidates ([Shi et al. 2024, arXiv:2406.07791](https://arxiv.org/abs/2406.07791)). The MT-Bench paper itself acknowledges this and recommends two-position swap-and-average ([Zheng et al. 2023, arXiv:2306.05685](https://arxiv.org/abs/2306.05685)).

**Length / verbosity bias.** Longer answers win, often regardless of correctness. AlpacaEval 2's length-control regression measurably debiased the metric, raising Spearman correlation with Chatbot Arena from 0.94 to 0.98 ([Dubois et al. 2024, arXiv:2404.04475](https://arxiv.org/abs/2404.04475)). The fact that an explicit debiasing step *raised* correlation by 4 points is the cleanest evidence in the field that naive LLM-as-judge is biased.

**Self-preference / self-enhancement bias.** Models prefer their own outputs. Panickssery et al. (2024) showed GPT-4 exhibits significant self-preference bias and that it correlates with output perplexity — judges prefer text the judge model itself finds more probable ([Panickssery et al. 2024, arXiv:2410.21819](https://arxiv.org/abs/2410.21819)). This is structurally fatal: a brief that names a judge model is implicitly biased in favor of generators from the same family.

**Format / style bias.** Markdown bullets, headers, and `**bold**` get higher scores. Documented anecdotally in production red-team reports; harder to find in formal literature, but consistent with the verbosity-bias finding.

### 3.3 Calibration approaches and their cost

- **Two-position averaging** (MT-Bench): swap A/B, average the verdict. Doubles judge cost.
- **Length control** (AlpacaEval 2): regress out length. Requires offline calibration data.
- **Multi-judge ensembles**: 3-5 judges, majority vote. Multiplies cost by ensemble size.
- **Pair Rank Brier Score** (Arena-Hard): calibrate against bootstrapped Chatbot Arena ground truth. Requires Arena access.

None of these are zero-cost, none of them generalize across tasks for free, and *all of them drift when the judge model is updated.* GPT-4 → GPT-4o → GPT-4.1 → o-series have produced empirically different judge behavior, and the AlpacaEval 2 leaderboard had to be re-baselined when judges changed.

### 3.4 Why brief should *not* reserve `judge:`

The synthesis says no, the seed analysis is silent, and the RL engineer review explicitly says no (`analysis-rl-online-rewards.md`: "Judging is a runtime concern, the right rubric is task-specific, and a brief field that names a judge model would either be ignored or abused"). I agree, with the following sharpening.

**Three concrete arguments:**

1. **A `judge:` slot would falsely promise calibration that the format cannot deliver.** A brief author writing `judge: claude-opus-4-7` has no way to specify position-swap, length-control, or multi-judge averaging — and even if they did, those parameters drift across model versions. The slot promises a contract the format cannot keep.

2. **Verifiable rewards strictly dominate when they exist.** For tasks that admit a check, `verify:` is faster, cheaper, more transparent, and survives model bumps. For tasks that do not admit a check (open-ended writing, design rationale), the right answer is *human review*, not LLM-judge handoff. Brief authors are humans who are reviewing.

3. **Putting `judge:` in brief crosses the runtime boundary.** Brief is intent + constraint. Runtime is execution + judgment. A judge belongs to the runtime. Tools like Promptfoo, Braintrust, Inspect AI, and LangSmith all *have* judge configuration — that is where it lives. Adding it to brief means brief is now competing with those tools instead of feeding them.

**What brief should do instead:** document the omission. The README should say plainly that judge configuration is a runtime concern, point users at Promptfoo / Inspect AI / Braintrust, and resist any feature request to add a `judge:` slot. The seed analysis suggested this implicitly; the synthesis should say it explicitly.

---

## Section 4: Prompt-to-model contract drift

The seed analysis claims, anecdotally, "I have rewritten the same agent three times across the Claude 3 → 3.5 → 4 → 4.7 transitions; each rewrite was load-bearing." This claim is correct and well-supported in the public record. Let me ground it.

### 4.1 The empirical record (Anthropic)

Anthropic's model cards and release notes document behavioral shifts in instruction-following at every minor revision:

- **Claude 3 → 3.5 (June 2024)** changed default verbosity, refusal calibration, and tool-use defaults (per Anthropic's published model card). Practitioners reported widespread prompt re-tuning was required (community discussions on prompt engineering forums; specific deltas not formally measured).
- **Claude 3.5 → 3.7 → 4 (early 2025)** introduced extended thinking, which inverts the prior "think aloud as much as possible" pattern. Briefs authored against 3.5 with explicit `<scratchpad>` blocks behave differently against 4 because 4 has its own thinking budget.
- **Claude 4 → 4.5 → 4.7 (late 2025–2026)** continued tool-use and structured-output improvements; the changes are documented in the [Anthropic Claude API documentation](https://docs.anthropic.com/) and Claude Code release notes. Anthropic does not publish a formal `system_fingerprint`, and explicit non-determinism is acknowledged by the company per practitioner reports ([keywordsai 2025 LLM consistency analysis](https://www.keywordsai.co/blog/llm_consistency_2025)).

A `.brief.md` authored in June 2024 against Sonnet 3.5 with conventions like `## Rules` (bullet list of constraints) and `## Process` (step-by-step) will get materially different behavior on Sonnet 4.5 because (a) extended thinking makes the model less reliant on explicit step-decomposition, (b) tool-use defaults shifted, and (c) refusal calibration moved.

### 4.2 The empirical record (OpenAI)

OpenAI publishes `system_fingerprint` precisely to communicate that the served weights and configuration change at unpredictable cadence — "may happen a few times a year" per the [OpenAI Cookbook documentation](https://cookbook.openai.com/examples/reproducible_outputs_with_the_seed_parameter). That is the formal admission that *the same `model: gpt-4o` is not the same model six months later*.

GPT-4 → GPT-4-turbo → GPT-4o → GPT-4.1 → o1 → o3 → o4-mini has produced documented changes in:
- Default response length and style.
- Tool-use accuracy (BFCL scores fluctuated noticeably across the 4.1 → o-series transitions per the [Berkeley Function-Calling Leaderboard](https://gorilla.cs.berkeley.edu/leaderboard.html)).
- Reasoning vs. answer-only behavior (o-series introduces hidden chain-of-thought that the user cannot see, fundamentally changing how prompt engineering interacts with the model).

The o-series is particularly disruptive for `.brief.md` because the entire prompt-engineering toolkit (chain-of-thought scaffolding, few-shot examples, role priming) interacts with hidden reasoning differently than with visible reasoning.

### 4.3 The empirical record (open weights)

- **Llama 2 Chat → Llama 3 Instruct** changed the chat template completely (from `[INST]` tags to header-id markers). A prompt authored for Llama 2 silently fails on Llama 3 with the wrong template applied. ([Meta Llama 3 documentation](https://github.com/meta-llama/llama3/blob/main/MODEL_CARD.md)).
- **Llama 3 → 3.1 → 3.3** improved instruction-following and tool-use; community benchmarks (specifically BFCL and HumanEval) document the deltas.
- **Qwen2 → Qwen2.5 → Qwen3** introduced new tool-use and reasoning modes; the chat templates are not 1:1 compatible.
- **Mistral 7B Instruct v0.1 → v0.2 → v0.3** required re-tuning for production prompts (community-documented, no formal benchmark).

### 4.4 The semantic implications for `tested_against:`

The seed analysis recommends `tested_against:` as a frontmatter list. I endorse this and want to specify the semantics carefully because the field needs to actually be useful, not decorative.

**Recommended shape:**

```yaml
tested_against:
  - id: anthropic:claude-opus-4-7
    date: 2026-04-27
    fingerprint: null    # Anthropic does not publish
  - id: openai:gpt-4o-2024-11-20
    date: 2026-04-27
    fingerprint: fp_3a2b1c4d   # if known at authoring time
```

**Semantics:**
- `id` is a `provider:model` string. Required.
- `date` is the authoring or last-validation date. Required.
- `fingerprint` is the upstream `system_fingerprint` if available. Optional, may be `null`.
- A `tested_against:` list with one entry is fine; multi-entry is encouraged.

**What `brief validate` does:**
- If the user runs the brief against a model whose `id` is not in `tested_against:`, warn (do not fail).
- If the model's `id` matches but the upstream `system_fingerprint` does not match, warn (do not fail) — the model has been silently swapped.
- If the model's `id` matches *and* the fingerprint matches *and* the brief was validated within N days (configurable, default 90), no warning.

**What this is *not*:** a replay guarantee. The model could change weights and the fingerprint could be unchanged; the fingerprint could change and behavior could be unaffected. This is best-effort drift detection, not a hash. That is the honest framing.

**Why this matters more than it looks:** every empirical study of LLM-driven CI/CD systems I am aware of has found that *the most common failure mode is silent behavior change after model upgrade*. A `tested_against:` field that surfaces this in `brief validate` is the cheapest possible defense, and it gives the user a paper trail when the brief misbehaves.

---

## Section 5: The reproducibility envelope

If `tested_against:` and `decoding:` are the *primitives*, the reproducibility envelope is what they are *for*. This section catalogs what an LLM call's inputs actually are, so that the brief team has a mental model of what subset of those inputs the format should carry.

### 5.1 The full envelope

To replay an LLM call deterministically, you would need:

1. **Model identifier** (`provider:model:version`) — the public name.
2. **Tokenizer version** — the exact tokenizer artifact. For most providers this moves with the model id.
3. **Decoding parameters** — temperature, top_p, top_k, min_p, repetition_penalty, presence/frequency_penalty, max_tokens, stop sequences, reasoning_effort/thinking_budget, logit_bias.
4. **Seed** — random seed for sampling.
5. **System fingerprint** — provider-specific identifier of the served infrastructure (only OpenAI publishes this).
6. **Chat template** — the exact Jinja or hand-rolled template that converts messages to tokens.
7. **Content hash of the prompt** — sha256 of the rendered token stream (or, more practically, of the canonical message structure).
8. **Tool definitions** — exact JSON Schemas if applicable.
9. **Conversation history / context** — exact turns prior to the current generation.
10. **Server load / batch composition** — for kernels that are not batch-invariant, the batch size and composition affect output ([Thinking Machines, *Defeating Nondeterminism in LLM Inference*, 2024](https://thinkingmachines.ai/blog/defeating-nondeterminism-in-llm-inference/); [vLLM batch invariance documentation](https://docs.vllm.ai/en/latest/features/batch_invariance/)).
11. **Hardware / driver / kernel version** — floating-point semantics differ across GPU generations and CUDA versions.

### 5.2 What providers actually deliver

- **OpenAI:** seed (best-effort), system_fingerprint (changes "a few times a year" per [OpenAI Cookbook](https://cookbook.openai.com/examples/reproducible_outputs_with_the_seed_parameter)). The official wording is "mostly deterministic" — same seed + same params + same fingerprint → same output, with explicit small probability of divergence.
- **Anthropic:** no seed parameter. Temperature 0 produces non-deterministic output. Practitioners have filed CLI bug reports about this exact issue ([anthropics/claude-code#3370](https://github.com/anthropics/claude-code/issues/3370)). The official position is "design your application to be robust to minor variations" ([keywordsai 2025 analysis](https://www.keywordsai.co/blog/llm_consistency_2025)).
- **Google Gemini:** partial seed support; behavior varies by model.
- **Azure OpenAI:** mirrors OpenAI's seed/system_fingerprint contract ([Microsoft Learn](https://learn.microsoft.com/en-us/azure/ai-foundry/openai/how-to/reproducible-output)).
- **vLLM (open source serving):** seed works; with `VLLM_BATCH_INVARIANT=1`, batch-size invariance is achievable on H100+ at performance cost ([vLLM batch invariance docs](https://docs.vllm.ai/en/latest/features/batch_invariance/)). With `VLLM_ENABLE_V1_MULTIPROCESSING=0` in offline mode, scheduling is deterministic ([vLLM reproducibility docs](https://docs.vllm.ai/en/latest/usage/reproducibility/)).
- **SGLang, TGI, llama.cpp:** vary; seed is generally honored for sampling but batch invariance is not guaranteed.

### 5.3 Why batch invariance is the unsolved corner

Even with a fixed seed, GPU kernels for matmul, attention, and normalization choose different reduction schedules at different batch sizes for performance. Per [Thinking Machines Lab (2024)](https://thinkingmachines.ai/blog/defeating-nondeterminism-in-llm-inference/), this means that the same sequence at temperature 0 with the same seed can produce different tokens depending on what other requests share the batch. vLLM's batch-invariance flag forces deterministic kernel selection at performance cost; OpenAI does not expose this. The implication: *bit-perfect determinism in production LLM serving is currently a research direction, not a delivered product*.

### 5.4 What `brief` should carry

The seed analysis recommends a `decoding:` block with `seed`, `temperature`, `top_p`, `top_k`, `max_tokens`, `stop`, `reasoning_effort`. I endorse this and want to be specific about the field semantics.

**Recommended shape:**

```yaml
decoding:
  seed: 42
  temperature: 0.0
  top_p: 1.0
  max_tokens: 4096
  stop: ["\n\nHuman:", "</answer>"]
  reasoning_effort: medium    # null | low | medium | high
```

**Semantics:**
- All fields optional. A missing field means "provider default."
- `seed` is an integer. The emitter routes to the provider's seed parameter where supported; otherwise it is documentary.
- `reasoning_effort` is a controlled vocabulary (`null | low | medium | high`) per the OpenAI o-series convention; emitters map to provider-specific values (Anthropic's `thinking.budget_tokens`, OpenAI's `reasoning.effort`).
- `stop` is a list of strings.

**What `brief validate` does:**
- Warns if `seed` is set but the active provider does not honor it (e.g., Anthropic).
- Warns if `temperature` and `seed` are both set on a provider where `temperature: 0` already implies seed-irrelevant determinism (it does not, anywhere — but practitioners often think it does).
- Confirms the decoding parameters are syntactically valid.

**What emitters do:**
- The `aichat` emitter writes `temperature` and `top_p` into `config.yaml` and *drops* the rest with a stderr warning per the synthesis recommendation. Aichat does not consume `seed` today; this is exactly the kind of cooperative pressure on aichat to grow the field that the synthesis advocates.
- The `prompt` emitter produces a fully-formed API call (or a config snippet for one).
- The `claude` emitter inlines a brief comment block in CLAUDE.md noting the intended decoding (Claude Code does not honor decoding from CLAUDE.md).

**The honest framing in `brief`'s README:** decoding parameters in `.brief.md` are the *intended* envelope. Whether the runtime honors them is a runtime concern; whether the provider serves them deterministically is a provider concern. Brief carries the intent; brief does not promise replay.

---

## Section 6: Content hashing and `brief.id`

The synthesis recommends `brief.id` as a sha256 over a "canonical parse tree." That is the right design instinct, but the term "canonical parse tree" needs to be specified to the byte level — otherwise different implementations will hash differently and the field becomes unusable. Here is the recommended construction.

### 6.1 The lineage (how comparable systems do this)

- **Git tree SHAs**: a `tree` object is a sorted list of `(mode, name, sha)` tuples; the SHA of the tree is the SHA of that serialized list. The hash is invariant to filesystem ordering (because git sorts) and invariant to whitespace within tracked files (only blob SHA matters). [Git internals: tree objects](https://git-scm.com/book/en/v2/Git-Internals-Git-Objects).
- **OCI content-addressable storage**: an image layer is hashed by the SHA-256 of its compressed-tar bytes; the manifest is hashed by the SHA-256 of its canonical JSON. [OCI Image Spec](https://github.com/opencontainers/image-spec).
- **DSPy compiled artifacts**: a compiled DSPy program serializes its prompt + few-shot examples + signature into a deterministic JSON form, hashed as the artifact identifier ([DSPy 2024](https://github.com/stanfordnlp/dspy)).
- **MCP protocol-version negotiation**: not a hash, but a relevant comparable — MCP carries a `protocolVersion` field in initialize that the server and client must agree on. Brief's `version:` should follow this discipline (§7).
- **HuggingFace dataset/model SHAs**: `revision` is a git commit SHA; everything is content-addressable through git LFS.
- **JSON Canonicalization Scheme (JCS, RFC 8785)**: an IETF-standard canonical JSON serialization that produces a unique byte representation for any JSON value. The right primitive for hashing a parsed document.

### 6.2 The recommended construction for `brief.id`

**Construction:**

1. Parse the `.brief.md` file into the typed `Brief` struct (frontmatter + body sections + lists).
2. Serialize the struct as JSON via [JSON Canonicalization Scheme (RFC 8785)](https://datatracker.ietf.org/doc/html/rfc8785) — sorted keys, no whitespace, no insignificant zeros, UTF-8 NFC normalization.
3. SHA-256 the JCS bytes.
4. The result, hex-encoded, is `brief.id`.

**Field placement:** `brief.id` is *computed*, not authored. It does not appear in the source file. It is emitted into target artifacts (CLAUDE.md, prompt output, aichat `index.yaml` as a comment line) so that downstream tooling can attach to it.

**What it captures:** the canonical typed structure of the brief. Two semantically-equivalent briefs with different prose formatting (e.g., spaces in lists, trailing newlines) hash to the same value. Two briefs with different *meaning* (different goal, different constraints) hash differently.

**What it deliberately does not capture:**
- The user's `decoding.seed`. (`decoding:` *is* in the canonicalized struct, so it does affect the hash. If the user changes seed, the hash changes — this is correct: same brief with different seed is operationally a different artifact.)
- Authoring order of constraints within a tier. (We sort lists at canonicalization time? **No** — see §6.3.)
- Whitespace, comments, frontmatter ordering.

### 6.3 Sort vs. preserve: a load-bearing decision

Should constraint lists be sorted before hashing?

**Argument for sort:** hashes become invariant to authoring order. Two authors who list the same hard constraints in different order get the same `brief.id`.

**Argument against sort:** order can be semantically meaningful (priority order; sequencing in `## Process`). Sorting destroys that.

**Recommendation:** preserve order. Authoring order is information; if the author cares enough to write `cargo test` before `cargo clippy`, that is part of the contract. This matches how git tracks file order (sorted by name, but inside files order matters), how OCI hashes layers (order-sensitive), and how JCS canonicalizes JSON arrays (preserves order).

The implication: if two briefs have the same content but list hard constraints in different order, their `brief.id`s differ. That is correct. If the user wants order-invariance, they sort.

### 6.4 What the field unlocks

`brief.id` is the linchpin for everything downstream:

- **`brief diff`** — given two ids, fetch and structurally compare.
- **`brief validate --pinned <hash>`** — refuse to run if the brief has been edited since the pin.
- **CI gating** — block PRs that change the brief without bumping `version:`.
- **Eval-result caching** — Promptfoo / Inspect AI / Braintrust can cache scores against `(brief.id, model_id, system_fingerprint, decoding_seed)` and never re-run a duplicate.
- **Provenance stamping** — emitted CLAUDE.md / prompt / aichat artifacts carry `brief.id` so an agent's output can be traced back to the brief that produced it.

This is a one-parser-change, high-leverage intervention. The synthesis is right to put it in Tier 1.

### 6.5 Implementation note (Rust ecosystem)

The Rust crates needed:
- `serde_json` already in scope.
- `serde_jcs` for JSON Canonicalization Scheme (RFC 8785) — small, single-purpose crate.
- `sha2` for SHA-256.

Total binary-size impact: <100KB. Well within the single-binary constraint.

---

## Section 7: Version semantics and additive-only evolution

The seed analysis is silent on `version:` semantics; the synthesis and the veteran review push for "real semver, additive-only within a major." I want to make this concrete because it is where most ML formats fail at year three.

### 7.1 What aichat's `version:` is, and why it is a cautionary tale

Aichat's `index.yaml` carries `version: "0.1.0"` (per the [aichat backend README](../../design/backends/aichat/README.md)). The field is parsed but never enforced — no compatibility check at load time, no schema-evolution story, no deprecation markers. It is decorative. The cost of this decision compounds: the moment a downstream tool wants to depend on aichat agents, the tool has to guess at format compatibility from undocumented field-presence heuristics.

Compare to MCP, which carries a `protocolVersion` string in every initialize handshake. The server inspects, negotiates, or refuses ([MCP specification](https://modelcontextprotocol.io/specification/)). Compare to Protobuf, which encodes field numbering as a forward-compatibility primitive — old code skips unknown fields, new code reads old messages. Compare to OpenAPI, which uses semver to gate breaking changes. These are systems that survive ten years; aichat's `version:` will not, and `brief` should not copy that mistake.

### 7.2 The recommended discipline for `brief`'s `version:`

**Field shape:** a string, currently `"1"` (per the existing CLAUDE.md spec).

**Semantics:**
- The version is the *major* of the format.
- Within a major, evolution is **additive-only**. New optional fields, new optional sections, new emit-target-specific extensions can land. Existing fields cannot change semantics. Required fields cannot be added.
- A major bump (`"1"` → `"2"`) is the only path for breaking changes. It implies an incompatibility marker; old `brief` binaries should refuse to parse.
- Unknown fields are preserved (the existing `unknown_sections` discipline in the parser).

**What `brief validate` does:**
- Reject `.brief.md` files with `version:` higher than the binary supports.
- Warn on `version:` lower than the current binary's major (suggest migration).
- For `version: "1"` files, validate against the v1 schema; unknown fields preserved.

**What this enables:**
- `brief migrate v1-to-v2` becomes tractable when v2 lands.
- The format can grow `verify:`, `tested_against:`, `decoding:`, `pin:`/`retrieve:`/`examples:`, `capabilities:`, `budget:`, etc. *all within v1* because all are additive optional fields. This is the Tier-1/Tier-2/Tier-3 plan from the synthesis: every field on the list is additive within v1.

### 7.3 Comparisons to other formats' version evolution

- **Kubernetes API**: `apiVersion: v1` / `apiVersion: apps/v1` is precisely this discipline; alpha/beta gates exist for optional new behavior. This works at planet scale.
- **JSON Schema**: meta-schema versioning with explicit dialects. Brief is simpler; one major is fine for years.
- **Cargo's `Cargo.toml`**: `edition = "2024"` carries breaking-change guards; within an edition, additive-only.
- **Python's `pyproject.toml` (PEP 621)**: implicit version through field-presence; less disciplined than Cargo, more disciplined than aichat.

### 7.4 The non-stationarity argument applied to versioning

The veteran review made the strongest version-related point: *the model-behavior contract is non-stationary*. This means the format's `version:` carries one kind of compatibility (parser-side), but the *behavioral* compatibility of the brief against a given model is a separate axis carried by `tested_against:`. Both are needed. `version:` says "this brief parses." `tested_against:` says "this brief behaves." Conflating them is a classic ML-tooling mistake and `brief` should not make it.

---

## Section 8: Eval-driven brief authoring

Should brief reserve a separate `eval:` slot, distinct from `verify:`? The seed analysis is silent on this; the synthesis treats them as the same primitive. I want to argue that they should *stay* the same primitive but that brief should *also* ship a tiny golden-set convention.

### 8.1 The tension

The `verify:` slot, as recommended, is a list of shell commands that pass or fail. That is a *binary check on the present codebase state*. It is not the same thing as a *test set* — a list of input prompts paired with expected outputs.

A test set is what every eval harness consumes: lm-evaluation-harness's `(doc_to_text, doc_to_target)`, HELM's `Instance`, Inspect AI's `Sample`, Promptfoo's `tests`, OpenAI Evals' JSONL-of-input-output-pairs. Without a test set, an eval harness has nothing to score against.

**Option A: separate `eval:` slot.** Make `eval:` a path or inline list of input/expected pairs. This is what most eval frameworks expect.

**Option B: golden-set convention via `verify:`.** The user writes `verify: ["./eval/run.sh"]`, and `run.sh` reads `./eval/inputs/*.json`, sends each to the model, compares against `./eval/expected/*.json`, and exits 0/non-zero. The `verify:` slot stays a single primitive; the test set lives in the project's filesystem.

### 8.2 The recommendation: Option B with a documented convention

**Why:** the 60-second-authoring constraint forbids forcing brief authors to learn an eval-harness DSL. Shell + filesystem is the lowest-friction convention. The author who wants no eval writes nothing; the author who wants a 3-pair golden set writes 3 JSON files and a shell script.

**The convention (documentary, not enforced):**

```
my-project/
├── .brief.md
└── eval/
    ├── inputs/
    │   ├── 001.json
    │   ├── 002.json
    │   └── 003.json
    ├── expected/
    │   ├── 001.json
    │   ├── 002.json
    │   └── 003.json
    └── run.sh
```

`verify:` includes `./eval/run.sh`. `run.sh` is whatever the user wants — exact-match, regex, an Inspect AI run, a Promptfoo run.

**What this gets:**
- The 60-second author writes nothing extra.
- The author who wants three input/expected pairs gets a working eval suite in 5 minutes.
- The author who wants HELM-grade evaluation hands `./eval/` to Inspect AI's converter and gets a real harness.
- `brief.id` covers the brief; the user's source-control covers `./eval/`. No new format machinery needed.

**What this avoids:** a separate `eval:` slot that brief would have to specify, validate, and emit. The synthesis-tier discipline holds: name the slot only when there is no other place to put the contract; here, `verify:` + filesystem convention is exactly enough.

### 8.3 The interaction with `tested_against:`

The cleanest workflow:

1. Author `.brief.md` with `tested_against: [anthropic:claude-opus-4-7]`.
2. Author 3 input/expected pairs.
3. Run `brief validate` — runs `verify:`, runs the 3-pair eval, all green.
4. Six months later, model bumps to claude-opus-4-9.
5. Run `brief validate` — `tested_against:` warns (model not in list); `verify:` runs; one pair fails.
6. User sees what drifted, decides whether to update the brief, re-add the model to `tested_against:`, or abandon.

This is the entire eval-driven authoring loop. It is achievable in v1 with two new optional frontmatter fields and a documentary convention. There is no Tier-1 or Tier-2 commitment to a separate `eval:` slot.

### 8.4 The 60-second constraint, re-examined

A reasonable concern: does shipping a golden set violate the 60-second-authoring constraint? Three things:

1. The 60-second constraint applies to *minimum-viable authoring*. A brief with no `eval/` directory still authors in 60 seconds. The eval directory is opt-in.
2. The 60-second constraint does not apply to *eval rigor*. A brief that wants an eval is, by definition, willing to spend more than 60 seconds.
3. The convention is opt-in and lives outside the brief. The brief itself does not grow.

This is the right altitude. The synthesis got it right by keeping `verify:` and rejecting `eval:`; this section's job is to spell out *why* that is sufficient.

---

## Section 9: Process vs. outcome rewards, and the assumption checkbox

The RL engineer review made the single most format-native observation in the entire synthesis: `brief`'s assumption checkbox is, in spirit, a process-reward primitive. I want to expand on this because the literature on process-vs-outcome rewards is the strongest case for the seed/synthesis recommendation to generalize the assumption checkbox.

### 9.1 The literature

**Outcome rewards** grade only the final answer. Cobbe et al.'s GSM8K verifier is an outcome reward; HumanEval's `pass@k` is an outcome reward; SWE-Bench's test-suite is an outcome reward. These rewards are robust but sparse: a 20-step solution that goes wrong at step 3 gets the same penalty as one that goes wrong at step 19, which makes credit assignment hard.

**Process rewards** grade each intermediate step. Lightman et al.'s "Let's Verify Step by Step" (PRM800K) is the foundational paper ([Lightman et al. 2023, arXiv:2305.20050](https://arxiv.org/abs/2305.20050)). They labeled 800,000 step-level human judgments on MATH-dataset solutions and found that *process supervision substantially outperforms outcome supervision* — their best process-supervised model solves 78% of a representative MATH subset, outperforming their best outcome-supervised model.

**Math-Shepherd** ([Wang et al. 2023, arXiv:2312.08935](https://arxiv.org/abs/2312.08935)) automated process reward construction by Monte Carlo Tree Search rollouts: a step is "good" if it leads to a correct answer in expectation. This eliminates the human-labeling bottleneck. Math-Shepherd improves Mistral-7B from 77.9% → 84.1% on GSM8K and 28.6% → 33.0% on MATH, with verification raising those to 89.1% and 43.5%.

**The PRM-vs-ORM consensus (2024–2026):** for long-horizon tasks (5+ steps), process rewards dominate. For short tasks, outcome rewards are sufficient. The agentic-coding regime is firmly in the long-horizon camp ([Kwon et al. 2024](https://arxiv.org/abs/2501.07301), survey on PRM development).

### 9.2 The assumption checkbox as a poor-man's PRM

`brief`'s `## Assumptions` section already encodes step-level signals:

```markdown
## Assumptions
- [ ] Bottleneck is synchronous DB writes
- [x] We have a baseline benchmark
- [ ] p99 > 200ms is the threshold
```

Each line is a step-level claim. `[ ]` is unvalidated; `[x]` is validated. The format already supports the *shape* of process-reward signals. The RL engineer review is right that generalizing this with optional shell checks is the most format-native improvement on offer.

**Recommended generalization (Tier 3 per the synthesis):**

```markdown
## Assumptions
- [ ] Bottleneck is sync DB writes — `./bench/db.sh > /tmp/r; grep -q "p99 > 200ms" /tmp/r`
- [x] We have a baseline benchmark
- [ ] p99 > 200ms is the threshold
```

**Semantics:**
- The check command is optional. Lines without a command stay prose.
- `brief validate` runs the commands. Exit-zero flips the checkbox to `[x]` (in-memory; the source file is untouched unless `brief validate --update` is set).
- Unflipped boxes after validation are surfaced as warnings.

**Backwards compatibility:** existing assumption lists keep working. The em-dash + backticked-shell pattern is parser-distinguishable from prose.

### 9.3 What this is *not*, and what it could become

This is *not* a learned process reward model. The brief author hand-writes the checks. It is closer to a unit test list than to a PRM in the Lightman/Wang sense.

What it *could* become, downstream, is a corpus of step-level signals that a future PRM trainer could consume — every brief in a corporate codebase contributes a few labeled `(step, pass/fail)` pairs. This is speculative; brief should not commit to it. But naming the slot in v1 with the right shape leaves the door open.

### 9.4 Why this is high-leverage despite being Tier 3

The synthesis correctly puts assumption-checkbox-with-shell-check in Tier 3, but the literature suggests the leverage is higher than Tier 3 implies. Three things make it underrated:

1. **It is the only step-level primitive in the format.** Every other constraint (`hard`, `soft`, `ask_first`, `sacred`, `verify:`) is either a global constraint or an outcome check. The assumption list is the only place where the brief models *intermediate state*.

2. **It generalizes with zero friction.** The em-dash + backticked-shell parser tweak is one change in `parse/body.rs`. Backwards-compatible. No frontmatter growth.

3. **It maps directly onto the long-horizon-agent regime brief is being aimed at.** Coding agents that work for 5-30 steps need PRM-shaped signals; the assumption list is the simplest viable PRM-shaped signal an authoring format can carry.

I would push this to Tier 2 in the next revision. The cost is one parser tweak; the leverage is a structural step-level primitive.

---

## Section 10: Recommendations and tier table

This section consolidates the prior nine into a prioritized recommendation list compatible with the existing tier table in `aichat-agent-gaps.md`. Where I agree with the seed and synthesis I say so; where I push further or disagree I note it.

### 10.1 Where I fully agree with the seed and synthesis

- **Split `context:` into `pin / retrieve / examples`** (Tier 1). Eval-input clarity: it is impossible to write a meaningful evaluation harness when the context channel is ambiguous between always-in-context and retrieved.
- **`verify:` slot in frontmatter** (Tier 1). Verifiable rewards survive model bumps; LLM-judge slots do not. The literature is unambiguous (§2, §3).
- **`brief.id` content hash + meaningful `version:`** (Tier 1). The eval ecosystem keys on content addressability; this is the cheapest possible wedge.
- **`brief emit aichat` warns on lossy projection** (Tier 1). Emit honesty is a precondition for downstream eval reproducibility — silent loss is worse than no loss because it pollutes trust in the artifact.
- **`tested_against:` model list** (Tier 2). Drift detection without this is impossible; with this it is automatic.
- **`decoding:` block with seed** (Tier 2). The reproducibility envelope minimum.
- **No `judge:` slot, ever** (out of scope). Document the omission.

### 10.2 Where I push further than the seed/synthesis

- **`brief.id` should be a SHA-256 over a JSON-Canonicalization-Scheme (RFC 8785) serialization of the typed Brief struct.** §6 specifies. The synthesis says "canonical parse tree" but does not specify; I am specifying.
- **`tested_against:` should carry `(id, date, fingerprint)` triples**, not bare model strings. §4.4. This costs one extra field per entry and gives drift detection a real anchor when fingerprints are available.
- **Promote assumption-checkbox-with-shell-check from Tier 3 to Tier 2.** §9. The leverage is higher than the synthesis recognized; the cost (one parser tweak) is lower than Tier-3 items.
- **Ship a documented golden-set convention via filesystem (`./eval/inputs/`, `./eval/expected/`, `./eval/run.sh`)** rather than a separate `eval:` slot. §8. This stays format-first and 60-second-authorable.
- **`brief validate` should run `verify:` and surface a structured pass/fail report**, not just check syntax. The seed analysis treats `validate` as a parse-time check; I am pushing it toward an eval-time gate.

### 10.3 Where I disagree (gently) with the seed analysis

- The seed analysis recommends `deliverable_schema:` for structured-output JSON Schema as Tier 3. I think this is *Tier 2* if the brief team takes structured outputs seriously, because it is the cleanest emit target for OpenAI Structured Outputs / Anthropic tool-use / aichat `output_schema:`, and *all three* benefit. But the synthesis kept it at Tier 3 and I do not have strong evidence to override that ordering; just flagging.
- The seed analysis does not engage with batch-invariance / kernel-determinism issues (§5.3). This is fine for v1 — the brief format cannot carry server load — but the README should acknowledge it so authors do not develop false confidence in seed-based replay.

### 10.4 Updated tier table

Compatible with `aichat-agent-gaps.md`. Lane indicates which review lens drove the recommendation: TF = transformer/seed, RL = RL-rewards, V = veteran, **E** = this eval-lens review (new contributions or sharpened items in **bold**).

| Tier | Item | Format change? | Cost | Lane | Why now |
|---|---|---|---|---|---|
| **1** | Split `context:` → `pin / retrieve / examples` | Frontmatter | Parser + emitters | TF, V | Highest single information-loss gap |
| **1** | `verify:` slot (and/or `## Verification` H2) | Both | Small parser change | RL, **E** | Verifiable rewards survive model bumps; judges do not |
| **1** | `brief.id` (SHA-256 over JCS-canonical JSON of typed struct) | Frontmatter (computed) | Parser + serde_jcs + sha2 | V, **E** | **Specify the canonicalization to byte level (§6)**; eval ecosystem keys on it |
| **1** | `brief emit aichat` warns on lossy projection | Emitter only | Emitter logic | TF, **E** | **Emit honesty is precondition for replay trust** |
| **2** | `tested_against:` (id + date + optional fingerprint) | Frontmatter | Trivial | TF, V, **E** | **Sharpen to triple form (§4.4)** to anchor drift detection |
| **2** | `decoding:` block (seed / temp / top_p / max_tokens / stop / reasoning_effort) | Frontmatter | Trivial | TF, **E** | **Document provider asymmetry (§5.2) in README** |
| **2** | `capabilities:` controlled vocabulary | Frontmatter | Schema + emitter mapping | V | Right altitude for tools without modeling JSON Schema |
| **2** | `budget:` block (max_turns, max_cost_usd, wall_clock_minutes) | Frontmatter | Trivial | RL | Real protection against tool-loop incidents |
| **2** | **Assumption checkbox with optional shell check (PROMOTED from T3)** | Body parser | Backwards-compatible tweak | RL, **E** | **Only step-level / process-reward primitive in the format (§9)** |
| **2** | Constraint structure preserved as source-of-truth | Discipline | None | All | Reaffirm; no concession to aichat |
| **2** | **Documented golden-set convention (`./eval/`)** | None (convention) | Documentation | **E** | **60-s authoring preserved; eval rigor opt-in (§8)** |
| **3** | `deliverable_schema:` (inline JSON Schema or path) | Frontmatter | Schema + emitter routing | TF | Compiles to OpenAI Structured Outputs / aichat output_schema / Claude tool |
| **3** | Reserve `delegates_to: [name, ...]` | Frontmatter | Field only | V, RL | Composition by name, not by graph |
| **3** | Reserve `trajectory_log:` and `preference_sink:` | Frontmatter | Two optional strings | RL | Stub-name the contract surface |
| **3** | Reserve `tools:` (MCP refs / aichat slugs) | Frontmatter | Field only | TF | Future capability without modeling tool I/O |
| **out** | LLM-as-judge config, tool I/O JSON Schemas, memory architecture, trace formats, eval-suite formats, sandbox policy, multi-agent graph topology, constrained-decoding choice | — | — | All | Runtime concerns; **document the LLM-judge omission explicitly per §3** |

### 10.5 Specific recommendations for the load-bearing fields

**`tested_against:`** — list of objects with `id` (string, required), `date` (ISO-8601 date, required), `fingerprint` (string, optional). `brief validate` warns on mismatch. Don't fail; drift detection is informational, not a gate.

**`decoding:`** — object with optional `seed`, `temperature`, `top_p`, `top_k`, `max_tokens`, `stop` (list of strings), `reasoning_effort` (`null|low|medium|high`). Emitters route per provider; warn on dropped fields.

**`verify:`** — list of strings (shell commands) OR list of objects `{cmd: string, timeout_s: int}` for Tier-2 timeout support. Run sequentially; first non-zero exit fails. Working dir is brief's directory.

**`brief.id`** — computed at parse time. SHA-256 over JCS-canonicalized JSON of the typed `Brief` struct. Emitted into target artifacts as a comment line; never authored in source.

**`version:`** — string, currently `"1"`. Additive-only within major. `brief migrate` exists but is empty in v1 (no migration needed yet).

### 10.6 Ordering for implementation

Practical implementation order, reflecting the synthesis Tier-1 commitment:

1. `brief.id` parser change + JCS dep + SHA-256 stamping into all emit targets.
2. `verify:` slot + parser + `brief validate` integration.
3. Lossy-emit warnings in the aichat emitter for known dropped fields.
4. `pin: / retrieve: / examples:` split — parser change + emitter routing.
5. `tested_against:` + `decoding:` (Tier 2 but cheap; bundle with Tier 1).
6. `budget:` and `capabilities:` (Tier 2).
7. Assumption-checkbox-with-shell-check (promoted Tier 2).
8. Documented `./eval/` convention in README + sample brief.
9. Tier 3 reservations (namespace-only).

This is achievable in 6–10 PRs. None of it violates the single-binary or 60-second constraints. None of it requires a `version:` bump (everything is additive-within-v1).

### 10.7 What I would put in the README, verbatim

The brief team's README should carry, in plain language:

> **Reproducibility caveat.** A `.brief.md` carries `tested_against:`, `decoding:`, and `brief.id` as best-effort reproducibility primitives. Bit-perfect replay of LLM calls is not a delivered feature of any major provider as of 2026; OpenAI publishes `system_fingerprint` to indicate when their served weights change ([OpenAI Cookbook 2024](https://cookbook.openai.com/examples/reproducible_outputs_with_the_seed_parameter)), Anthropic does not expose seed or fingerprint, and even with vLLM's batch-invariance flag enabled, kernel determinism requires specific hardware ([vLLM batch invariance docs](https://docs.vllm.ai/en/latest/features/batch_invariance/)). Brief carries the *intent* to be reproducible; whether the runtime delivers on that intent is a runtime concern.

> **No LLM-as-judge slot.** The format does not carry a `judge:` field. LLM-as-judge protocols are documented to suffer position bias, length bias, and self-preference bias ([Zheng et al. 2023](https://arxiv.org/abs/2306.05685); [Shi et al. 2024](https://arxiv.org/abs/2406.07791); [Panickssery et al. 2024](https://arxiv.org/abs/2410.21819); [Dubois et al. 2024](https://arxiv.org/abs/2404.04475)), and the calibration techniques that mitigate these biases drift across judge-model versions. Use `verify:` for verifiable rewards; use a runtime tool (Promptfoo, Inspect AI, Braintrust) for judge configuration.

These two paragraphs would do more for downstream user expectations than any feature.

---

## References

### Eval harnesses, benchmarks, and methodologies

- Chen, M. et al. (2021). *Evaluating Large Language Models Trained on Code.* arXiv:2107.03374. https://arxiv.org/abs/2107.03374 (HumanEval; pass@k methodology.)
- Cobbe, K. et al. (2021). *Training Verifiers to Solve Math Word Problems.* arXiv:2110.14168. https://arxiv.org/abs/2110.14168 (GSM8K dataset; verifier methodology.)
- Hendrycks, D. et al. (2021). *Measuring Mathematical Problem Solving With the MATH Dataset.* arXiv:2103.03874. https://arxiv.org/abs/2103.03874 (MATH benchmark.)
- Liang, P. et al. (2022). *Holistic Evaluation of Language Models (HELM).* arXiv:2211.09110. https://arxiv.org/abs/2211.09110
- Suzgun, M. et al. (2022). *Challenging BIG-Bench Tasks and Whether Chain-of-Thought Can Solve Them.* arXiv:2210.09261. https://arxiv.org/abs/2210.09261 (BBH.)
- Zheng, L. et al. (2023). *Judging LLM-as-a-Judge with MT-Bench and Chatbot Arena.* arXiv:2306.05685. https://arxiv.org/abs/2306.05685 (MT-Bench; original LLM-as-judge methodology and bias documentation.)
- Liu, Y. et al. (2023). *G-Eval: NLG Evaluation using GPT-4 with Better Human Alignment.* arXiv:2303.16634. https://arxiv.org/abs/2303.16634
- Chiang, W.-L. et al. (2024). *Chatbot Arena: An Open Platform for Evaluating LLMs by Human Preference.* arXiv:2403.04132. https://arxiv.org/abs/2403.04132
- Li, T. et al. (2024). *From Crowdsourced Data to High-Quality Benchmarks: Arena-Hard and BenchBuilder Pipeline.* arXiv:2406.11939. https://arxiv.org/abs/2406.11939 (Arena-Hard methodology and Pair Rank Brier Score calibration.)
- Dubois, Y., Galambosi, B., Liang, P., & Hashimoto, T. B. (2024). *Length-Controlled AlpacaEval: A Simple Way to Debias Automatic Evaluators.* arXiv:2404.04475. https://arxiv.org/abs/2404.04475
- Wang, Y. et al. (2024). *MMLU-Pro: A More Robust and Challenging Multi-Task Language Understanding Benchmark.* arXiv:2406.01574. https://arxiv.org/abs/2406.01574 (NeurIPS 2024.)
- Jain, N. et al. (2024). *LiveCodeBench: Holistic and Contamination Free Evaluation of Large Language Models for Code.* arXiv:2403.07974. https://arxiv.org/abs/2403.07974
- Jimenez, C. et al. (2024). *SWE-bench: Can Language Models Resolve Real-world Github Issues?* arXiv:2310.06770. https://arxiv.org/abs/2310.06770
- SWE-Bench team & OpenAI (2024). *SWE-bench Verified.* https://www.swebench.com/verified.html (500-instance human-filtered subset.)
- Yao, S., Shinn, N., Razavi, P., & Narasimhan, K. (2024). *τ-bench: A Benchmark for Tool-Agent-User Interaction in Real-World Domains.* arXiv:2406.12045. https://arxiv.org/abs/2406.12045
- Patil, S. et al. (2024). *The Berkeley Function Calling Leaderboard (BFCL): From Tool Use to Agentic Evaluation of Large Language Models.* ICML 2025. https://gorilla.cs.berkeley.edu/leaderboard.html
- EleutherAI (2024–2026). *lm-evaluation-harness.* https://github.com/EleutherAI/lm-evaluation-harness
- UK AISI / Arcadia Impact / Vector Institute (2024–2026). *Inspect AI.* https://inspect.aisi.org.uk/ ; https://github.com/UKGovernmentBEIS/inspect_ai (Open-source LLM evaluation framework with strong agentic-task and tool-use orientation.)
- Promptfoo (2024–2026). *Promptfoo: Test your prompts, agents, and RAGs.* https://github.com/promptfoo/promptfoo

### Verifiers, RLVR, process rewards

- Lightman, H. et al. (2023). *Let's Verify Step by Step.* arXiv:2305.20050. https://arxiv.org/abs/2305.20050 (PRM800K; process supervision dominates outcome supervision.)
- Wang, P. et al. (2023). *Math-Shepherd: Verify and Reinforce LLMs Step-by-step without Human Annotations.* arXiv:2312.08935. https://arxiv.org/abs/2312.08935 (ACL 2024.)
- Lambert, N. et al. (2024). *Tülu 3: Pushing Frontiers in Open Language Model Post-Training.* arXiv:2411.15124. https://arxiv.org/abs/2411.15124 (RLVR formalization.)
- DeepSeek-AI (2025). *DeepSeek-R1: Incentivizing Reasoning Capability in LLMs via Reinforcement Learning.* arXiv:2501.12948. https://arxiv.org/abs/2501.12948 (Pure rule-based RL; emergent self-reflection.)
- Bai, Y. et al. (2022). *Constitutional AI: Harmlessness from AI Feedback.* arXiv:2212.08073. https://arxiv.org/abs/2212.08073
- Skalse, J. et al. (2022). *Defining and Characterizing Reward Hacking.* arXiv:2209.13085. https://arxiv.org/abs/2209.13085

### Judge bias literature

- Shi, L. et al. (2024). *Judging the Judges: A Systematic Study of Position Bias in LLM-as-a-Judge.* arXiv:2406.07791. https://arxiv.org/abs/2406.07791
- Panickssery, A. et al. (2024). *Self-Preference Bias in LLM-as-a-Judge.* arXiv:2410.21819. https://arxiv.org/abs/2410.21819

### Reproducibility / determinism

- OpenAI (2024). *How to make your completions outputs consistent with the new seed parameter.* OpenAI Cookbook. https://cookbook.openai.com/examples/reproducible_outputs_with_the_seed_parameter
- Microsoft Learn (2024–2026). *How to generate reproducible output with Azure OpenAI.* https://learn.microsoft.com/en-us/azure/ai-foundry/openai/how-to/reproducible-output
- vLLM project (2024–2026). *Reproducibility documentation.* https://docs.vllm.ai/en/latest/usage/reproducibility/
- vLLM project (2024–2026). *Batch Invariance feature.* https://docs.vllm.ai/en/latest/features/batch_invariance/
- Thinking Machines Lab (2024). *Defeating Nondeterminism in LLM Inference.* https://thinkingmachines.ai/blog/defeating-nondeterminism-in-llm-inference/
- KeywordsAI (2025). *How to get consistent and reproducible LLM outputs in 2025 (OpenAI, Gemini, Claude, vLLM).* https://www.keywordsai.co/blog/llm_consistency_2025
- anthropics/claude-code GitHub (2025). *Issue #3370: Claude CLI produces non-deterministic output for identical inputs.* https://github.com/anthropics/claude-code/issues/3370

### Format / spec lineage

- IETF RFC 8785 (2020). *JSON Canonicalization Scheme (JCS).* https://datatracker.ietf.org/doc/html/rfc8785
- Open Container Initiative. *OCI Image Spec.* https://github.com/opencontainers/image-spec
- Git documentation. *Git Internals — Git Objects.* https://git-scm.com/book/en/v2/Git-Internals-Git-Objects
- Model Context Protocol (2024–2026). *MCP Specification.* https://modelcontextprotocol.io/specification/
- Stanford NLP (2024). *DSPy.* https://github.com/stanfordnlp/dspy
- Meta AI (2024). *Llama 3 Model Card.* https://github.com/meta-llama/llama3/blob/main/MODEL_CARD.md

### Brief project documents

- `analysis-llm-supervised.md` — transformer/structured-decoding lane review.
- `analysis-ml-veteran.md` — historical/cognitive-architectures lane review.
- `analysis-rl-online-rewards.md` — RL/reward-systems lane review.
- `aichat-agent-gaps.md` — synthesis of the above three reviews.
- `docs/design/backends/aichat/README.md` — aichat backend design document.
- `CLAUDE.md` — `brief` project instructions and format specification.
- `README.md` — `brief` project README.

---

*End of document.*
