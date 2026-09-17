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

# Machine-readable stdout targets (no --install: pipe or redirect them)
brief emit json                 # the parsed briefing as canonical JSON
brief emit xml                  # XML envelope: every section has a closing tag
brief emit xml > brief.xml      # ...so a payload appended after it cannot bleed in

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
brief_version: "1"
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

Frontmatter keys: `stack` (required), `context`, `model`, `brief_version`,
`skill_name`, `skill_description`. `brief_version` is the `.brief.md` format
version — it defaults to `"1"`, and a version this build does not understand is
a `brief validate` error. (It was spelled `version` through 0.6.x; that spelling
still parses.) Unknown keys are ignored, so newer briefs stay readable by older
tools.

The machine contract for frontmatter is
[docs/schema/brief-frontmatter-v1.schema.json](docs/schema/brief-frontmatter-v1.schema.json)
— JSON Schema Draft 2020-12, generated from the Rust types and embedded in the
binary, with a test that fails if the committed copy drifts. The canonical JSON
of a *parsed* brief (what `brief emit json` writes) is
[docs/schema/brief-v1.schema.json](docs/schema/brief-v1.schema.json), specified
in [docs/schema/SPEC.md](docs/schema/SPEC.md).

`brief validate` also reads the frontmatter of the brief and of each local
`context:` document and reports what their provenance keys say: a `isBasedOn`
or `superseded_by` pointing at a missing file, a context document that has been
superseded, a `dateModified` older than the file's last commit. All of it is
advisory — these keys live in documents brief did not write, so none of it is an
error and none of it changes the exit code. `brief validate --hints` adds the
one check that is noise on a long-lived brief: a context document edited more
recently than the brief pointing at it. The vocabulary is specified in
[docs/design/provenance-schema.md](docs/design/provenance-schema.md).

`emit xml` exists for one reason: a Markdown heading ends only when the next
heading appears, so a briefing concatenated with untrusted or bulky content (a
diff, a file bundle, tool output) inside a CI prompt has no section terminus.
`</sacred>` does. Every authored value is escaped, so `Record<T>`, `a && b`, and
fenced code blocks survive intact and cannot close a tag early. It is not a
token optimization — the wrapper costs more than it saves; `--budget` and
`--compact` are where savings live.

See [examples/sample.brief.md](examples/sample.brief.md) for a complete example. Also see [tests/fixtures](tests/fixtures/) for the tested examples of well-formed and malformed brief documents.

## Why?

As AI agents handle more technical execution, the human→agent interface becomes the bottleneck. Existing approaches are either too unstructured (prose CLAUDE.md files) or too programmatic (YAML prompt languages). `brief` sits in the gap: a format any developer can write in 60 seconds that any agent runtime can consume. If built correctly, brief should enhance the implementation of existing formats without impeding their inherent structure.

## See Also

Simon Willison's [Showboat](https://github.com/simonw/showboat), which effectively solves the inverse of what I am 
trying to solve here, which is to ease the burden of human's communicating with agents.

## License

MIT
