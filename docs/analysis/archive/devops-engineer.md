# DevOps Engineer Analysis: Operational Readiness & CI/CD Integration

**Date:** 2026-03-29
**Role:** DevOps Engineer evaluating brief for production pipeline integration, enforcement, and team-scale operations.

---

## 1. Operational Context Gap

AI agents need operational context to work safely. When Claude Code opens a project and reads its CLAUDE.md, the first thing it looks for is how to build, test, and lint. The second thing it looks for is what not to break. Brief captures the second category well (sacred regions, constraints) but entirely ignores the first.

### What Agents Actually Execute

In practice, the most frequently referenced sections of a CLAUDE.md are not constraints or sacred regions. They are commands:

- **Build:** `cargo build`, `npm run build`, `make all`
- **Test:** `cargo test`, `pytest -x`, `npm test`
- **Lint/Format:** `cargo clippy && cargo fmt --check`, `ruff check .`, `eslint .`
- **Type Check:** `tsc --noEmit`, `mypy .`
- **Integration Test:** `docker-compose up -d && cargo test --features integration`
- **Deploy:** `kubectl apply -k overlays/staging/`, `fly deploy`

These commands are the operational backbone. An agent that cannot run the test suite is flying blind. An agent that runs the wrong build command wastes cycles. Brief currently has no way to express any of this.

### Infrastructure Dependencies

Beyond commands, agents need to know what external systems exist:

- **Databases:** PostgreSQL on port 5432, Redis on 6379, connection string patterns
- **Message queues:** Kafka topics and consumer groups
- **Cloud services:** S3 buckets, KMS keys, IAM roles
- **Environment variables:** `DATABASE_URL`, `REDIS_URL`, `API_KEY` (patterns, not values)
- **Local development requirements:** Docker, specific tool versions, config files to copy

Brief's `stack` field lists technology names (`[Python 3.12, PostgreSQL 16, Kafka 3.7]`) but not how to connect to them, start them, or verify they are running. An agent seeing `PostgreSQL 16` in the stack cannot infer whether it needs `docker-compose up db` or whether a cloud instance is always available.

### Monitoring and Observability

When agents make changes, operators need to know what to watch:

- Key metrics (latency p99, error rate, queue depth)
- Log locations and grep patterns for failures
- Health check endpoints
- Alerting thresholds that would indicate a regression

### Should Brief Capture These?

Yes, but selectively. The question is not whether this information matters -- it clearly does. The question is whether it belongs in the brief format or should be left to existing files. The answer is a pragmatic middle ground:

**Brief should capture commands.** Build, test, lint, and deploy commands are the single most operationally impactful piece of context an agent can have. They change infrequently, they are project-specific, and they are the most common source of duplication between CLAUDE.md and existing config files. A `## Commands` section with 4-6 lines replaces the most-referenced part of a typical CLAUDE.md.

**Brief should NOT capture infrastructure details inline.** Connection strings, environment variable catalogs, and infrastructure topology belong in dedicated files (`docker-compose.yml`, `.env.example`, `infrastructure.md`). Brief should reference these via the existing `context` field. The `init` command should detect and include them automatically.

**Brief should support verification commands in deliverables.** The deliverable section should allow structured acceptance criteria with commands that can be run to verify completion, not just freeform prose.

---

## 2. CI/CD Enforcement Pipeline

Brief's current enforcement story is: there is none. `brief validate` checks format correctness. `brief check <path>` checks a single path against sacred regions. Neither integrates into any CI pipeline. There are no hooks, no Actions, no templates.

### GitHub Actions Workflow

A minimal but production-useful workflow for brief validation on PRs:

```yaml
name: brief-validate
on:
  pull_request:
    paths:
      - '**.brief.md'
      - 'src/**'
      - 'migrations/**'

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install brief
        run: cargo install brief-cli
      - name: Validate brief format
        run: brief validate
      - name: Check sacred regions
        run: |
          git diff --name-only origin/${{ github.base_ref }}...HEAD | while read file; do
            brief check "$file" || exit 1
          done
```

This reveals two immediate gaps in the current CLI:

1. **No batch check command.** The `brief check` command takes a single path. Checking an entire PR diff requires a shell loop. A `brief check-diff <base-ref>` command that accepts a git ref and checks all changed files would be far more practical.

2. **No machine-readable output.** The current output uses colored terminal text with Unicode check marks. CI pipelines need structured output -- exit codes (which exist), JSON diagnostics (which do not), and GitHub-compatible annotation format (which does not exist).

