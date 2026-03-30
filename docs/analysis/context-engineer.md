# Context Engineer Analysis

**Agent Role:** Analyze the technical pipeline (parsing, validation, emission) for flexibility, extensibility, and robustness.

**Scope:** Data model, parser pipeline, validation logic, all emitters, CLI structure, tests, and dependencies.

---

## Executive Summary

The `brief` tool is well-engineered for Phase 1 (MVP) — clean code, tested, functional. However, the architecture hits a wall at medium scale. The data model is too shallow (constraints as strings) to express nuanced requirements, the parser loses structure on every emission, and the emitter architecture is hand-coded with no abstraction layer. The engineering limits are architectural, not implementation quality.

---

## Data Model Limitations (Score: 4/10)

### Constraints Are Semantically Opaque
- Each constraint is a flat `String` — no metadata about scope, category, risk, or dependencies
- Cannot distinguish "Do not break tests" (behavioral) from "Use async/await everywhere" (stylistic) from "Must support 10k concurrent users" (non-functional)
- Emitters cannot provide intelligent prioritization or nested explanations
- **Evidence:** `model.rs` — `Constraints` struct is a fixed 3-bucket system (`hard/soft/ask_first`, all `Vec<String>`)

### Sacred Regions Are Path-Only
- No severity levels (do-not-touch vs. be-careful-when-touching)
- No scope of modifications (read-only vs. test-only vs. new-code-only)
- A sacred `src/auth/**` cannot express "you can add new test files in src/auth/tests/"
- **Evidence:** `SacredEntry` is just `path: String` + `reason: String` + `well_formed: bool`

### No Rich Deliverable Structure
- Single optional string — cannot express acceptance criteria, success metrics, testing requirements
- "Working code with tests" is ambiguous and un-automatable
- **Evidence:** `model.rs` — `pub deliverable: Option<String>`

### Assumptions Lack Context
- No risk level, validation timing, validation criteria, or dependencies between assumptions
- Cannot generate intelligent validation checklists or prerequisite checking

### Model Does Not Support
- Constraint hierarchies or nesting
- Per-file or per-module constraint scopes
- Conditional briefs ("if using PostgreSQL, then X applies")
- Brief inheritance or composition
- Temporal constraints (validity windows)
- Author/reviewer metadata or provenance

---

## Parser Robustness (Score: 6/10)

### Strengths
- Uses `pulldown-cmark` with event streaming and state machine
- Handles H1 goal extraction, H2/H3 section detection, list items, code segments, checkbox syntax
- Preserves unknown sections for extensibility

### Fragile Assumptions & Edge Cases
- **No nested list support** — sub-items under constraints are silently lost; hierarchy flattened
- **Code blocks in constraints broken** — multi-line code blocks within sections not captured
- **Sacred entry parsing too strict** — entries without backtick-wrapped paths marked malformed even when readable (e.g., `- migrations/ — Database migrations`)
- **No inline formatting preservation** — bold, italic, links in constraints are stripped; only code spans preserved
- **Deliverable text collection is naive** — lists become flat text, multiple paragraphs may lose line breaks, tables lost
- **Unknown sections lose Markdown structure** — content captured as plain text strings, not structured lists/paragraphs
- **Frontmatter YAML errors lack line numbers** — malformed YAML produces generic parse errors with no location
- **No validation of path globs during parsing** — invalid glob syntax parsed successfully, caught later in validation

---

## Validation Limitations (Score: 5/10)

### Current Checks
- Stack is non-empty
- H1 goal exists
- Context files exist (warning)
- Sacred entries well-formed (backticks present)
- Sacred globs match files
- Assumptions have checkboxes

### Missing Validations
- **No constraint syntax validation** — empty, duplicate, trivial, or contradictory constraints not detected
- **No assumption consistency checks** — assumptions that contradict sacred regions not caught
- **No goal validation** — extremely vague goals or goal-deliverable contradictions not flagged
- **No cross-field validation** — stack lists Python but deliverable says "npm package" not caught
- **No model-specific validation** — invalid or outdated model identifiers not checked
- **No constraint redundancy detection** — same text in both Hard and Soft sections not warned

---

## Emitter Fidelity (Score: 6/10)

| Emitter | Fidelity | Key Losses |
|---------|----------|------------|
| **JSON** | 100% | None — lossless round-trip |
| **SKILL** | 95% | Unknown sections dropped |
| **CLAUDE** | 80% | Unknown sections not emitted; constraint types relabeled; no explanation of "why" |
| **AGENTS_MD** | 75% | Constraint types merged into single section; lose hard/soft distinction |
| **PROMPT** | 70% | No stack details; deliverable unformatted; no unknown sections |

### Critical Extensibility Bottleneck
- All emitters are hand-coded string builders, not template-based
- Adding a new section type, constraint property, or frontmatter field requires updating **every single emitter** (5 files) plus all tests
- No base emitter trait or template system to reduce duplication

---

## Extensibility Architecture (Score: 4/10)

### Extension Points
- **Unknown sections:** Parsed and preserved, but lose Markdown structure and aren't emitted in text targets
- **Frontmatter:** Has `serde(default)` for easy field addition, but each new field must be handled in all emitters
- **New emit targets:** ~150 lines boilerplate per new emitter, no shared abstraction
- **New constraint types:** Hardcoded to Hard/Soft/AskFirst — adding a new type touches 6 files (~50-line change)
- **New validation rules:** Centralized and easy to extend (cleanest extension point)

---

## Composition & Scaling (Score: 2/10)

- Single `.brief.md` file per project, no inheritance, no composition, no referencing
- Cannot express org-wide policies inherited by project briefs
- Cannot share sacred regions across services
- Cannot template common constraint sets
- Updating a company policy across 100 microservices requires 100 manual edits
- No hierarchical validation (project-level strict, task-level lenient)

---

## Key Recommendations

1. **Introduce `ConstraintMetadata` struct** — scope, category, risk level, examples; unlocks semantic-aware emission
2. **Rich deliverable structure** — acceptance criteria, success metrics, testing requirements, verification commands
3. **Template/composition system** — `extends:` field for brief inheritance; org-wide + project-specific layering
4. **Introduce emitter traits** — `trait Emitter { fn emit(&self, brief: &Brief) -> String; }` to reduce duplication
5. **AST-based Markdown parsing** — build AST from pulldown-cmark events, preserve lists/code/emphasis structure
6. **Conflict detection** — `detect_conflicts(brief: &Brief) -> Vec<Conflict>` for overlapping sacred regions, contradictory constraints
7. **Brief diff improvements** — semantic diff showing which constraints changed, which sacred regions shifted; JSON diff for tooling
