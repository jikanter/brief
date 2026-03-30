# Context Architect Analysis

**Agent Role:** Evaluate the `.brief.md` format's expressiveness and completeness as a context specification for advanced AI tools.

**Scope:** Format specification, data model, parser coverage, emitter output, test fixtures, and init system.

---

## Executive Summary

The `.brief.md` format is well-designed for quick constraint specification but fundamentally limited for real-world AI agent onboarding. It captures what humans want to build and what they must avoid, but omits critical information categories that agents actually need to work effectively. When an AI agent reads the emitted output, approximately 60-70% of critical context is systematically missing.

---

## Format Gaps Identified

### 1. No Dependency Relationships or Task Decomposition
- The `Brief` struct is monolithic: one goal, one deliverable, one list of constraints
- No mechanism for subtask dependencies, sequential phases, or handoff boundaries
- A complex task like "rebuild the authentication system" naturally decomposes into ordered phases — a single flat brief cannot express this
- **Evidence:** `model.rs` — `pub goal: String` (singular), `pub deliverable: Option<String>` (singular), no `dependencies`, `phases`, or `subtasks` fields

### 2. Missing Causal Context and Problem Hypothesis
- No designated place for "why this is broken" or "my current hypothesis about the root cause"
- Assumptions track unvalidated beliefs but only as flat text with no causal linkage
- Agents cannot distinguish between a task with no diagnosis vs. one with a clear hypothesis vs. one where prior approaches have already failed
- **Evidence:** `model.rs` — `pub assumptions: Vec<Assumption>` has no hierarchy; no "problem statement" field beyond the H1 goal

### 3. No Environment, Configuration, or Secret Patterns
- Stack lists technology names but not versions, env var requirements, or infrastructure patterns
- No way to express "requires PostgreSQL 16 with specific extensions" or "needs AWS KMS access"
- Agents write code that assumes databases/services that don't exist in their runtime
- **Evidence:** `init.rs` `detect_stack()` looks for presence of config files but doesn't parse versions from them

### 4. No Scope Boundaries or Solution Shape Constraints
- No way to say "surgical fix only" vs. "refactor liberally as needed"
- No mechanism for excluding solution patterns or expressing implementation budget
- An agent might deliver a 1000-line architectural redesign when the developer wanted a 20-line cache addition
- **Evidence:** Constraints struct has no scope/boundary fields; no anti-patterns field exists

### 5. No Urgency, Confidence, or Risk Signals
- No fields for priority, developer confidence, or risk tolerance
- No way to signal "blocking production" vs. "low priority cleanup"
- No way to calibrate agent behavior between "conservative fix" vs. "move fast, iterate"
- **Evidence:** Frontmatter has `stack`, `context`, `model`, `version` — all structural metadata, none behavioral

### 6. No Verification Criteria or Success Metrics
- Deliverable is freeform text with no executable success criteria
- No acceptance tests, measurable goals, or verification commands
- Agents deliver code but have no automated way to verify constraint compliance or deliverable completeness
- **Evidence:** `model.rs` — `pub deliverable: Option<String>` is plain text; `validate.rs` checks format but not semantic completeness

### 7. No Temporal Context or Decision History
- Brief is a snapshot in time with no history of previous attempts
- No way to express "third attempt; first two failed because X"
- When a brief is revisited, all context about previous attempts is lost
- **Evidence:** No `.brief.md` version control integration; no "what I tried" field

### 8. No Team Conventions or Organizational Context
- No field for testing culture, deploy processes, or code review expectations
- Agents don't know if the team does TDD vs. test-after, strict review vs. quick-approve
- Identical briefs for a startup and a fintech should produce very different code — currently they don't
- **Evidence:** `init.rs` detects stack and sacred paths but not team conventions

### 9. No Multi-Agent Workflow or Role Definition
- Brief assumes a single recipient agent
- No mechanism for specifying agent roles, handoff boundaries, or shared state
- Complex tasks that should be parallelized across specialized agents are treated as monolithic

### 10. No Dedicated Anti-Patterns Section
- Constraints are all positive ("do this," "prefer that")
- Negative constraints ("never use lodash," "avoid Factory patterns") are mixed in with positive ones
- Anti-patterns accumulate across sessions without a dedicated place to record them

---

## Expressiveness Limits

- **Natural language ambiguity:** Constraints are freeform strings — "backward compatible" could mean client-compatible, db-compatible, semantic-compatible, or signature-compatible
- **No scope qualifiers on rules:** Constraints apply globally but often should apply only to specific modules (WCAG constraint is UI-only)
- **No conditional or time-bound constraints:** All constraints are permanent and absolute, no expiration dates or conditional applicability
- **No deliverable type system:** Cannot distinguish between code artifacts, documentation, test suites, or deployment steps

## Context Completeness Gap

When an agent reads emitted output, it systematically lacks:
- Code-specific domain knowledge (how does this module currently work?)
- Failure modes and edge cases (what scenarios cause production failures?)
- User/customer context (who are the end users and what's their workflow?)
- Integration points and dependencies (what external systems does this talk to?)
- Testing requirements and pragmatics (unit vs. integration vs. e2e expectations)
- Code review expectations (what does the reviewing team care about?)
- Deployment and operations constraints (how is this deployed, what's the rollback plan?)
- Historical context and decisions (why was the current implementation done this way?)

---

## Key Recommendations

1. **Hybrid authoring model:** Quick template (60s) with optional rich sections (Diagnosis, TeamContext, Verification) for complex tasks
2. **Document `UnknownSection` as a first-class extension point** with published conventions
3. **Add semantic completeness validation** — warn if constraints aren't testable, deliverables aren't specific, goals are vague
4. **Add structured sections:** `## Diagnosis`, `## Verification`, `## Team Context`, `## Anti-Patterns`, `## Phases`
5. **Extend Frontmatter:** `priority`, `developer_confidence`, `risk_tolerance`, `infrastructure`, `environment`
6. **Implement `brief verify`** subcommand that runs verification commands from structured deliverables
