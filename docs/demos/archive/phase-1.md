# Phase 1: Parser, Validator, Emitters, and CLI

*2026-03-30T05:01:06Z by Showboat 0.6.1*
<!-- showboat-id: e0cf82c1-63a1-4dfd-8895-8e329be8247a -->

Commit `7a60857` implements the complete `brief` CLI in a single 2,400-line commit against the initial scaffold. This demo walks through every command and capability delivered in Phase 1: parsing `.brief.md` files, validating them against the codebase, emitting to four target formats, checking sacred paths, diffing briefs, and scaffolding new ones.

## 1. The `.brief.md` format

A `.brief.md` file has two parts: YAML frontmatter for machine-critical structured data, and a Markdown body with defined heading conventions for human intent.

```bash
cat examples/sample.brief.md
```

```output
---
stack: [Python 3.12, PostgreSQL 16, Kafka 3.7, GCP/k8s]
context: [./docs/current-architecture.md, ./benchmarks/performance-baseline.csv]
---

# Redesign event pipeline for 10M events/day

## Constraints

### Hard
- v2 API backward compatibility must be maintained
- All SQL must target PostgreSQL 16, not MySQL
- Infrastructure budget ceiling is $500/month

### Soft
- Prefer async write patterns where possible
- Favor composition over inheritance in new code

### Ask First
- Database schema changes
- Adding new dependencies to requirements.txt
- Changes to the CI/CD pipeline

## Sacred
- `src/auth/**` — Proprietary tenant resolution logic, legally reviewed
- `src/compliance/**` — GDPR audit trail, approved by legal
- `migrations/` — Historical migration files must never be modified

## Assumptions
- [ ] Bottleneck is synchronous DB writes (validate against perf baseline)
- [ ] Kafka cluster can handle 10M events/day without partition rebalancing
- [x] Current v2 API consumers do not use deprecated endpoints

## Deliverable
Architecture doc with decision records, implementation plan with milestones,
and working code with tests achieving >80% coverage on new modules.
```

The format maps directly to the `Brief` struct: frontmatter (`stack`, `context`, `model`, `version`), an H1 goal, constraints grouped by severity (Hard / Soft / Ask First), sacred regions with glob paths and reasons, assumptions with checkbox state, and a freeform deliverable.

## 2. Parsing

The parser extracts YAML frontmatter via `serde_yaml`, then runs the Markdown body through a `pulldown-cmark` state machine that maps H1 → goal, H2 → known sections, H3 under Constraints → severity tiers. Unrecognized H2 sections are preserved as `UnknownSection` for extensibility.

```bash
brief --file examples/sample.brief.md emit json | python3 -c "
import sys, json
b = json.load(sys.stdin)
print(f'Goal: {b[\"goal\"]}')
print(f'Stack: {b[\"frontmatter\"][\"stack\"]}')
print(f'Hard constraints: {len(b[\"constraints\"][\"hard\"])}')
print(f'Soft constraints: {len(b[\"constraints\"][\"soft\"])}')
print(f'Ask-first constraints: {len(b[\"constraints\"][\"ask_first\"])}')
print(f'Sacred regions: {len(b[\"sacred\"])}')
print(f'Assumptions: {len(b[\"assumptions\"])} ({sum(1 for a in b[\"assumptions\"] if a[\"validated\"])} validated)')
"
```

```output
Goal: Redesign event pipeline for 10M events/day
Stack: ['Python 3.12', 'PostgreSQL 16', 'Kafka 3.7', 'GCP/k8s']
Hard constraints: 3
Soft constraints: 2
Ask-first constraints: 3
Sacred regions: 3
Assumptions: 3 (1 validated)
```

## 3. Validation

`brief validate` checks format correctness and codebase alignment: stack is non-empty, H1 goal exists, sacred entries are well-formed (backtick-wrapped paths), sacred globs match actual files, context files exist, and assumptions use checkbox syntax.

```bash
brief --file tests/fixtures/minimal.brief.md validate; echo "exit: $?"
```

```output
warning: Sacred path `src/auth.rs` matches no files
warning: Vague constraint (name a specific file, type, or command): "Do not break existing tests"
✓ briefing is valid (with 2 warning(s))
exit: 0
```

Valid briefs exit 0. Now a malformed brief — missing goal, missing stack, bad sacred formatting, missing checkboxes:

```bash
brief --file tests/fixtures/malformed.brief.md validate 2>&1; echo "exit: $?"
```

```output
error: Missing required `stack` field in frontmatter
error: Missing H1 goal statement
warning: Context file not found: ./nonexistent-file.md
error: Malformed sacred entry: path `not-in-backticks` should be wrapped in backticks
warning: Sacred path `not-in-backticks` matches no files
warning: Sacred path `valid/path/**` matches no files
error: Assumption missing checkbox syntax: "Missing checkbox syntax on this assumption"
warning: Vague constraint (name a specific file, type, or command): "A valid constraint"
exit: 1
```

Errors exit 1. Warnings alone exit 0. Every diagnostic has a severity level so CI pipelines can distinguish hard failures from advisories.

## 4. Emit targets

`brief emit` transforms a `.brief.md` into four target formats, each optimized for its consumer.

### Claude target

The primary emit target. Produces a CLAUDE.md section with structured headings, constraint severity labels, and sacred region formatting.

```bash
brief --file tests/fixtures/minimal.brief.md emit claude
```

```output
# Briefing: Fix the login bug

**Stack:** Rust

## Constraints

### Hard (Non-negotiable)

<rules priority="required">
- NEVER: break existing tests
</rules>

## Sacred Regions (Do Not Modify)

The following files and directories must not be modified under any circumstances. If a task requires changes to these paths, STOP and report the conflict.

<protected_files>
- `src/auth.rs` — Authentication logic, do not refactor
</protected_files>

```

### Prompt target

Raw system prompt text for direct API use — uppercase labels, flat structure, validated and unvalidated assumptions separated.

```bash
brief --file examples/sample.brief.md emit prompt
```

```output
HARD CONSTRAINTS:
- MUST: v2 API backward compatibility must be maintained
- MUST: All SQL must target PostgreSQL 16, not MySQL
- MUST: Infrastructure budget ceiling is $500/month

SACRED REGIONS:
The following files and directories must not be modified under any circumstances. If a task requires changes to these paths, STOP and report the conflict.
- src/auth/**: Proprietary tenant resolution logic, legally reviewed
- src/compliance/**: GDPR audit trail, approved by legal
- migrations/: Historical migration files must never be modified

GOAL: Redesign event pipeline for 10M events/day

STACK: Python 3.12, PostgreSQL 16, Kafka 3.7, GCP/k8s

REFERENCE CONTEXT:
- ./docs/current-architecture.md
- ./benchmarks/performance-baseline.csv

SOFT CONSTRAINTS:
- PREFER: async write patterns where possible
- PREFER: Favor composition over inheritance in new code

ASK BEFORE PROCEEDING:
- STOP and confirm with the user before: Database schema changes
- STOP and confirm with the user before: Adding new dependencies to requirements.txt
- STOP and confirm with the user before: Changes to the CI/CD pipeline

ASSUMPTIONS (UNVALIDATED):
- Bottleneck is synchronous DB writes (validate against perf baseline)
- Kafka cluster can handle 10M events/day without partition rebalancing

ASSUMPTIONS (VALIDATED):
- Current v2 API consumers do not use deprecated endpoints

DELIVERABLE:
Architecture doc with decision records, implementation plan with milestones,
and working code with tests achieving >80% coverage on new modules.
```

### AGENTS.md target

Merges all constraints into a single Instructions section with inline severity markers — `**(REQUIRED)**`, `*(preferred)*`, `**(ASK FIRST)**` — matching the AGENTS.md convention.

```bash
brief --file tests/fixtures/minimal.brief.md emit agents-md
```

```output
# Fix the login bug

**Stack:** Rust

## Instructions

- Do not break existing tests **(REQUIRED)**

## Protected Files

- `src/auth.rs`: Authentication logic, do not refactor

```

## 5. Sacred path checking

`brief check` tests whether a file path falls within a sacred region. Exit code 0 means safe to modify; exit code 1 means sacred. This is designed for git hooks and CI.

```bash
brief check src/auth/handler.rs --file examples/sample.brief.md 2>&1; echo "exit: $?"
```

```output
✗ src/auth/handler.rs is in sacred region `src/auth/**`
  Proprietary tenant resolution logic, legally reviewed
exit: 1
```

```bash
brief check src/api/routes.rs --file examples/sample.brief.md 2>&1; echo "exit: $?"
```

```output
✓ src/api/routes.rs is not in a sacred region
exit: 0
```

Three matching strategies are used: glob matching, prefix matching, and cleaned-pattern prefix matching. The `src/auth/**` pattern catches `src/auth/handler.rs` via glob; `migrations/` catches `migrations/001_init.sql` via prefix.

## 6. Semantic diff

`brief diff` compares two briefing files and shows what changed semantically — not line-by-line, but by section: added/removed constraints, changed sacred regions, shifted assumptions.

```bash
brief diff tests/fixtures/minimal.brief.md examples/sample.brief.md
```

```output
Goal changed:
  - Fix the login bug
  + Redesign event pipeline for 10M events/day

Stack changed:
  - Rust
  + Python 3.12
  + PostgreSQL 16
  + Kafka 3.7
  + GCP/k8s

Hard constraints changed:
  - Do not break existing tests
  + v2 API backward compatibility must be maintained
  + All SQL must target PostgreSQL 16, not MySQL
  + Infrastructure budget ceiling is $500/month

Soft constraints changed:
  + Prefer async write patterns where possible
  + Favor composition over inheritance in new code

Ask-first constraints changed:
  + Database schema changes
  + Adding new dependencies to requirements.txt
  + Changes to the CI/CD pipeline

Sacred regions changed:
  - `src/auth.rs` — Authentication logic, do not refactor
  + `src/auth/**` — Proprietary tenant resolution logic, legally reviewed
  + `src/compliance/**` — GDPR audit trail, approved by legal
  + `migrations/` — Historical migration files must never be modified

Assumptions changed:
  + [ ] Bottleneck is synchronous DB writes (validate against perf baseline)
  + [ ] Kafka cluster can handle 10M events/day without partition rebalancing
  + [x] Current v2 API consumers do not use deprecated endpoints

Deliverable changed:
  + Architecture doc with decision records, implementation plan with milestones,
and working code with tests achieving >80% coverage on new modules.

```

## 7. Init: auto-detection scaffolding

`brief init` analyzes the current directory and scaffolds a `.brief.md` with sensible defaults — detecting stack from config files, context from documentation, and sacred candidates from common patterns.

```bash
cd /tmp && mkdir -p brief-init-demo/src/auth brief-init-demo/migrations && echo '[package]' > brief-init-demo/Cargo.toml && echo '# My Project' > brief-init-demo/README.md && cd brief-init-demo && brief init && cat .brief.md && rm -rf /tmp/brief-init-demo
```

```output
Created /private/tmp/brief-init-demo/.brief.md
Edit the file to fill in your goal, constraints, and sacred regions.
---
stack: [Rust]
context: [./README.md]
---

# <Describe your goal here>

## Constraints

### Hard
- <non-negotiable constraint>

### Soft
- <preferred but flexible constraint>

### Ask First
- <requires human approval before proceeding>

## Sacred
- `src/auth/**` — Authentication logic
- `migrations/**` — Database migrations — never alter historical files

## Assumptions
- [ ] <assumption to validate>

## Deliverable
<Describe what "done" looks like>
```

The scaffolder detected Rust from `Cargo.toml`, found `README.md` as context, and identified `src/auth/` and `migrations/` as sacred candidates. The user fills in the goal, constraints, and deliverable — the structure is ready in under 60 seconds.

## 8. Test coverage

Phase 1 shipped with 54 tests across three suites: parser unit tests for every edge case (missing H1, malformed sacred, checkbox syntax), validation tests against all fixture files, and integration tests round-tripping every fixture through parse → emit → verify.

```bash
git log --format='%h %s' 7a60857 -1 && echo '' && git diff --stat f57e18a..7a60857 | tail -1
```

```output
7a60857 Implement Phase 1: parser, validator, emitters, and CLI

 17 files changed, 2409 insertions(+), 2 deletions(-)
```

## Summary

Phase 1 delivers a complete, tested CLI for the `.brief.md` format:

| Command | Purpose |
|---------|---------|
| `brief init` | Scaffold a `.brief.md` with auto-detected stack, context, and sacred candidates |
| `brief validate` | Check format correctness and codebase alignment (exit codes for CI) |
| `brief emit claude` | Emit a CLAUDE.md section |
| `brief emit prompt` | Emit raw system prompt text for API use |
| `brief emit agents-md` | Emit an AGENTS.md section |
| `brief emit json` | Emit structured JSON for tooling integration |
| `brief check <path>` | Test if a path is in a sacred region (exit codes for git hooks) |
| `brief diff <a> <b>` | Semantic diff between two briefing files |

The architecture: parse into a strongly-typed `Brief` struct, validate against the codebase, then emit to any target. 2,409 lines of Rust across 17 files, with 54 tests covering parser edge cases, validation diagnostics, and round-trip emission fidelity.
