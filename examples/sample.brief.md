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