### Pre-commit Hook Integration

Two approaches, both currently unsupported:

**Standalone git hook (`.git/hooks/pre-commit`):**
```bash
#!/bin/sh
brief validate || exit 1
git diff --cached --name-only | while read file; do
  brief check "$file" 2>/dev/null
  if [ $? -eq 1 ]; then
    echo "BLOCKED: $file is in a sacred region"
    exit 1
  fi
done
```

**Pre-commit framework (`.pre-commit-config.yaml`):**
```yaml
repos:
  - repo: https://github.com/jikanter/brief
    rev: v0.1.0
    hooks:
      - id: brief-validate
        name: Validate .brief.md
        entry: brief validate
        language: rust
        files: '\.brief\.md$'
      - id: brief-check-sacred
        name: Check sacred regions
        entry: brief check
        language: rust
        pass_filenames: true
```

Brief currently ships none of this. The pre-commit framework integration requires a `.pre-commit-hooks.yaml` in the repository, which does not exist. The standalone hook requires documentation and a `brief init-hooks` command to install it.

### How `brief validate-diff` Should Work

This is the most operationally important missing command. Here is how it should work technically:

1. Accept a git ref as argument: `brief validate-diff main` or `brief validate-diff HEAD~3`
2. Run `git diff --name-only <ref>...HEAD` to get the list of changed files
3. For each changed file, run sacred region checking against all sacred patterns in the brief
4. Optionally accept `--staged` flag to check only staged changes (for pre-commit use)
5. Report violations with file path, sacred pattern matched, and reason
6. Exit 0 if no violations, exit 1 if any sacred file was touched
7. Support `--json` flag for machine-readable output
8. Support `--github-annotations` flag for PR annotation format

Implementation-wise, this is straightforward. The `check_path` function in `src/check.rs` already handles the core matching logic. The new command would be a thin wrapper that:
- Shells out to `git diff --name-only` (or uses the `git2` crate for library-level access)
- Iterates the file list through `check_path`
- Aggregates and formats results

The tricky part is handling renamed files (sacred path `src/auth/handler.rs` renamed to `src/auth/handlers/main.rs` -- is the new path still sacred?) and deleted files (should deleting a sacred file be a violation? Almost certainly yes).

### Constraint Compliance Reporting

Beyond sacred regions, constraint compliance on PRs is a harder problem. Most constraints are natural-language prose ("v2 API backward compatibility must be maintained") that cannot be mechanically verified. However, a subset of constraints could be tied to verification commands:

- "All tests must pass" maps to `cargo test`
- "Coverage >= 80%" maps to `cargo tarpaulin --threshold 80`
- "No lint warnings" maps to `cargo clippy -- -D warnings`

If the `## Commands` section and structured deliverables with verification commands exist, `brief verify` could run each command and report pass/fail. This transforms constraint compliance from "read the constraint and hope the agent followed it" into "run the check and know."

---

## 3. The Commands Section Debate

### The Case For

Commands are the most operationally impactful context an AI agent receives. In every CLAUDE.md I have seen in production use, the commands section is the most referenced. Agents run `cargo test` dozens of times per session. They need to know the exact command, not guess.

The information exists in Makefile, package.json scripts, Cargo.toml, and similar files. But:

1. **It is scattered.** A Rust project might have `cargo test` for unit tests, `docker-compose up -d && cargo test --features integration` for integration tests, and `cargo clippy && cargo fmt --check` for lint. These live in different places (Cargo.toml, docker-compose.yml, CI config). Brief would centralize them.

2. **It is ambiguous.** `package.json` might have `"test": "jest"`, but the actual command developers run is `npm test -- --coverage --watchAll=false`. The canonical invocation matters.

3. **Agents already look for it.** Claude Code's CLAUDE.md format explicitly expects a Commands section. Brief emitting to `claude` target without a Commands section produces output that is missing the section agents are most trained to look for.

### The Case Against

Duplication is real. If `Cargo.toml` says `cargo test` and the brief also says `cargo test`, now there are two sources of truth. When someone adds a feature flag to tests, they must update both. This is the classic DRY violation that leads to stale documentation.

### Resolution

Brief should have a `## Commands` section, but `brief init` should auto-detect commands from existing config files, and `brief validate` should warn when commands reference binaries not found on PATH or commands that appear to duplicate config file entries without adding information.

Auto-detection strategy for `brief init`:

| Source File | Detected Commands |
|------------|-------------------|
| `Cargo.toml` | `cargo build`, `cargo test`, `cargo clippy`, `cargo fmt` |
| `package.json` scripts | `npm run <script>` for build, test, lint, dev, start |
| `Makefile` | Parse targets, prioritize `build`, `test`, `lint`, `fmt`, `deploy` |
| `pyproject.toml` | Look for `[tool.pytest]`, `[tool.ruff]`, `[tool.mypy]` |
| `docker-compose.yml` | `docker-compose up`, `docker-compose run test` |
| `.github/workflows/*.yml` | Extract run commands from CI steps |

The scaffold would produce:

```markdown
## Commands

- Build: `cargo build`
- Test: `cargo test`
- Lint: `cargo clippy`
- Format: `cargo fmt`
```

This is concise, readable, and directly useful. It also creates a natural bridge to the `## Deliverable` verification story: if the deliverable says "all tests pass," `brief verify` knows to run `cargo test`.

### Relationship to Deliverable Verification

Commands and deliverables form a verification pipeline:

```markdown
## Commands
- Test: `cargo test`
- Lint: `cargo clippy -- -D warnings`
- Coverage: `cargo tarpaulin`

## Deliverable
- [ ] Tests pass: `cargo test`
- [ ] No lint warnings: `cargo clippy -- -D warnings`
- [ ] Coverage >= 80%: `cargo tarpaulin --threshold 80`
- [ ] No sacred violations: `brief validate-diff main`
```

Each deliverable acceptance criterion references a command. `brief verify` runs them in sequence and reports pass/fail. This closes the loop: brief is no longer advisory, it is executable.

---

## 4. Multi-Service / Monorepo Operations

Brief's current model is one `.brief.md` per project root. This works for a single-service repo. It does not work for:

- **Monorepos** with 5-50 services under `services/`, each with their own stack, constraints, and sacred regions
- **Organizations** with shared policies (security constraints, compliance sacred regions) that apply across all repos
- **Platform teams** maintaining shared infrastructure that every service depends on

### Brief Inheritance

The `extends` field in frontmatter is the right primitive:

```yaml
---
extends: ../../.brief.md
stack: [Python 3.12, Redis 7]
---
```

