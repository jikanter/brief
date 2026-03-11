# Vision: Brief as Invisible Infrastructure

*Agent: Workflow Integrator*

How brief/flint embeds into real developer workflows -- not as an extra step, but as the connective tissue between human intent and agent execution.

---

## 1. Git-Native: Brief as Part of the Commit/PR Cycle

### The Scenario

Maya is a senior engineer at a fintech company. She opens a GitHub issue: "Migrate payment processor from Stripe v2 to v3 API." She assigns it to herself and starts working with Claude Code.

**Today (without brief):** Maya types a long prompt into Claude Code. She forgets to mention the compliance module is untouchable. Claude rewrites the webhook signature verification. The PR gets rejected in review. Two hours lost.

**With brief integrated into git:**

```
$ git brief start PROJ-1234
```

This single command:
1. Fetches the issue title and description from GitHub
2. Runs `brief init` with the issue context pre-populated
3. Detects the payment module's existing sacred regions from a project-level `.brief.md`
4. Opens the brief in her editor with the goal pre-filled

Maya spends 40 seconds adding one hard constraint ("Do not modify webhook signature verification") and saves. She starts her Claude Code session. The brief is automatically loaded as context.

When she opens a PR, the brief travels with it:

```
$ git brief pr
```

This creates a PR where:
- The `.brief.md` is included as a file in the PR (or embedded in the description)
- CI runs `brief validate` as a check -- verifying sacred regions weren't touched, assumptions are addressed
- Reviewers see not just *what* changed but *what was intended* and *what was off-limits*

The PR review becomes faster because the reviewer can compare the brief's constraints against the diff. "Did the agent respect the boundaries?" is now a mechanical check, not a judgment call.

### The Post-Commit Hook

After Maya's PR merges, a post-merge hook runs:

```
$ brief suggest --from-diff HEAD~1..HEAD
```

This analyzes what just changed and suggests a brief for follow-up work. If the migration touched 3 of 5 payment modules, it drafts a brief for the remaining two, pre-populated with the constraints that worked and the assumptions that were validated. The backlog writes itself.

### What This Requires

- `brief start` command that integrates with issue trackers (GitHub Issues, Linear, Jira) via their APIs
- Project-level `.brief.md` inheritance: a root brief that defines org-wide sacred regions (compliance, auth, migrations) that every task-level brief inherits
- `brief validate --against-diff` mode that checks a git diff against the brief's constraints
- A GitHub Action / CI check that runs validation on PR

---

## 2. IDE Integration: Brief as a Living Sidebar

### The Scenario

Carlos is a mid-level developer using Cursor. He's working on a feature brief: "Add rate limiting to the public API." He has the brief open in a sidebar panel.

**The sidebar shows:**

```
ACTIVE BRIEF: Add rate limiting to the public API

Hard Constraints
  [met]     Do not modify existing endpoint signatures
  [met]     Rate limits must be configurable per-tenant
  [VIOLATED] Response headers must include X-RateLimit-*

Sacred Regions
  src/auth/** -- Auth logic (0 touches)
  src/billing/** -- Billing logic (0 touches)

Assumptions
  [x] Redis is available in all environments
  [ ] Current p99 latency is under 50ms  <-- NEEDS VALIDATION

Progress: 2/3 hard constraints met
```

The "VIOLATED" constraint turns red when Carlos's current working tree has API responses that don't include the rate limit headers. This isn't a linter -- the IDE extension is running `brief check` against the constraint descriptions using a lightweight local model or pattern matching. It's a reminder, not an enforcer.

When Carlos hovers over a sacred file in the file tree, a tooltip shows: "Sacred: Auth logic -- defined in .brief.md". If he tries to open `src/auth/handler.rs`, a subtle banner appears: "This file is in a sacred region. Edits here require approval per your active brief."

### Inline Brief Authoring

Carlos realizes he needs to add a new assumption. He types a slash command in the editor:

