# Friction Analysis: The Human-to-LLM Communication Pipeline

*Agent: Friction Analyst*

---

## Part 1: Where Friction Lives Today

Ranked by severity and opportunity (highest first):

---

### 1. The "I know it when I see it" problem -- Severity: Critical

The developer has a mental image of the finished work -- an architectural shape, a code feel, a performance envelope -- but no vocabulary to express it. They write "redesign the event pipeline" when what they actually mean is "I want the same fan-out pattern we used in the notification service, but with backpressure, and it should feel like the Kafka Streams topology DSL, not like raw consumer polling."

The brief format captures *what* and *constraints*, but not *aesthetic intent* or *reference implementations*. The current `context` field (defined in `/Volumes/ExternalData/admin/Developer/Projects/brief/src/model.rs` as `pub context: Vec<String>`) points at files, but there is no way to say "do it *like this*" with a pointer to a pattern rather than a file. The distinction matters: a context file says "here is information"; a reference pattern says "here is a template for the shape of the solution."

---

### 2. Context is scattered and the human is the only index -- Severity: Critical

A developer working on a codebase knows that the performance issue relates to three files, a Slack thread from two weeks ago, a PR comment, and a mental model of how the ORM batches queries. None of this is addressable.

The current `context: [./file1, ./file2]` field requires the human to manually enumerate relevant files. For any non-trivial task, the relevant context lives across 10-30 files, prior git commits, issue trackers, and the developer's head. The human becomes a manual RAG pipeline, and they are terrible at it -- they forget files, include irrelevant ones, and cannot articulate *why* a file is relevant.

Looking at `detect_context` in `/Volumes/ExternalData/admin/Developer/Projects/brief/src/init.rs`, the tool currently checks a static list of five well-known filenames (README.md, docs/architecture.md, etc.). This helps with the first 5% of context discovery. The other 95% -- the files actually relevant to *this specific task* -- is left entirely to the developer.

---

### 3. Constraint granularity mismatch -- Severity: High

The current model has three constraint tiers: Hard, Soft, Ask First. But real constraints have far more dimensions. "Don't break the API" is a hard constraint, but it is also *scoped* (only to v2 endpoints), *conditional* (except during the planned deprecation window), and *verifiable* (run the contract test suite).

The flat list of strings in `constraints.hard: Vec<String>` loses all of this structure. A developer either over-specifies (writing a paragraph per constraint) or under-specifies (writing "backward compatibility" and hoping the agent infers scope). There is no mechanism for a constraint to carry its own verification command, scope boundary, or expiration condition.

---

### 4. The blank page problem -- init is not enough -- Severity: High

`brief init` detects stack and sacred candidates from the filesystem. Looking at `scaffold_brief` in init.rs, it produces a template with placeholders like `# <Describe your goal here>` and `- <non-negotiable constraint>`. This is good for the first 10% of the brief.

But the transition from auto-detected structure to human-authored content is a cliff. The developer opens the file, sees the placeholders, and has to context-switch from "I want to fix this bug" (action mode) to "let me formally describe what I want" (specification mode). This cognitive mode switch is where authoring stalls. The tool detects *what the project is* but cannot help articulate *what the developer wants to do with it*.

---

### 5. Temporal context is invisible -- Severity: High

A brief is a snapshot. It does not know that this is the third iteration of this task, that the first two attempts failed because of a database locking issue, that the developer tried approach X and it did not work.

The model receives the brief cold every time. Session history, failed approaches, and incremental learning are lost between invocations. There is no field for "what I already tried" or "what the agent did wrong last time." The `Brief` struct in model.rs has `unknown_sections: Vec<UnknownSection>` which could theoretically hold custom sections, but there is no convention for temporal context, no tooling to auto-populate it from git history of the `.brief.md` itself, and no guidance that such a section would be useful.

---

### 6. Urgency, confidence, and risk signals are absent -- Severity: Medium-High

When a developer says "fix the login bug," they might mean "this is blocking production, drop everything" or "clean this up when you get a chance." The brief format has no way to express:

- **Urgency / priority** -- how quickly does this need to be done?
- **Developer confidence level** -- "I'm pretty sure the bug is in the session handler" vs. "I have no idea where this is"
- **Risk tolerance** -- "move fast, I'll review carefully" vs. "be extremely conservative, this is the payment path"
- **Scope tolerance** -- "if you need to refactor the adjacent module, go for it" vs. "surgical fix only"

