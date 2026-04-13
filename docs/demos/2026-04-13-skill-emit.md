# brief skill emit — Generating Agent Skills from Briefings

The `brief skill emit` command transforms a `.brief.md` file into an `agentskills.io` compliant `SKILL.md` — a reusable skill that can be discovered by AI agents. This demo walks through the workflow: viewing a brief, emitting its skill representation, and installing it into the agent's skills directory.

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

`brief skill emit` transforms the structured brief into a `SKILL.md`.

```bash
./target/debug/brief --file tests/fixtures/skill.brief.md skill emit
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

The `--install` flag writes the skill directly to `.claude/skills/review/SKILL.md`, ready for discovery.

```bash
./target/debug/brief --file tests/fixtures/skill.brief.md skill emit --install
```

```output
Installed .claude/skills/review/SKILL.md
```

The skill is now discoverable. Any developer with this repo can now use the skill to follow the team's exact standards.

```bash
rm -rf .claude/skills/review
```
