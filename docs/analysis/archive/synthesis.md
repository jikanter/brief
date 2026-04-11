# Team Lead Synthesis: Critical Feature Recommendations

**Date:** 2026-03-20
**Analysis performed by:** Three parallel AI agents (Claude Opus 4.6) operating as specialized roles:
- **Context Architect** — format expressiveness and completeness ([full report](./context-architect.md))
- **Context Engineer** — technical pipeline flexibility and extensibility ([full report](./context-engineer.md))
- **Integration Engineer** — ecosystem integration and real-world workflows ([full report](./integration-engineer.md))

---

## Diagnosis

The brief tool nails the **authoring problem** — fast, Markdown-native, zero learning curve. But it is fundamentally **advisory, not operational**. When an AI agent reads the emitted output, constraints are prose suggestions, not enforceable boundaries. The format captures approximately 30-40% of what an advanced agent actually needs to work effectively.

---

## Scoring Summary

| Dimension | Score | Key Bottleneck | Evaluator |
|-----------|-------|----------------|-----------|
| Format Expressiveness | 5/10 | Flat constraints, no scope/metadata | Context Architect |
| Parser Robustness | 6/10 | Loses nested structure, code blocks | Context Engineer |
| Emitter Fidelity | 6/10 | Hand-coded, no trait abstraction | Context Engineer |
| Ecosystem Coverage | 3/10 | Claude-only, no MCP/Copilot/Cursor | Integration Engineer |
| Composition/Scaling | 2/10 | Single brief, no inheritance | All three |
| Enforcement | 2/10 | Advisory only, no runtime gates | Integration Engineer |
| **Overall** | **4/10** | **Architecturally sound MVP, needs Phase 2** | Consensus |

---

## The 5 Most Critical Features

### 1. MCP Server — Runtime Enforcement Bridge

**Flagged by:** All three agents. Integration Engineer ranked it #1.

Today brief emits text that agents read passively. It needs to expose tools agents can invoke at runtime:

- `check_sacred_path(filepath)` → `{is_sacred, reason}`
- `get_constraints(type?)` → `[{text, severity, scope}]`
- `log_decision(action, constraint_ref, rationale)` → ack
- `get_briefing()` → Brief as JSON

This transforms brief from documentation into infrastructure. Multiple agents (Claude Code, Copilot, custom tools) query the same source of truth. Sacred regions become enforceable gates, not advisory notes.

**Why it matters:**
- Context Architect: "Brief assumes a single recipient agent — no mechanism for shared state"
- Context Engineer: "No separation between briefing content and runtime behavior"
- Integration Engineer: "If brief exposed itself as an MCP server, all agents could invoke `check_sacred_path` and get a consistent answer"

---

### 2. Rich Constraint Model — From Strings to Structured Types

**Flagged by:** Context Architect (expressiveness gaps) and Context Engineer (data model score 4/10).

Current constraints are `Vec<String>` — flat, unscoped, no metadata. Needed:

```rust
pub struct Constraint {
    pub text: String,
    pub scope: Option<Vec<String>>,     // glob patterns this applies to
    pub category: Option<Category>,      // Security, Performance, Compatibility
    pub verification: Option<String>,    // command to check compliance
    pub risk_level: Option<RiskLevel>,
}
```

Plus a dedicated `## Anti-Patterns` section separating negative constraints from positive ones.

**Why it matters:**
- Context Architect: "Cannot distinguish 'Do not break tests' (behavioral) from 'Use async/await' (stylistic) from 'Must support 10k users' (non-functional)"
- Context Engineer: "Emitters cannot provide intelligent prioritization or nested explanations"
- Integration Engineer: "Constraint enforcement as tools requires structured constraint data"

---

### 3. Cross-Ecosystem Emit Targets

**Flagged by:** Integration Engineer as the adoption multiplier.

Missing targets with large user bases:
- `.github/copilot-instructions.md` — GitHub Copilot
- `.cursorrules` — Cursor (millions of DAUs)
- `.windsurfrules` — Windsurf
- `.aider.conf.yml` — Aider

A single `brief sync` command writing all detected targets from one `.brief.md` source of truth.

**Why it matters:**
- Integration Engineer: "Without cross-IDE emission, brief becomes a tool for Claude Code users, not a tool for AI-assisted development"
- Context Engineer: "All emitters are hand-coded string builders — no trait abstraction makes adding targets expensive"
- Context Architect: "Emit targets have wildly different demands but no way to express target-specific content"

