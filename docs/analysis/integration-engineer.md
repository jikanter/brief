# Integration Engineer Analysis

**Agent Role:** Evaluate how well `brief` integrates with advanced AI tooling ecosystems — Claude Code, MCP, GitHub Copilot, Cursor, and other agent runtimes.

**Scope:** All emitters, CLI commands, init auto-detection, and real-world workflow integration.

---

## Executive Summary

`brief` is architecturally sound at the format and parser level but has significant integration gaps across real-world AI tooling ecosystems. The Claude Code emitter works but is basic; MCP integration is entirely missing; git/CI workflow integration is minimal; and critical emit targets (Cursor rules, Copilot instructions, Windsurf conventions) don't exist. The tool solves authoring friction elegantly but hasn't integrated deeply with runtime enforcement, multi-agent coordination, or developer workflow automation.

---

## Claude Code Integration (Shallow, Passive)

### What Works
- Emits format-correct CLAUDE.md with all core briefing elements
- Properly distinguishes constraint severity (Hard → "Non-negotiable", Soft → "Preferred", Ask First → "Requires approval")
- Sacred regions clearly marked as "Do Not Modify"
- Tests verify output correctness

### What's Missing
- **No permissions/enforcement hook integration** — Claude Code supports permission rules that could enforce sacred regions; brief doesn't emit them
- **No MCP server specification** — Claude Code reads MCP server configs from `.claude/mcp.json`; brief doesn't declare or configure these
- **No slash command context** — Brief could define task-specific `/` commands (e.g., `/validate-migration`, `/check-sacred`)
- **No hook spec** — Claude Code supports `before_write.sh`, `after_commit.sh` hooks; sacred regions should be enforceable via `brief check` in hooks
- **Context files not inlined** — Listed as prose references, not embedded or structured for Claude Code's parser
- **Model selection ignored** — Frontmatter `model` field not emitted in a way Claude Code can respect
- **Assumption validation workflow missing** — No pre-task gate or validation checklist mechanism

### Impact
The brief is advisory, not operational. Claude Code follows constraints because humans read them and tell it to, not because the format enables automated enforcement.

---

## MCP Integration (Entirely Missing — Critical Gap)

### What Should Exist
- **MCP server** exposing brief data as tools:
  - `check_sacred_path(filepath)` → `{is_sacred: bool, reason: str}`
  - `get_constraints(type?)` → constraint list
  - `get_briefing()` → full Brief as JSON
  - `log_decision(action, constraint_ref, rationale)` → decision audit trail
- **MCP resource protocol** — expose `.brief.md` as an MCP resource for agents to read through transport layer
- **Resource subscriptions** — auto-invalidate assumptions when sacred regions are touched
- **Constraint enforcement as tools** — runtime gates instead of prompt-embedded suggestions

### Why This Is Critical
- Multiple agents (Claude Code, Copilot, custom tools) need to respect the same constraints
- Currently each agent reads a different emit target with divergent fidelity
- An MCP server makes brief the single source of truth queryable by any agent at runtime

---

## CI/CD & Git Hook Integration (Minimal, Ad Hoc)

### Current State
- `brief check <path>` exists and works for sacred path checking
- Can be manually wired into git hooks

### What's Missing
- **No pre-commit framework integration** — No hook package for the `pre-commit` framework (~40% of OSS projects)
- **No diff-based validation** — `brief validate` checks static state but not actual changes; needs `brief validate-diff <branch>`
- **No GitHub Actions docs or examples** — No CI workflow templates for brief validation
- **No CI/CD context passing** — No standard interface for extracting brief data as environment variables
- **No commit message validation** — No mechanism to link commits back to briefs

### Impact
Teams using brief for local development have no way to enforce it in CI. Constraints remain advisory; malicious or careless pushes to sacred regions go uncaught.

---

## Multi-Agent Coordination (No Cross-Agent Consistency)

