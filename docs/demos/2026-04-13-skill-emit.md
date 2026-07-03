# brief skill emit — Generating Agent Skills from Briefings

*2026-06-22T03:58:01Z by Showboat 0.6.1*
<!-- showboat-id: 26147ebd-9cc1-4de6-a6a0-49c113e35eae -->

The `brief skill emit` command transforms a `.brief.md` file into an `agentskills.io`-compliant `SKILL.md` — a reusable skill discoverable by AI agents. This demo views a skill-oriented brief, emits its skill representation, and installs it into the agent skills directory.

## 1. A skill-oriented brief

A `.brief.md` becomes a skill when you add `skill_name` and `skill_description` to the frontmatter.

```bash
cat tests/fixtures/skill.brief.md
```

```output
---
stack: [Python 3.12, PostgreSQL 16]
context: [./docs/api-spec.yaml]
skill_name: review
skill_description: Review code changes following team standards
---

# Review code following team standards

## Constraints

### Hard
- All SQL must target PostgreSQL 16
- API backward compatibility must be maintained

### Soft
- Prefer async patterns

### Ask First
- Database schema changes

## Sacred
- `src/auth/**` — Authentication logic, SOC2 audited
- `migrations/**` — Historical migrations, never alter

## Assumptions
- [ ] Current tests cover critical paths
- [x] CI pipeline runs on every PR

## Deliverable
Clear review comments with specific file/line references and suggested fixes.
```

## 2. Emitting the skill

`brief skill emit` transforms the structured brief into a `SKILL.md`: hard constraints become MUST rules, soft become preferences, ask-first become confirmation gates, sacred regions become protected paths, and unvalidated assumptions become a pre-flight checklist.

```bash
brief --file tests/fixtures/skill.brief.md skill emit
```

```output
---
name: review
description: Review code changes following team standards
---

Review code changes following team standards. This project uses Python 3.12, PostgreSQL 16.

Before starting, read these files for context:

- `./docs/api-spec.yaml`

## Rules

You MUST follow these rules:

- All SQL must target PostgreSQL 16
- API backward compatibility must be maintained

Prefer these approaches when possible:

- Prefer async patterns

Ask the user before proceeding with:

- Database schema changes

## Protected regions

Do NOT modify or suggest changes to these files:

- `src/auth/**` — Authentication logic, SOC2 audited
- `migrations/**` — Historical migrations, never alter

## Verify before proceeding

Confirm these assumptions still hold before acting on them:

- Current tests cover critical paths

## Expected output

Clear review comments with specific file/line references and suggested fixes.
```

## 3. Installing the skill

The `--install` flag writes the skill directly to `.claude/skills/<name>/SKILL.md`, ready for discovery by any agent with this repo checked out.

```bash
brief --file tests/fixtures/skill.brief.md skill emit --install
```

```output
Installed .claude/skills/review/SKILL.md
```

## 4. Cleanup

```bash
rm -rf .claude/skills/review && echo "skill removed"
```

```output
skill removed
```