These signals dramatically change how an agent should behave. They are currently communicated only through tone in free-text prompts, which models partially decode but imperfectly. The frontmatter in model.rs has `stack`, `context`, `model`, and `version` -- all structural metadata. None of these behavioral/attitudinal metadata fields exist.

---

### 7. The assumption validation loop is broken -- Severity: Medium-High

The Assumptions section with `- [ ]` / `- [x]` checkboxes is a strong idea. But the loop is entirely manual. Looking at how assumptions flow through the system: they are parsed with `validated: bool` and `has_checkbox: bool`, emitted faithfully by the prompt emitter (which separates validated from unvalidated), and validated only for checkbox syntax.

There is no mechanism for the agent to *challenge* an assumption, *report* that an assumption was tested and found false, or *add new assumptions* it discovered during work. The validation in `/Volumes/ExternalData/admin/Developer/Projects/brief/src/validate.rs` checks that checkboxes are present but not whether the assumptions are *testable* or *tested*. Assumptions are write-once, validate-never in practice.

---

### 8. The emit target is chosen too late -- Severity: Medium

The developer writes a `.brief.md` and then runs `brief emit claude` or `brief emit prompt`. But the target runtime affects how the brief should be authored.

Claude Code can read files, run commands, and ask clarifying questions. A brief optimized for Claude Code (where you can say "look at the test suite") is different from a brief optimized for a stateless API prompt (where you need to inline the relevant test cases). Looking at the emitters: `emit_claude` produces markdown with headings and formatting; `emit_prompt` produces flat uppercase-labeled text. These are surface-level format differences. The deeper issue is that the *content* should differ based on the agent's capabilities, and there is no mechanism for that.

---

### 9. Multi-agent coordination is unaddressed -- Severity: Medium

As agent-driven workflows become common, a single task may involve multiple agents -- one for architecture, one for implementation, one for testing. The brief format assumes a single recipient. There is no way to express task decomposition, agent roles, handoff protocols, or shared state between agents working on related sub-tasks.

The `Brief` struct is monolithic: one goal, one deliverable. A multi-agent workflow would need a brief that decomposes into sub-briefs with dependency relationships.

---

### 10. The "what I don't want" problem -- Severity: Medium

Developers spend significant prompt energy on negative constraints: "don't use ORM magic," "don't add a new dependency for this," "don't refactor the existing tests." The brief format bundles these into Hard constraints alongside positive constraints, but negative intent is qualitatively different from positive intent.

Negative constraints often come from past bad experiences ("last time the agent added lodash for a single function"), and they accumulate across sessions. There is no mechanism for persistent negative preferences that carry across briefs -- each new brief starts from scratch.

---

## Part 2: What Information Is Lost in Translation

When a developer has an idea and reduces it to a `.brief.md`, the following categories of information are systematically lost:

### Tacit architectural knowledge

The developer knows that module A communicates with module B through an event bus, that the team tried a direct coupling last year and reverted it, and that the event bus has a 50ms latency SLA. None of this is in any single file. It lives in the developer's head, distributed across commit messages, PR discussions, and Slack threads. The `context` field can point at architecture docs, but most codebases have outdated or missing architecture docs. The real architecture lives in the code and in people's heads.

### Emotional and social signals

- "This is my first week on this codebase, I'm not confident in my understanding" -- signals the agent should be more explanatory, more cautious
- "I've been debugging this for 3 hours and I'm frustrated" -- signals the agent should be direct, not exploratory
- "My tech lead will review this PR" -- signals the agent should optimize for reviewability, not just correctness
- "This is a hackathon project" -- signals the agent can cut corners, prioritize speed

### The shape of the solution space

The developer often knows the *category* of solution they want but cannot specify it precisely. "I want something like a middleware, not a decorator" or "this should be a data transformation, not a stateful service." These are high-signal, low-cost hints that dramatically narrow the solution space, but they do not fit neatly into any brief section.

### Negative examples and anti-patterns

"The last time I asked an agent to do this, it created 47 files and a factory-of-factories pattern. I want the opposite of that." This is extremely valuable signal that is routinely lost.

### Organizational context

Who else is working on adjacent code? What is the deploy cadence? Is there a feature flag system? What is the testing culture (TDD vs. test-after vs. no tests)? What is the PR review process? These are not per-task signals -- they are ambient project context that shapes every task.

### The developer's theory of the bug/problem