```
/brief assume Redis SCAN operations are O(N) for our key patterns
```

This appends `- [ ] Redis SCAN operations are O(N) for our key patterns` to his `.brief.md` without switching files. The sidebar updates immediately.

When the agent session produces evidence that validates an assumption (e.g., a benchmark result), Carlos can mark it validated from the sidebar with one click.

### What This Requires

- VS Code / Cursor extension that watches `.brief.md` for changes
- Constraint status tracking -- mapping constraint text to observable conditions in the working tree (starts simple with sacred region monitoring, grows toward semantic matching)
- Inline brief editing via slash commands or command palette
- Brief-aware file decorations (sacred region indicators in the file tree)

---

## 3. Team Collaboration: Brief Libraries and Brief Reviews

### The Scenario

DevTooling team at a 200-person company. They've been using brief for three months. Patterns have emerged.

**Brief Templates:**

The team maintains a `.brief-templates/` directory in their monorepo:

```
.brief-templates/
  new-api-endpoint.brief.md
  database-migration.brief.md
  frontend-component.brief.md
  security-patch.brief.md
  dependency-upgrade.brief.md
```

When someone runs `brief init --template new-api-endpoint`, they get a pre-filled brief with the team's standard constraints for API work (backward compatibility, OpenAPI spec must be updated, integration tests required) and the standard sacred regions (auth, billing, compliance).

**Brief Reviews:**

The team has adopted a rule: every PR that was AI-assisted must include its brief. The brief review happens *before* the code review. A senior engineer looks at the brief and says: "You're missing an ask-first constraint for database schema changes. Add that before the agent starts." This is cheaper than catching the problem in the code review.

Over time, the team notices patterns:

```
$ brief analytics --last-quarter

Most violated constraints:
  1. "Update OpenAPI spec" (violated in 23/45 PRs)
  2. "Add integration tests" (violated in 18/45 PRs)
  3. "Do not modify shared types" (violated in 12/45 PRs)

Most wrong assumptions:
  1. "Existing tests cover this path" (wrong 8/15 times)
  2. "No breaking API changes" (wrong 5/12 times)

Briefs with best outcomes (merged without revisions):
  - Used template: 78% success rate
  - Freeform: 41% success rate
```

This data feeds back into improving templates. The team promotes "Update OpenAPI spec" from a soft constraint to a hard constraint in the API template. They add a CI check that verifies the OpenAPI spec is updated when API routes change.

### Organizational Sacred Regions

The security team maintains a root-level `.brief.md` with sacred regions that apply to every brief in the repo:

