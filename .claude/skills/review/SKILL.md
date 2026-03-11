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
