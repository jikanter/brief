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