### The Problem
Real projects use multiple AI tools simultaneously:
- Claude Code reads `CLAUDE.md`
- GitHub Copilot reads `.github/copilot-instructions.md`
- Cursor reads `.cursorrules`
- Windsurf reads `.windsurfrules`
- Aider reads `.aider.conf.yml`

Brief emits to Claude and AGENTS.md only. A developer using both Claude Code and Cursor must manually maintain duplicate configurations. If the brief changes, three files need manual updates — defeating the single-source-of-truth purpose.

### What's Needed
- `brief emit copilot` → `.github/copilot-instructions.md`
- `brief emit cursor` → `.cursorrules`
- `brief emit windsurf` → `.windsurfrules`
- `brief emit aider` → `.aider.conf.yml`
- `brief sync` or `brief emit --all-ides` → writes all detected targets from one source

---

## Missing High-Value Emit Targets

1. **System prompt for API deployment** — optimized for Claude API usage, not just chat context
2. **OpenAI custom instructions** — compatible with OpenAI's system message format
3. **GitHub Copilot Workspace hints** — workspace-level configuration
4. **Test generation scaffolds** — convert assumptions and deliverable expectations into test stubs
5. **Decision log templates** — ADR/MADR format cross-referenced with brief constraints

---

## Workflow Gaps

### Brief Evolution
- No `brief history` to show changes with commit context
- No `brief blame <constraint>` to show who added each constraint
- Brief changes aren't tracked semantically — just raw git diff

### Brief-Driven Code Review
- No `brief check-pr <pr-number>` to validate a PR diff against constraints
- No reviewer tool to highlight sacred region violations or constraint non-compliance

### Onboarding
- No `brief onboard --interactive` to walk new developers through project constraints
- No structured way to verify understanding of sacred regions and conventions

### Monorepo Composition
- No hierarchical brief inheritance for org-level + service-level constraints
- No way to audit which services comply with org-wide policy

### Constraint Conflict Resolution
- No `brief lint` to detect contradictions within a brief
- No `brief check-consistency --against <CLAUDE.md>` to compare against actual prompt text

---

## Init Command Gaps

### Current Detection
- Stack: Cargo.toml, go.mod, Gemfile, pyproject.toml, package.json, docker-compose.yml
- Context: README.md, docs/architecture.md, CONTRIBUTING.md
- Sacred candidates: src/auth/**, migrations/**

### Missing Detection
- **No framework detection** — React, Django, Gin, Express not identified from imports/dependencies
- **No cloud/infrastructure** — Terraform, Kubernetes, Serverless Framework not detected
- **No test framework detection** — Pytest, Jest, Go tests, Rust tests not included in stack
- **No CI/CD detection** — `.github/workflows`, `.gitlab-ci.yml`, `Jenkinsfile` not detected as context
- **No license header detection** — Files with proprietary/GPL headers not flagged as sacred candidates
- **Weak sacred heuristics** — Doesn't detect build outputs (`.next`, `dist/`, `target/`), generated files (protobuf, codegen), or test files

---

## Key Recommendations (Prioritized by Impact)

### Tier 1 — Critical for Real Adoption
1. **MCP Server** — enables multi-agent consistency and runtime enforcement
2. **Copilot/GitHub emit target** — covers the largest user base
3. **Cursor rules emit target** — millions of DAUs
4. **Pre-commit framework integration** — standard in OSS ecosystem
5. **CI/CD validation commands** — `brief validate-diff`, GitHub Actions templates

### Tier 2 — High Value for Workflows
6. **Brief evolution tracking** — `brief history`, `brief blame`
7. **Multi-service composition** — monorepo inheritance model
8. **System prompt emission** — direct Claude API users
9. **Brief-driven code review** — structured PR validation

### Tier 3 — Nice to Have
10. **Windsurf/Aider rules targets**
11. **Interactive onboarding CLI**
12. **Decision log template emission**
13. **Semantic linting** (goal quality, constraint clarity)
14. **Constraint conflict detection**
