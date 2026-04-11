# Aider Backend

**Status:** Planned (phase2-synthesis P4). Listed as "trivial" in the live roadmap, but the integration audit found this characterization is incomplete — Aider is genuinely a two-file emit if the user wants conventions to auto-load.

## Target file structure

Aider uses two files that work together:

| File | Purpose |
|---|---|
| `CONVENTIONS.md` | Freeform Markdown with bulleted natural-language preferences and style guidance |
| `.aider.conf.yml` | YAML config; carries `read: CONVENTIONS.md` to auto-load conventions per session |

Without the config-file side, `CONVENTIONS.md` is not automatically loaded — the user has to type `/read CONVENTIONS.md` manually each session.

## .aider.conf.yml fields relevant to brief

```yaml
read: CONVENTIONS.md
model: claude-opus-4-6
```

The `model:` key is where brief's existing `model` frontmatter field naturally lands. This is the only structured config that brief currently has anywhere to go in this ecosystem — neither Cursor nor Windsurf has an analogous "preferred model" config that brief can write.

[open-question] Should brief's aider emit always write `model:` if the source `.brief.md` declares one, or only when explicitly requested? Writing it could conflict with a user's existing aider config that sets a different model.

## Why "trivial" understates the work

Phase 2 synthesis P4 budgets aider as a trivial wrapper — same effort as copilot or windsurf. The audit's correction:

A "complete" aider emit needs to write or patch **both** files:
1. `CONVENTIONS.md` with brief's content
2. `.aider.conf.yml` with `read: CONVENTIONS.md` (creating the file or merging into an existing one)

A single-file emit (CONVENTIONS.md only) is technically simpler but produces a half-integration: the file sits there until the user manually `/read`s it, which most users will forget.

[open-question] Should the P4 aider emitter aim for the half-integration (CONVENTIONS.md only, document the manual `/read` step in user-facing output) or the full integration (touch `.aider.conf.yml`)? The full version is more useful but touches a second config file, which expands blast radius.

[open-question] If brief writes `.aider.conf.yml`, how should it merge with an existing user config? The same problem `--install` solves for CLAUDE.md (idempotent injection between markers), but YAML doesn't support comment-style markers as cleanly as Markdown does. Need a different idempotency mechanism.

## Idiomatic register

Aider's idiomatic CONVENTIONS.md style is **conversational** — bulleted natural-language preferences, not imperative directives. Brief's NEVER/MUST/PREFER/STOP register (designed for Claude Code) is meaningfully wrong here. The aider emitter should:

- Render `### Hard` constraints as plain statements ("Use Result<T, AppError> for error handling.") not `MUST:` or `NEVER:` prefixes
- Render `### Soft` constraints as preferences ("Prefer small focused commits.") without `PREFER:` markers
- Render `### Ask First` items as "Ask before:" prefixes (Aider supports interactive flows naturally)

See [emit-quality-refinements.md](../../../analysis/emit-quality-refinements.md) §3 for the broader per-target tone discussion.

## Mapping brief content to aider's structure

| Brief field | Aider destination |
|---|---|
| `model` (frontmatter) | `.aider.conf.yml` `model:` |
| Goal (H1) | CONVENTIONS.md top-of-file paragraph |
| Hard / Soft / Ask First constraints | CONVENTIONS.md bulleted lists |
| Sacred regions | CONVENTIONS.md "Files not to modify" section |
| Stack | CONVENTIONS.md preamble |
| Context files | Not directly supported; could be appended as a "Reference" section |

[open-question] Does Aider have a native concept analogous to brief's `context: [./file.md]` frontmatter — a way to declare "always include these files in the context"? If so, that maps to `.aider.conf.yml` rather than CONVENTIONS.md and changes the emit shape.

## Action checklist for implementation

1. Decide scope: half-integration (CONVENTIONS.md only) or full integration (also `.aider.conf.yml`).
2. If full integration: design an idempotent YAML merge strategy.
3. Implement the conversational register — do not reuse claude emit output.
4. Verify Aider's current convention-loading mechanics against the docs before shipping.
