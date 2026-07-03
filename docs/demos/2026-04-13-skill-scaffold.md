# brief skill scaffold — Hand-Editable Skill Skeletons

*2026-06-22T02:46:25Z by Showboat 0.6.1*
<!-- showboat-id: 96eaaeb2-7966-491d-af79-ff098d781ace -->

The `brief skill scaffold` command writes a spec-compliant Agent Skills skeleton you finish by hand. Source precedence: `--from-brief` (an existing `.brief.md`), then `--description` (skill-first, no brief), then the active `.brief.md`. brief owns only the `metadata.brief.source` stamp — the body is yours to edit. This demo scaffolds from a literal description, validates it, then scaffolds from a brief.

## 1. Scaffold from a description

The skill-first path: pass `--description` and an explicit `--name`. No `.brief.md` required.

```bash
mkdir -p /tmp/brief-scaffold-demo && cd /tmp/brief-scaffold-demo && rm -rf pr-security-review review review.brief.md && brief skill scaffold --description "Review pull requests for security issues" --name pr-security-review && echo "---" && ls -R pr-security-review
```

```output
Scaffolded /private/tmp/brief-scaffold-demo/pr-security-review
Edit SKILL.md, scripts/, and references/ — brief owns only metadata.brief.source.
---
references
scripts
SKILL.md

pr-security-review/references:

pr-security-review/scripts:
```

The skeleton is a valid `SKILL.md` plus empty `scripts/` and `references/` directories. brief stamps `metadata.brief.source`; everything else is a placeholder for you to fill in.

```bash
cat /tmp/brief-scaffold-demo/pr-security-review/SKILL.md
```

```output
---
name: pr-security-review
description: Review pull requests for security issues
metadata:
  brief.source: Review pull requests for security issues
---

# Pr Security Review

Add instructions here.
```

## 2. Validate the skeleton

A fresh scaffold already passes `brief skill validate` against the Agent Skills spec — name, description, and frontmatter are well-formed out of the box.

```bash
cd /tmp/brief-scaffold-demo && brief skill validate pr-security-review && echo "exit: $?"
```

```output
SKILL.md at "pr-security-review/SKILL.md" is valid.
exit: 0
```

## 3. Scaffold from an existing brief

With `--from-brief`, the skill name and description derive from the briefing instead of literal flags. `metadata.brief.source` records the originating file, so the skill is traceable back to its brief.

```bash
cd /tmp/brief-scaffold-demo && cat > review.brief.md << "EOF"
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
EOF
rm -rf review && brief skill scaffold --from-brief review.brief.md && echo "---" && cat review/SKILL.md
```

```output
Scaffolded /private/tmp/brief-scaffold-demo/review
Edit SKILL.md, scripts/, and references/ — brief owns only metadata.brief.source.
---
---
name: review
description: Review code changes following team standards
metadata:
  brief.source: ../review.brief.md
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

## 4. Cleanup

```bash
rm -rf /tmp/brief-scaffold-demo && echo "workspace removed"
```

```output
workspace removed
```