When a developer says "fix the login bug," they usually have a hypothesis: "I think it's the session expiry logic." This hypothesis is valuable even when wrong, because it tells the agent where the developer has already looked and what their mental model of the system is. The brief format has no designated place for "my current hypothesis" vs. "the actual goal."

---

## Part 3: The "60-Second Brief" Ceiling

The current target is 60-second authoring. Here is what each time horizon enables:

### 60 seconds (current)

Fill in a pre-scaffolded template with goal, a few constraints, and key sacred paths. Works for well-understood tasks in familiar codebases. Falls apart for exploratory tasks or unfamiliar codebases.

### 30 seconds -- the "voice-note brief"

What if the developer could *speak* their intent and the tool parsed it? "I need to fix the login timeout, the bug is probably in session.rs, don't touch the auth middleware, and I need it backward compatible with v2." A natural-language parser (LLM-powered) could extract:

- Goal: fix login timeout
- Hypothesis: session.rs
- Sacred: auth middleware
- Hard constraint: v2 backward compatibility

The tool would generate the `.brief.md` and ask the developer to confirm/edit the 2-3 things it is unsure about.

### 10 seconds -- the "git-message brief"

`brief "fix login timeout, probably session.rs, keep v2 compat"`

A single command-line string, parsed with LLM assistance into structured fields. The tool infers the rest from the codebase: it scans for session-related files, checks the existing `.brief.md` for persistent sacred regions, pulls constraint history from past briefs. The developer reviews a 3-line summary and hits enter.

### 5 seconds -- the "just do it" brief

`brief --from-issue 47`

The tool reads the issue, reads the codebase, reads past briefs, and generates a complete brief. The developer reviews and approves with a single keystroke. At this point, the tool is essentially doing the human's job of *thinking about what they want*, and the human's role shifts from *authoring intent* to *approving inferred intent*.

### Sub-5 seconds -- the "ambient brief"

The tool watches the developer's editor activity, git history, and issue tracker. When the developer opens an issue and starts reading related code, the tool pre-generates a brief in the background. By the time the developer is ready to invoke the agent, the brief is waiting. The developer glances at it, adjusts one constraint, and launches.

### The key insight

As authoring time decreases, the tool must *infer more* and *ask less*. This is a classic explore/exploit tradeoff. The tool needs a model of the developer's persistent preferences, the project's ambient constraints, and the current task context -- all of which must be maintained across sessions. The current `brief init` in init.rs is a one-shot inference at project setup time. The path toward sub-10-second briefs requires *continuous* inference.

---

## Part 4: Asymmetric Information Problems

### What the human knows that the model does not

| Signal | Current expressibility | Gap |
|--------|----------------------|-----|
| Why this task matters now | None | No urgency/priority field |
| What they already tried | None | No "prior attempts" section |
| Their confidence in their own understanding | None | No confidence/uncertainty signal |
| Organizational politics ("don't refactor Jane's module") | Awkwardly via Sacred | No social-context mechanism |
| The visual/UX they want | None (text only) | No mockup/screenshot attachment |
| Their mental model of the system | Partially via context files | No "my understanding is X" field |
| How much time/budget they have for this | None | No scope-bounding field |

### What the model knows that the human does not

| Signal | Currently surfaced? | Gap |
|--------|-------------------|-----|
| The brief is ambiguous in specific ways | No | Agent cannot flag ambiguities pre-execution |
| A constraint contradicts the goal | No | No pre-flight conflict detection |
| The assumed architecture does not match the actual code | No | No assumption-vs-reality checking |
| A similar task was solved in this codebase before | No | No pattern/precedent search |
| The sacred region is already broken/inconsistent | No | Validation only checks existence, not integrity |
| Better approaches exist that the brief does not consider | No | No "counter-brief" or suggestion mechanism |

### The mediation opportunity

A tool sitting between the human and the model could:

1. **Pre-flight analysis.** Before emitting, the tool could run the brief against the codebase and flag: "Your constraint says backward compatibility, but I found 3 deprecated endpoints in the v2 API that have no tests. Want to address this?" This turns the brief from a one-way document into a conversation.

2. **Ambiguity scoring.** Rate each section of the brief for ambiguity. "Your goal is clear. Your hard constraints are specific. Your deliverable is vague -- consider adding acceptance criteria." This gives the developer targeted feedback on where to invest more authoring time.

