# Brief Agents

<brief:generated>
# Brief: the source of record for human intent to AI systems

**Stack:** Rust

## Context

Refer to these files for background:

- `./README.md`
- `./docs/architecture/README.md`

## Instructions

- A new `.brief.md` takes under sixty seconds to author. **(REQUIRED)**
- brief declares and emits files; it never executes. No model calls, sessions or tool loops, and no network crates (`tokio`, `reqwest`, `hyper`, `ureq`). **(REQUIRED)**
- Distributed as a single binary, or a single binary plus configuration files. **(REQUIRED)**
- Emitted files fit each target's existing conventions (`AGENTS.md`, `CLAUDE.md`, `.cursor/rules/`, aichat roles, promptfoo providers) and never replace hand-authored content. **(REQUIRED)**
- Installs are non-destructive, idempotent and reversible: generated content lives inside `<brief:generated>` markers or in files brief owns, and `--uninstall` removes exactly that. **(REQUIRED)**
- The same brief emits byte-identical output on every run: no timestamps or random values in emitted files. aichat hashes an Entity's definition into its `lock_hash`, so unstable output reads as a new version. **(REQUIRED)**
- The seams with the sibling repos (aichat, astrophage, llm-functions, rocky, pi) are emitted files only. A change a sibling repo needs is documented in this repo and applied in that one. **(REQUIRED)**
- The format extends through unknown `##` sections and new emit targets, without a grammar change. **(REQUIRED)**
- Keep the format low cognitive load, not merely lower burden than prose: an author should not need `docs/brief-format.md` open to write a valid brief. *(preferred)*
- Dogfood brief: this file is its first use case, and `brief emit claude --install` generates this repo's `CLAUDE.md` section from it. *(preferred)*
- A new frontmatter field clears the YAGNI bar in `docs/design/frontmatter-additions.md` before it is built. *(preferred)*
- Adding an emit target for a runtime brief does not support yet. **(ASK FIRST)**
- Changing the `.brief.md` grammar or bumping `brief_version`. **(ASK FIRST)**

## Protected Files

- `docs/schema/brief-frontmatter-v1.schema.json`: generated from the Rust Frontmatter type; regenerate with BLESS_SCHEMA=1 cargo test --test frontmatter_schema_tests, never edit by hand

## Deliverable

A human-authorable format, and one CLI that checks it, compiles it, installs it reversibly and gates work against it, for every runtime that consumes intent: briefings for the coding agents (`AGENTS.md` for Codex, Cursor, Copilot and Claude Code; `CLAUDE.md` with hooks and skills; Windsurf; Aider) and definitions for the integrated stack (an aichat role, promptfoo replay against astrophage). Secondarily, the format and the target set stay extensible, so a new runtime is a new emitter rather than a new format.

## Emit Targets

`brief emit <target>` compiles a `.brief.md` into one runtime's native artifact. The full reference is `docs/emit-targets-reference.md`.

- `claude` — a `CLAUDE.md` section in Claude Code conventions. The primary target.
- `agents-md` — an `AGENTS.md` section.
- `cursor`, `copilot`, `windsurf`, `aider`, `readme` — each ecosystem's native file, in its own register.
- `prompt` — raw system prompt text for API use.
- `anchor` — a ≤5-line NEVER/MUST constraint anchor for mid-session re-injection.
- `json` — the parsed briefing as structured JSON, with its content-hash `id` (`docs/schema/SPEC.md` §7).
- `jsonl` — one JSON line per brief: the `id` plus the goal, deliverable and constraints it hashes. Lines append into a stream that training data joins on `brief_id`.
- `xml` — an XML envelope with closing tags, for prompts that concatenate untrusted content.
- `promptfoo` — promptfoo providers that replay the brief's cassette sets through the astrophage gateway (base URL plus `x-aichat-cache-mode` and `x-aichat-cassette` headers). Every provider also sends `x-brief-id`. `--gateway <address>` overrides the default `127.0.0.1:8080`. `--session-id` switches to promptfoo `http` providers that send `X-Eridian-Session-Id` per test case. Stdout only; brief never runs astrophage or promptfoo. Mode selection is astrophage-side: aichat has no cassette flags, by decision.
</brief:generated>
