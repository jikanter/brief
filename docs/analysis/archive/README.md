# Analysis Archive

Historical analysis documents from earlier phases of the project. Kept for
context and provenance but no longer representative of the current state of
the tool or the roadmap. Links and claims in these files may be out of date;
treat them as a record of what was decided when, not as authoritative guidance.

## Contents

### Phase 1 (2026-03-20): Original multi-agent analysis

Three parallel agents evaluated the Phase 1 MVP and fed into a team-lead synthesis.

- [synthesis.md](./synthesis.md) — Team Lead Synthesis: Critical Feature Recommendations
- [context-architect.md](./context-architect.md) — Format expressiveness and completeness review
- [context-engineer.md](./context-engineer.md) — Technical pipeline flexibility and extensibility review
- [integration-engineer.md](./integration-engineer.md) — Ecosystem integration and workflow review

### Phase 2 (2026-03-29 → 2026-03-30) Obsolete features

Four parallel agents produced the input reports, then two reviewer agents
revised the synthesis. The forward-looking portions of the Phase 2 synthesis
are maintained in the live [../phase2-synthesis.md](../phase2-synthesis.md);
the archive retains only what has been shipped, removed, or downgraded.

- [obsolete-features.md](./obsolete-features.md) — Completed Tier 0 items, removed proposals (permissions.deny, subagent generation, brief composition), and downgraded items (Commands/Style parsing, MCP server, emitter trait refactor). This is the Phase 2 synthesis minus its still-live roadmap.
- [flexibility-gap.md](./flexibility-gap.md) — Data model / parser / emitter gap analysis
- [format-expressiveness.md](./format-expressiveness.md) — Gap analysis vs. full-featured CLAUDE.md
- [emit-integration-audit.md](./emit-integration-audit.md) — Feature parity and cross-ecosystem analysis
- [devops-engineer.md](./devops-engineer.md) — Operational readiness and CI/CD integration

## Current analysis

Current, forward-looking analysis lives one directory up in [../](../):

- `compiled-intent-theory.md` — Theoretical extrapolation of brief beyond the text layer
- `terse-prompt-format-research.md` — Research on terse prompt formats as a future emit target