3. **Counter-briefing.** After parsing the brief, the tool generates a "model's understanding" summary: "Here's what I think you're asking me to do: [summary]. Here's what I'm unsure about: [list]. Here's what I think you might be missing: [list]." The developer reviews this in 10 seconds and the effective communication quality doubles.

4. **Persistent preference learning.** Track patterns across briefs. If the developer always marks `migrations/` as sacred, always adds "no new dependencies" as a hard constraint, always specifies async patterns -- build these into a `.brief-profile` that pre-populates every new brief.

---

## Part 5: The Feedback Loop Problem

### The current feedback loop is catastrophically slow

1. Developer writes brief (60 seconds)
2. Developer runs `brief emit claude` and feeds to agent (10 seconds)
3. Agent executes for 2-30 minutes
4. Developer reviews output (5-30 minutes)
5. Developer realizes the brief was under-specified or wrong
6. Developer edits brief and repeats from step 2

This is a 10-40 minute feedback loop. For comparison, a compiler error loop is 5-30 seconds. The brief tool needs to move feedback *before* agent execution, not after.

### What a tight feedback loop looks like

**Layer 1 -- Instant (0-2 seconds): Structural validation.**
This exists today via `brief validate`. It checks format correctness, sacred path existence, checkbox syntax, etc. This is necessary but insufficient -- you can have a perfectly valid brief that communicates the wrong thing.

**Layer 2 -- Fast (2-10 seconds): Semantic pre-flight.**
Before emitting, the tool should:
- Parse the goal and check if it is actionable ("redesign the pipeline" is vague; "add backpressure to the event consumer in src/pipeline/consumer.rs" is actionable)
- Check constraints for internal contradictions
- Verify that the scope implied by the goal matches the files referenced in context/sacred
- Estimate the complexity of the task relative to the constraints (are you asking for a 3-day task with 20 hard constraints? that is probably over-constrained)

**Layer 3 -- Medium (10-30 seconds): Dry-run preview.**
The tool could invoke the model with a meta-prompt: "Given this brief, what is your execution plan? List the files you would modify, the approach you would take, and any clarifying questions you have." The developer reviews a 10-line plan, not 500 lines of code. If the plan is wrong, the brief is wrong, and the developer can fix it in 10 seconds instead of waiting for full execution.

**Layer 4 -- Progressive (during execution): Live constraint checking.**
As the agent works, continuously verify that its actions comply with the brief's constraints. If the agent is about to modify a sacred file, interrupt immediately. If the agent is drifting from the stated goal, flag it. This turns the brief from a pre-flight checklist into a continuous guardrail.

**Layer 5 -- Post-execution (seconds after completion): Brief-vs-output diff.**
After the agent finishes, automatically compare the output against the brief. Were all hard constraints satisfied? Were sacred regions untouched? Were assumptions tested? Generate a compliance report that the developer can review in 15 seconds. This also feeds back into the brief: "Your assumption about synchronous DB writes was invalidated by the agent's analysis. Update the brief?"

### The compound effect

If you stack all five layers, the developer gets feedback at every stage. The brief becomes a living document that evolves during the task, not a static input that is written once and hoped to be sufficient. The 60-second authoring investment pays off across the entire execution lifecycle instead of being a one-shot gamble.

---

## Summary: Top 5 Opportunities by Impact

1. **Pre-flight semantic analysis** (counter-briefing / ambiguity scoring). Moves feedback before execution. Transforms the tool from "format validator" to "communication quality analyzer." This is the single highest-leverage feature because it shortens the feedback loop from 30+ minutes to under 30 seconds.

2. **Persistent developer/project profiles.** A `.brief-profile` or `.brief-defaults` file that accumulates preferences, sacred regions, and constraints across sessions. Cuts authoring time from 60 seconds toward 10 seconds for repeat users. The tool already detects stack and sacred paths at init time; extending this to learn from brief history is a natural evolution.

3. **LLM-assisted brief generation from minimal input.** Accept a one-line description plus the codebase and generate a full brief for review. This is the path from 60-second to 5-second authoring. The `init` command already scaffolds structure; this adds content inference.

4. **Temporal context and session memory.** A "Prior Attempts" or "History" section that carries forward what was tried, what failed, and what the developer learned. Could be auto-populated from git history of the `.brief.md` file itself.

5. **Dry-run / plan preview.** Before full execution, ask the agent to produce a 10-line plan from the brief. Developer reviews the plan, not the code. Cuts the feedback loop from 30 minutes to 30 seconds for catching brief-level errors.