Merge semantics:
- `stack`: union of parent and child
- `context`: union, with child paths relative to child directory
- `constraints.hard`: union (child cannot weaken parent hard constraints)
- `constraints.soft`: child overrides parent for conflicts
- `sacred`: union (child can add sacred regions, cannot remove parent's)
- `commands`: child overrides parent for same command name

This creates a natural hierarchy:

```
org/.brief.md                  # Company-wide: security, compliance, coding standards
  services/api/.brief.md       # API service: extends org, adds API-specific constraints
  services/worker/.brief.md    # Worker service: extends org, adds queue-specific constraints
  shared/auth/.brief.md        # Shared lib: extends org, marks everything sacred
```

### Cross-Service Sacred Region Enforcement

In a monorepo, service A should not modify service B's sacred regions. This requires:

1. `brief validate-diff` understanding which brief applies to which directory subtree
2. A brief discovery mechanism: walk up from changed file to find nearest `.brief.md`
3. Aggregate reporting: "PR touches 3 services, 2 have sacred violations"

Implementation: `brief validate-diff main --monorepo` scans the diff, groups files by nearest `.brief.md`, validates each group against its own brief, and reports per-service results.

### Aggregate Compliance Reporting

For org-wide visibility:

```
brief audit --recursive
```

Walk the repository, find all `.brief.md` files, validate each, and produce a summary:

```
services/api/.brief.md       VALID  (2 warnings)
services/worker/.brief.md    INVALID (missing H1 goal)
services/auth/.brief.md      VALID
shared/proto/.brief.md        VALID  (1 warning: sacred path matches nothing)

3/4 briefs valid. 1 error, 3 warnings total.
```

This is useful for platform teams tracking compliance across dozens of services.

---

## 5. Brief as Infrastructure

The progression from file format to infrastructure follows a natural path:

### Stage 1: Passive Documentation (Current State)

Brief is a file. Humans write it, agents read the emitted output. No enforcement, no runtime interaction. This is where brief is today.

### Stage 2: CI Gate (Immediate Next Step)

Brief becomes a CI check. `brief validate` and `brief validate-diff` run on every PR. Sacred region violations block merges. Format errors block merges. This is achievable with the current architecture plus 2-3 new commands and a GitHub Action.

This is the highest-impact, lowest-effort transition. It requires:
- `brief validate-diff <ref>` command
- `brief validate --json` output mode
- A GitHub Action or reusable workflow
- Pre-commit hook support
- Documentation

### Stage 3: Development Workflow Tool

Brief drives local development. `brief verify` runs acceptance criteria. `brief init-hooks` installs git hooks. `brief watch` monitors file changes and warns when sacred regions are touched. Agents query brief at runtime through the MCP server.

### Stage 4: Organizational Infrastructure

Brief becomes the standard for expressing and enforcing development policies. Brief inheritance creates org-wide policies. Aggregate compliance dashboards show which teams and services comply. Brief changes go through review processes. Brief compliance is a deployment gate.

### Integration with Existing DevOps Tools

Brief should not try to replace existing tools. Instead, it should integrate:

- **Terraform/Kubernetes:** Brief's `context` field references infrastructure definitions. Brief's sacred regions can protect Terraform state files and K8s manifests. Brief does not manage infrastructure.
- **CI/CD platforms:** Brief provides validation commands that plug into any CI system (GitHub Actions, GitLab CI, Jenkins). Brief does not orchestrate pipelines.
- **Git hooks:** Brief provides the check commands; the hook framework (pre-commit, husky, lefthook) provides the plumbing. Brief does not replace hook managers.
- **Monitoring:** Brief's `## Commands` section can include health check commands. Brief does not collect metrics.

The principle: brief is the source of truth for development intent and constraints. It emits to where agents read, validates in CI, and exposes data via MCP. It does not try to become the CI system, the deployment tool, or the monitoring platform.

---

## 6. Concrete Recommendations

Ranked by practical impact for making brief production-ready for teams:

### 1. `brief validate-diff <ref>` Command

**Impact: Critical.** This single command enables CI enforcement of sacred regions. Without it, brief is advisory only. With it, sacred region violations block PRs.

Implementation: ~200 lines of Rust. Shell out to `git diff --name-only`, iterate through `check_path`, aggregate results. Add `--json` and `--staged` flags. This could ship in a week.

### 2. `## Commands` Section with Auto-Detection

**Impact: High.** This is the most frequently referenced section in any agent context file. Adding it to the format spec, parser, model, and all emitters makes brief output immediately more useful to agents. Auto-detection in `brief init` means the section is populated correctly from day one.

Implementation: Add `commands` field to `Brief` model as `Vec<CommandEntry>` (name + command string). Parse `## Commands` section as list items with `Name: \`command\`` format. Extend all 5 emitters. Extend `init.rs` with detection from Cargo.toml, package.json, Makefile, pyproject.toml. Medium effort, ~400 lines across all files.

### 3. GitHub Action and Pre-commit Hook Package

**Impact: High.** These are the distribution channels for CI enforcement. A GitHub Action (`jikanter/brief-action@v1`) and a `.pre-commit-hooks.yaml` in the repo make adoption a one-line config change for teams.

Implementation: A reusable GitHub Action is a small YAML file plus a Dockerfile or composite action. The pre-commit hook definition is about 20 lines. Documentation is the main effort. Could ship alongside `validate-diff`.

### 4. Machine-Readable Output (`--json`, `--github-annotations`)

**Impact: Medium-High.** CI pipelines, dashboards, and tooling integrations all need structured output. The current colored terminal output is human-readable but machine-hostile. Adding `--format json` to `validate`, `check`, and `validate-diff` unlocks downstream tooling.

Implementation: The `Diagnostic` struct already exists and derives `Serialize`. JSON output is mostly `serde_json::to_string_pretty(&diagnostics)`. GitHub annotation format requires mapping diagnostics to `::error file=...,line=...::message` format. Small effort, ~100 lines.

### 5. `brief verify` Command for Executable Acceptance Criteria

**Impact: Medium.** Closes the loop from "advisory constraints" to "verified compliance." Runs verification commands from structured deliverables, reports pass/fail. Transforms brief from documentation into a test harness for development intent.

Implementation: Requires structured deliverable parsing (acceptance criteria with embedded commands), a command runner with timeout/output capture, and reporting. Medium effort, ~300 lines. Depends on the Commands section existing first.

---

## Summary

Brief's operational gap is not a design flaw -- it is a scope gap. Phase 1 correctly focused on the authoring and format problem. Phase 2 needs to close the enforcement loop. The path is clear: `validate-diff` for CI gates, a Commands section for operational context, pre-commit hooks and GitHub Actions for distribution, machine-readable output for tooling, and `verify` for executable acceptance criteria. These five features transform brief from a documentation format into operational infrastructure that teams can actually enforce.
