# Brief - Structured Briefings for AI Agents

*Very Important* Agents: Read [Claude.md](./CLAUDE.md) to better understand the projects

Brief is a structured briefing format for AI agents. It is designed to be human-writable, opinionated,
and more heavily structured than other formats. It includes a CLI to validate, compose, and emit 
briefings non-destructively, leveraging context-dependent pattern matching to ensure 
interoperability.

`brief` provides a fast, familiar format (`.brief.md`) for humans to express intent, constraints, commands, and sacred code regions to AI agents — and a CLI to validate, compose, and emit those briefings to multiple agent runtimes.

## Quick Start

```bash
# Initialize a briefing from your repo
brief init

# Validate against your codebase
brief validate

# Emit for Claude Code (idempotent install into CLAUDE.md)
brief emit claude --install
brief emit claude --install --position top   # primacy placement + reconciliation preamble
brief emit claude --full                     # section + skill + hooks + command permissions
brief emit claude --uninstall                # reverse it all (section, hooks, skill)

# Emit for OpenAI Codex / cross-vendor agents (idempotent install into AGENTS.md)
brief emit agents-md --install

# Emit for other ecosystems, each in its idiomatic register
brief emit cursor --install     # .cursor/rules/brief.mdc
brief emit copilot --install    # .github/copilot-instructions.md
brief emit windsurf --install   # .windsurf/rules/brief.md
brief emit aider --install      # CONVENTIONS.md + .aider.conf.yml

# Stay under the context-window budget
brief emit claude --budget      # report estimated tokens/chars (to stderr)
brief emit prompt --compact     # strip reference prose; keep only the essentials

# Compact, re-injectable constraint anchor (NEVER/MUST framed, <=5 lines)
brief emit anchor

# Lint the briefing (flags vague constraints; --install warns on conflicts)
brief validate

# Check if a path is sacred
brief check src/auth/handler.rs

# Author agent skills (hand-editable; brief owns only metadata.brief.source)
brief skill search review                          # find existing local skills
brief skill scaffold --description "Review PRs"    # or --from-brief ./x.brief.md
brief skill install ./review                        # idempotent into .claude/skills/
brief skill uninstall review
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

See [examples/sample.brief.md](examples/sample.brief.md) for a complete example. Also see [tests/fixtures](tests/fixtures/) for the tested examples of well-formed and malformed brief documents.

## Why?

As AI agents handle more technical execution, the human→agent interface becomes the bottleneck. Existing approaches are either too unstructured (prose CLAUDE.md files) or too programmatic (YAML prompt languages). `brief` sits in the gap: a format any developer can write in 60 seconds that any agent runtime can consume. If built correctly, brief should enhance the implementation of existing formats without impeding their inherent structure.

## See Also

Simon Willison's [Showboat](https://github.com/simonw/showboat), which effectively solves the inverse of what I am 
trying to solve here, which is to ease the burden of human's communicating with agents.

## License

MIT
