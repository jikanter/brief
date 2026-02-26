# brief

Structured briefing format for AI coding agents.

`brief` provides a fast, familiar format (`.brief.md`) for humans to express intent, constraints, and sacred code regions to AI agents — and a CLI to validate, compose, and emit those briefings to multiple agent runtimes.

## Quick Start

```bash
# Initialize a briefing from your repo
brief init

# Validate against your codebase
brief validate

# Emit for Claude Code
brief emit claude > CLAUDE.md

# Check if a path is sacred
brief check src/auth/handler.rs
```

## The Format

A `.brief.md` file is Markdown with YAML frontmatter:

```markdown
---
stack: [Python 3.12, PostgreSQL 16]
context: [./docs/architecture.md]
---

# Redesign event pipeline for 10M events/day

## Constraints

### Hard
- v2 API backward compatibility

### Soft
- Prefer async patterns

### Ask First
- Database schema changes

## Sacred
- `src/auth/**` — Proprietary tenant resolution

## Assumptions
- [ ] Bottleneck is synchronous DB writes

## Deliverable
Architecture doc + implementation plan + working code
```

See [examples/sample.brief.md](examples/sample.brief.md) for a complete example.

## Why?

As AI agents handle more technical execution, the human→agent interface becomes the bottleneck. Existing approaches are either too unstructured (prose CLAUDE.md files) or too programmatic (YAML prompt languages). `brief` sits in the gap: a format any developer can write in 60 seconds that any agent runtime can consume.

## License

MIT
