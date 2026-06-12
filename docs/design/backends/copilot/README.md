# GitHub Copilot Backend

**Status:** Planned (phase2-synthesis P4). Trivial base case (single Markdown file), real work for path-scoped instructions if/when brief grows scoped constraints.

## Target file formats

### Project-wide instructions
`.github/copilot-instructions.md` — plain Markdown, injected as a system-level instruction block for Copilot Chat and Copilot-powered tools across the repository. No frontmatter required.

### Path-scoped instructions
`.github/instructions/<name>.instructions.md` — Markdown with YAML frontmatter:

```yaml
---
applyTo: "**/*.ts,**/*.tsx"
---
```

A single file's `applyTo` glob determines which files trigger that instruction set. Multiple instruction files can coexist; Copilot composes matching files when the user edits a path that matches.

**Verified 2026-06-12** against [VS Code custom-instructions docs](https://code.visualstudio.com/docs/copilot/customization/custom-instructions) and [GitHub Docs](https://docs.github.com/copilot/customizing-copilot/adding-custom-instructions-for-github-copilot):
- `applyTo` is a **single string**; multiple patterns are comma-separated within it (`"**/*.ts,**/*.tsx"`), not a YAML array.
- `applyTo` is **file-level** — one glob set per `.instructions.md`. To scope different rules to different paths, write separate files. When several files match the edited path, they all stack (no guaranteed order).
- Same file-level-scope constraint as Cursor and Windsurf; brief's scoped-constraint emit must fan out one `.instructions.md` per distinct scope.

## Cross-convention discovery

The audit reports that Copilot also reads other agent convention files when present:

- `AGENTS.md` — nearest in directory tree
- `.github/CLAUDE.md`
- `.github/GEMINI.md`

[open-question] Does Copilot actually discover and read `AGENTS.md` / `.github/CLAUDE.md` / `.github/GEMINI.md` in current versions? The audit cited this but did not verify. The behavior matters for the brief emitter because if Copilot reads `CLAUDE.md` directly, the marginal value of a separate copilot emit is lower than expected — the existing claude emit already covers this user.

[open-question] Does Copilot's discovery walk up the directory tree the way Claude Code does, or only check fixed locations? Affects monorepo emit behavior.

## Implications for brief's emitter

**Base case (no scoping).** A single `.github/copilot-instructions.md` rendered identically to the `claude` target output, minus the `<brief:generated>` marker envelope. This is the "trivial wrapper" that P4 references.

**Path-scoped case.** If/when brief introduces format-level scoped constraints (see [open-questions.md](../../../open-questions.md) `[format]` scoped constraints), the natural copilot emit shifts to one `.github/instructions/<name>.instructions.md` per scope rather than a single bundled file. This is the same forcing function that affects the cursor backend — they share the underlying need for per-glob output.

[open-question] If brief adds scoped constraints, should the copilot emitter split by scope (multiple `.instructions.md` files) or always emit a single `copilot-instructions.md` and embed scope as text inside it? The split-by-scope version is more idiomatic but produces more files for users to track.

## Idiomatic register

Copilot conventions trend descriptive rather than imperative. The aggressive NEVER/MUST register that brief uses for Claude Code is likely overwrought here. The copilot emitter should render Soft constraints as plain bulleted preferences and use lighter imperatives (or none) for Hard constraints.

See [emit-quality-refinements.md](../../../analysis/emit-quality-refinements.md) §3 for the per-target tone adaptation discussion.

[open-question] Is there empirical evidence that Copilot responds differently to imperative vs. descriptive registers, or is this an inherited assumption from how the existing copilot-instructions.md community examples are written? Worth confirming before tuning the emitter to a specific register.

## Action checklist for implementation

1. Verify current Copilot documentation on `applyTo`, discovery order, and any character limits.
2. Decide whether base-case emit produces one file or zero (skip emit when an existing AGENTS.md is detected and Copilot is known to read it).
3. Match the descriptive register — do not directly reuse the claude emit output.
