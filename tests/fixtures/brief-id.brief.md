---
stack: [Rust]
context: [./README.md]
model: gpt-4o-mini
---

# Review Rust changes — "safely"

## Constraints

### Hard
- Never touch `unsafe` blocks
- [`src/ui/**`] WCAG 2.1 AA

### Soft
- Prefer small diffs

### Ask First
- Adding a dependency

## Sacred
- `src/auth/**` — security reviewed

## Assumptions
- [x] CI is green

## Deliverable
A review comment per finding.
Each names a file and a line.