```markdown
# Organization Defaults

## Sacred
- `src/auth/**` -- Security-reviewed authentication. Changes require security team approval.
- `src/compliance/**` -- GDPR/SOC2 audit trail. Changes require compliance sign-off.
- `infrastructure/terraform/**` -- Production infrastructure. Changes require SRE review.
- `**/migrations/*.sql` -- Committed migrations are immutable.
```

Task-level briefs inherit these sacred regions automatically. A developer cannot accidentally un-sacred the auth module in their task brief.

### What This Requires

- Template system: `brief init --template <name>` reads from a templates directory
- Brief inheritance / composition: task briefs inherit from project/org briefs
- `brief analytics` command that aggregates outcome data from merged PRs
- Convention for brief-in-PR (either as a file or as structured PR description metadata)

---

## 4. Multi-Agent Orchestration: Brief Decomposition

### The Scenario

Platform team needs to refactor the notification system from a monolithic service into domain-specific handlers. This touches 4 modules: email, SMS, push notifications, and in-app. Each module has its own database tables, its own external API integrations, and its own test suite.

The tech lead writes a parent brief:

```markdown
---
stack: [Go 1.22, PostgreSQL 16, Redis, AWS SNS, SendGrid, Twilio]
context: [./docs/notification-architecture.md, ./docs/api-contracts.md]
---

# Decompose notification service into domain handlers

## Constraints

### Hard
- All four channels must remain operational throughout migration
- No changes to the public notification API contract
- Each handler must be independently deployable

### Sacred
- `pkg/auth/**` -- Authentication middleware
- `pkg/billing/**` -- Usage tracking for notification costs
- `api/v2/**` -- Public API surface
```

She runs:

```
$ brief decompose --agents 4
```

Brief analyzes the codebase and the parent brief, then generates four sub-briefs:

```
.brief.d/
  email-handler.brief.md
  sms-handler.brief.md
  push-handler.brief.md
  inapp-handler.brief.md
```

Each sub-brief:
- Inherits all sacred regions from the parent
- Adds cross-agent sacred regions: the email handler's brief marks `pkg/sms/`, `pkg/push/`, and `pkg/inapp/` as sacred, and vice versa. No agent can trample another agent's module.
- Gets a scoped goal: "Extract email notification logic into standalone handler with independent deployment configuration"
- Gets scoped constraints relevant to its module
- Includes a coordination section: "Shared types in `pkg/notification/types.go` -- coordinate changes via the parent brief"

Each sub-brief can be handed to a separate agent session. The parent brief tracks overall progress:

```markdown
## Sub-briefs
- [ ] email-handler.brief.md -- assigned
- [ ] sms-handler.brief.md -- assigned
- [x] push-handler.brief.md -- complete, merged in PR #847
- [ ] inapp-handler.brief.md -- in progress
```

### The Conflict Resolution Problem

What happens when two agents need to modify a shared file? The parent brief defines a `## Coordination` section:

```markdown
## Coordination
- `pkg/notification/types.go` -- Shared type definitions. Sub-briefs must not modify.
  Changes here require a separate brief with all four sub-briefs as context.
- `pkg/notification/router.go` -- Routing logic. Will be modified in a final
  integration brief after all handlers are extracted.
```

This is cooperative, not enforced -- but it gives agents clear instructions about boundaries. Combined with the cross-agent sacred regions, accidental conflicts drop dramatically.

### What This Requires

- `brief decompose` command that analyzes codebase structure and generates sub-briefs
- Sub-brief / parent-brief relationship model (the `.brief.d/` directory convention)
- Cross-agent sacred region generation (each sub-brief protects the others' modules)
- Progress tracking in the parent brief
- This is where brief starts becoming an orchestration layer, not just a file format

---

## 5. Session Continuity: "Continue Where We Left Off"

### The Scenario

Priya is three hours into a complex refactoring session with Claude Code. She's been working on converting a callback-based Node.js module to async/await. She's completed 4 of 7 files. Her laptop battery dies.

**Today:** She reopens Claude Code. "Continue the async/await migration." The agent has no memory. She spends 20 minutes re-establishing context: which files are done, which approach she chose for error handling, why she decided to keep one specific callback pattern in the legacy adapter.

**With brief session tracking:**

Brief has been silently updating a session log alongside the `.brief.md`:

```
.brief-sessions/
  2026-03-09T14-30-00.session.yaml
```

```yaml
brief: .brief.md
started: 2026-03-09T14:30:00Z
last_activity: 2026-03-09T17:15:00Z
status: interrupted

progress:
  - file: src/handlers/auth.js
    status: complete
    approach: "Converted all callbacks to async/await. Used try/catch wrapping."
  - file: src/handlers/payment.js
    status: complete
    approach: "Kept callback pattern for Stripe webhook handler (legacy adapter constraint)."
  - file: src/handlers/notification.js
    status: complete
    approach: "Full conversion. Added Promise.allSettled for multi-channel send."
  - file: src/handlers/search.js
    status: in_progress
    approach: "Started conversion. Elasticsearch client already supports promises."
  - file: src/handlers/analytics.js
    status: pending
  - file: src/handlers/export.js
    status: pending
  - file: src/handlers/legacy-adapter.js
    status: skipped
    reason: "Hard constraint: legacy adapter must maintain callback interface."

decisions:
  - "Error handling: wrap each handler in try/catch, propagate with custom AppError class"
  - "Testing: update each test file immediately after converting its source"
  - "Legacy adapter stays callback-based per hard constraint"

assumptions_updated:
  - text: "Elasticsearch client supports native promises"
    was: unvalidated
    now: validated
    evidence: "Confirmed in @elastic/elasticsearch@8.x docs"

failed_approaches:
  - file: src/handlers/payment.js
    attempted: "Full async conversion of Stripe webhook handler"
    failed_because: "Stripe SDK v2 webhook verification requires synchronous signature check"
    resolution: "Kept callback pattern, wrapped in promise only for internal callers"
```

Priya powers her laptop back up and runs:

```
$ brief resume
```

This command:
1. Finds the most recent interrupted session
2. Generates a context prompt that includes the full session state
3. Feeds it to her agent session

The agent immediately knows: "I see we completed 3 files, started on search.js, and have 3 remaining. The approach is async/await with try/catch wrapping. The legacy adapter stays callback-based. Let me continue with search.js where we left off."

No re-explaining. No lost decisions. No repeated mistakes.

### What This Requires

- Session logging: brief tracks agent activity during a session (file changes, decisions made, approaches tried)
- This could work via MCP (Phase 2) -- the `log_decision` tool already planned would feed session state
- `brief resume` command that reconstructs context from session logs
- Session log format design (YAML is shown above but could also be append-only structured log)
- Privacy consideration: session logs contain work-in-progress thinking. They should be `.gitignore`d by default but optionally committable for team knowledge sharing

---

## 6. Pipeline Integration: Brief as Universal Adapter

### The Scenario: From Slack to Shipped Code

Product manager drops a message in Slack:

> We're getting reports that the dashboard is slow for customers with >10k transactions. Can someone look into this? The transactions table might need an index. Don't touch the reporting module though, finance team is mid-audit.

An engineer reacts with a `:brief:` emoji. A Slack bot responds:

```
Generated brief from thread:

---
stack: [auto-detect]
context: [./docs/database-schema.md]
---

# Investigate and fix dashboard performance for high-transaction customers

## Constraints
### Hard
- Do not modify the reporting module (finance team mid-audit)
### Ask First
- Database schema changes (adding indexes)

## Sacred
- `src/reporting/**` -- Finance team mid-audit

## Assumptions
- [ ] Bottleneck is the transactions table query
- [ ] An index on transactions table will resolve the performance issue
- [ ] Customers with >10k transactions are the affected population

## Deliverable
Dashboard loads within acceptable performance threshold for customers
with >10k transactions.
```

The engineer reviews, tweaks one constraint, and clicks "Create Issue + Brief." A GitHub issue is created with the brief attached. An agent picks it up.

### The Scenario: CI as Brief Enforcer

The CI pipeline includes a brief validation step:

```yaml
# .github/workflows/brief-check.yml
name: Brief Validation
on: [pull_request]

jobs:
  brief-check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: jikanter/brief-action@v1
        with:
          command: validate-pr
```

`validate-pr` does:
1. Finds the `.brief.md` associated with this PR (in the PR branch, or linked in the PR description)
2. Runs `brief validate` -- checks that context files exist, sacred globs resolve
3. Runs `brief check` against every file in the PR diff -- flags if sacred regions were touched
4. Posts a PR comment summarizing constraint compliance

### The Scenario: Brief from Issue Templates

GitHub issue templates generate briefs automatically:

```yaml
# .github/ISSUE_TEMPLATE/feature.yml
name: Feature Request (Brief-enabled)
body:
  - type: input
    id: goal
    label: What should be accomplished?
  - type: textarea
    id: hard_constraints
    label: What must NOT happen?
  - type: textarea
    id: ask_first
    label: What needs approval before proceeding?
  - type: textarea
    id: sacred
    label: What code/files should not be touched?
```

When the issue is created, a GitHub Action runs `brief generate --from-issue` and commits a `.brief.md` to a feature branch, ready for an agent to pick up.

### What This Requires

- Slack integration (bot that watches for reactions or slash commands, calls an NLP-to-brief converter)
- GitHub Action that runs brief validation on PRs
- `brief generate --from-issue` command that reads GitHub issue structured fields
- `brief validate-pr` command that cross-references a brief against a PR diff
- Brief-to-issue and issue-to-brief bidirectional mapping

---

## 7. Feedback and Learning: The Brief Corpus

### The Scenario

Six months in, the platform team has 340 merged PRs with associated briefs. A new engineer, Jin, joins the team. He's assigned to add a new payment method to the checkout flow.

```
$ brief init --template new-payment-method
```

The template isn't just a static file. It's been *refined* by the outcomes of the 12 previous payment-related briefs:

```markdown
---
stack: [TypeScript, PostgreSQL 16, Stripe]
context: [./docs/payment-architecture.md, ./docs/pci-compliance.md]
---

# Add [PAYMENT_METHOD] to checkout flow

## Constraints

### Hard
- PCI DSS compliance: no raw card data in application logs
- All payment flows must be idempotent (learned from PR #234 incident)
- Stripe webhook handlers must validate signatures before processing
  (learned: PR #189 skipped this, caused production incident)

### Soft
- Prefer the PaymentIntent API over legacy Charges API
- Use existing PaymentMethod base class rather than creating new abstractions

### Ask First
- Changes to the checkout UI component tree
- New environment variables (require DevOps review)
- Database schema changes (require DBA review)

## Sacred
- `src/auth/**` -- Authentication
- `src/compliance/**` -- PCI audit trail
- `src/payments/stripe-webhook-verify.ts` -- Signature verification, security-reviewed
  (added after PR #189 incident)

## Assumptions
- [ ] Stripe supports [PAYMENT_METHOD] in current API version
- [ ] Existing test fixtures cover the checkout flow being modified
- [ ] No rate limiting concerns for the new payment method's verification calls
```

Notice the "(learned from PR #234 incident)" annotations. These came from the feedback loop: when a brief led to a problem, the team annotated the template with the lesson. Future briefs inherit organizational memory.

### Brief Scoring

After each PR merges, the brief gets a lightweight outcome score:

```
$ brief close --outcome success --notes "Merged cleanly, no revision needed"
```

Over time, this builds a corpus:

```
$ brief insights

Patterns in successful briefs (>80% of the time):
  - Had 2+ hard constraints (vs. 0-1 in failed briefs)
  - Referenced specific context files (not just README)
  - All assumptions were validated before deliverable was marked complete

Patterns in failed briefs:
  - Vague goal statements ("improve performance" vs "reduce p99 latency below 200ms")
  - Missing sacred regions for shared infrastructure code
  - Assumptions about external APIs that weren't validated early
```

This is organizational knowledge capture. Not in a wiki that nobody reads, but embedded in the tool that everyone uses every day.

### What This Requires

- `brief close` command that records outcome metadata
- Outcome storage (could be as simple as a YAML file alongside the brief, or a lightweight SQLite database)
- `brief insights` command that aggregates patterns from the corpus
- Template refinement workflow: promoting lessons from specific briefs into shared templates
- Eventually: an LLM-powered template generator that synthesizes the corpus into better defaults

---

## 8. The "Flint" Angle: From Briefing File to Execution Spark

Brief is a file format and a CLI. Flint is what happens when you take the core insight -- structured human intent as the input to agent execution -- and expand it beyond Markdown files.

### Flint as a Verb

"Flint it" becomes the developer shorthand for: take this vague intent and turn it into a structured, executable plan.

A developer mutters "we need to fix the search ranking" and a colleague says "flint it" -- meaning: write a brief, define the constraints, mark the sacred regions, state your assumptions. Don't just tell the agent to "fix search." Give it structure.

### Flint as a Protocol

What if the `.brief.md` format becomes a protocol that tools speak?

```
GitHub Issue --> flint --> .brief.md --> agent execution --> PR
Slack thread --> flint --> .brief.md --> agent execution --> PR
Voice memo  --> flint --> .brief.md --> agent execution --> PR
Jira ticket --> flint --> .brief.md --> agent execution --> PR
```

Flint is the universal translator between "human intent in any format" and "structured agent instruction." The input side is infinitely flexible (text, voice, issue trackers, chat). The output side is a single structured format that every agent knows how to consume.

### Flint as an Intelligence Layer

The file format is the foundation. But flint grows into something more:

**Intent compiler.** You provide a goal and flint infers constraints from context. "Add rate limiting to the API" + analysis of the codebase = hard constraint: "existing endpoints must not change signature" + sacred region: auth module + assumption: "Redis is available."

**Execution monitor.** Flint watches the agent work (via MCP tool calls) and tracks constraint compliance in real-time. When the agent is about to touch a sacred file, flint warns before the edit happens, not after.

**Outcome predictor.** After enough data: "This brief has characteristics similar to 15 previous briefs. 12 succeeded. The 3 that failed all had unvalidated assumptions about external API compatibility. Consider validating assumption #2 before proceeding."

**The minimal product vision for flint:**

```
$ flint "migrate the payment processor to Stripe v3"
```

One command. Flint:
1. Analyzes the codebase (stack detection, dependency analysis)
2. Infers constraints from code patterns (existing tests suggest backward compatibility matters; compliance directory suggests sacred regions)
3. Generates a `.brief.md` with smart defaults
4. Opens it in the editor for human review (the 30-second checkpoint)
5. After approval, emits to the active agent runtime
6. Monitors execution
7. Captures the outcome for future learning

The brief file is still there -- it's the artifact of human review, the audit trail, the thing that lives in git. But the experience is: speak your intent, review the structure, let it execute.

### The Product Spectrum

```
brief (today)          --> file format + CLI
brief (6 months)       --> git integration + CI checks + IDE sidebar
flint (12 months)      --> intent compiler + multi-agent orchestration
flint (24 months)      --> organizational knowledge graph of intent, constraints, outcomes
```

The key transition: brief is a tool you use. Flint is infrastructure you build on.

---

## Summary of Required Capabilities by Phase

### Phase 1.5 (Near-term, extends current CLI)
- `brief start` -- create brief from issue tracker context
- `brief resume` -- reconstruct session from logs
- `brief close` -- record outcome
- `brief suggest` -- generate follow-up brief from git diff
- Template system (`brief init --template`)

### Phase 2 (MCP + Integrations)
- MCP server (already planned): `get_briefing`, `check_path`, `log_decision`, `get_constraints`
- Session logging via MCP tool calls
- GitHub Action for PR validation
- VS Code extension (sidebar, sacred region decorations)

### Phase 3 (Composition + Orchestration)
- Brief inheritance (project/org defaults + task overrides)
- `brief decompose` -- split brief into sub-briefs for parallel agents
- Cross-agent sacred regions
- Parent brief progress tracking

### Phase 4 (Intelligence + Flint)
- Intent inference from natural language + codebase analysis
- `brief analytics` / `brief insights` from outcome corpus
- Outcome prediction
- Real-time constraint monitoring during agent execution
- Multi-input adapters (Slack, voice, issue trackers)

---

## The Core Bet

Every tool in this vision document rests on a single thesis: **the constraint on AI-assisted development is not the capability of the agent but the quality of the instructions it receives.** Improving instruction quality by even 20% -- through structure, validation, organizational memory, and reduced authoring friction -- compounds into dramatic productivity gains.

Brief is the file format that proves this thesis. Flint is the product that scales it.
