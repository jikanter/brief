# Agent Skills (agentskills.io) — cached reference

This directory caches the Agent Skills open standard at the time it was downloaded. It exists so that brief's implementation can be checked against a snapshot of the spec, rather than re-fetching the live site or relying on memory.

- **Source:** https://agentskills.io
- **Spec page:** https://agentskills.io/specification (cached as [specification.md](specification.md))
- **Quickstart:** https://agentskills.io/skill-creation/quickstart.md (cached as [quickstart.md](quickstart.md))
- **Documentation index (`llms.txt`):** https://agentskills.io/llms.txt (cached as [llms.txt](llms.txt))
- **Cached on:** 2026-05-04

## Why this is here

The user instruction in `CLAUDE.md` is: when asked to implement against a standard, download and cache it under `docs/reference/standards/`. This snapshot supports the roadmap entry on skill discovery / scaffolding / install (see `docs/analysis/phase2-synthesis.md` §P7). The "metadata boundary" — the line between what agentskills.io owns and what brief is free to extend — is taken directly from the cached `specification.md`.

## Refreshing

The standard is openly developed at <https://github.com/agentskills/agentskills>. Re-fetch the three files above when the spec version of interest changes. There is no formal version field in the standard today; date the cache and note any diffs in this README when you re-fetch.