---

### 4. Brief Composition & Inheritance

**Flagged by:** All three agents as the scaling bottleneck.

Current state: one `.brief.md` per project, no inheritance. Needed:

```yaml
---
extends: ../.brief.md   # org-level constraints inherited
stack: [Rust, Redis]     # additive to parent
---
```

Enables org-wide sacred regions (auth, compliance, migrations) flowing down to every service. Updating a policy constraint propagates automatically.

**Why it matters:**
- Context Architect: "No mechanism for shared constraints across related tasks"
- Context Engineer: "Composition scored 2/10 — doesn't scale beyond single project"
- Integration Engineer: "Monorepo support is essential for enterprise adoption"

---

### 5. Verification & Validation Depth

**Flagged by:** Context Architect (verification criteria gap) and Context Engineer (validation score 5/10).

Current deliverable is freeform text. Needed:

```markdown
## Deliverable
- [ ] Tests pass: `cargo test`
- [ ] Coverage >= 80%: `cargo tarpaulin`
- [ ] No sacred violations: `brief check-diff main`
- [ ] Constraint compliance: `brief verify`
```

Plus `brief verify` command running verification commands and `brief validate-diff <branch>` checking git diffs against sacred regions.

**Why it matters:**
- Context Architect: "Agents deliver code but have no automated way to verify constraint compliance"
- Context Engineer: "Validation catches format errors but not completeness errors — no semantic checks"
- Integration Engineer: "Teams have no way to enforce brief constraints in CI — everything remains advisory"

---

## Implementation Roadmap

| Phase | Feature | Effort | Impact |
|-------|---------|--------|--------|
| **2a** | Cross-IDE emit targets (Copilot, Cursor) | Small | High — unlocks adoption |
| **2a** | `brief validate-diff` + pre-commit hook | Small | High — enables CI enforcement |
| **2b** | Rich constraint model (scope, category, verification) | Medium | High — enables smart emission |
| **2b** | Structured deliverable with acceptance criteria | Medium | High — closes verification loop |
| **2c** | Brief composition/inheritance (`extends:`) | Medium | High — enables scaling |
| **2c** | MCP server prototype | Large | Transformative — enables multi-agent |
| **2d** | `brief verify` command | Medium | Medium — automated compliance |
| **2d** | Emitter trait abstraction | Medium | Medium — reduces extension cost |

---

## Agent Perspectives — Bullet-Point Breakdown

### Context Architect's View
- The 60-second authoring target conflicts with completeness — works for narrow tasks, fails for complex/exploratory ones
- The format confuses "what you want built" with "what the agent needs to work" — captures goals and constraints but not diagnosis, verification, or calibration
- The `UnknownSection` extensibility mechanism is brilliant but undiscoverable — needs published conventions
- Recommends a hybrid authoring model: quick template with optional deep sections (Diagnosis, TeamContext, Verification)
- Identified 10 missing context categories, 4 expressiveness limits, and 8 systematic context gaps

### Context Engineer's View
- Architecture is clean but hits a wall at medium scale — the data model is the bottleneck
- Constraints as flat strings make semantic-aware emission impossible
- Parser loses structure (nested lists, code blocks, emphasis) that emitters need
- All 5 emitters are hand-coded string builders with no shared abstraction — adding targets or fields is expensive (touches 6+ files)
- Composition scored 2/10 — no inheritance, no templates, no org-wide policy mechanism
- Recommends: emitter trait abstraction, AST-based parsing, constraint metadata struct, conflict detection engine

### Integration Engineer's View
- Claude Code integration is shallow — emits readable CLAUDE.md but misses permissions, hooks, MCP, slash commands
- MCP integration is the single biggest gap — transforms brief from documentation into queryable infrastructure
- CI/CD enforcement is ad hoc — `brief check` exists but no pre-commit framework, no GitHub Actions templates, no diff-based validation
- Cross-IDE coverage is the adoption gate — Copilot, Cursor, Windsurf, Aider users are locked out
- Init auto-detection misses frameworks, cloud infra, test frameworks, CI/CD configs
- Recommends: MCP server first, then cross-IDE targets, then CI/CD integration

---

## Core Insight

All three evaluations converge on the same conclusion: **brief's format is the product, but the format needs to become executable, not just readable.** The gap between "an agent can read this" and "an agent can enforce this at runtime" is where the most value lives.
