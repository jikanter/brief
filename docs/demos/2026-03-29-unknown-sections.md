# Unknown Section Passthrough — Extensible Briefs Without Format Changes

*2026-06-22T04:18:50Z by Showboat 0.6.1*
<!-- showboat-id: 466228af-2a13-4467-9456-04bfbe9c820e -->

The `.brief.md` format supports arbitrary `##` sections that pass through to **every** emit target with full Markdown fidelity. Add `## Commands`, `## Code Style`, `## Architecture`, or any heading the parser does not recognize — it survives to the output verbatim. No parser changes, no new dependencies: write the heading and go. This demo proves passthrough across the claude, prompt, agents-md, json, and skill targets, and shows that rich nested Markdown survives intact.

## 1. The fixture carries unknown sections

The test fixture `tests/fixtures/full.brief.md` ends with two unknown sections — `Commands` and `Code Style` — after the standard brief sections.

```bash
tail -14 tests/fixtures/full.brief.md
```

```output
test coverage. Ship as a feature-flagged beta behind `ENABLE_COLLAB_EDITOR`.

## Commands

- Build: `npm run build`
- Test: `npm test`
- Lint: `npm run lint`
- Type Check: `tsc --noEmit`

## Code Style

- Use TypeScript strict mode for all new files
- Prefer functional components with hooks over class components
- Use `zod` for runtime type validation at API boundaries
```

## 2. Claude target

The unknown `Commands` and `Code Style` sections appear at the end of the CLAUDE.md output, after the standard sections. (Standard constraints render in the current framed form — `<rules priority="required">` with `MUST:`/`PREFER:` prefixes — while unknown sections pass through as plain Markdown.)

```bash
brief --file tests/fixtures/full.brief.md emit claude | tail -16
```

```output
concurrent editing, offline support with sync-on-reconnect, and comprehensive
test coverage. Ship as a feature-flagged beta behind `ENABLE_COLLAB_EDITOR`.
</deliverable>

## Commands

- Build: `npm run build`
- Test: `npm test`
- Lint: `npm run lint`
- Type Check: `tsc --noEmit`

## Code Style

- Use TypeScript strict mode for all new files
- Prefer functional components with hooks over class components
- Use `zod` for runtime type validation at API boundaries
```

## 3. Prompt target

The prompt emitter uppercases unknown section headings, matching its convention for known sections like `HARD CONSTRAINTS:`.

```bash
brief --file tests/fixtures/full.brief.md emit prompt | tail -14
```

```output
warning: prompt output is ~567 tokens, over the 500-token budget; consider --compact
- Build: `npm run build`
- Test: `npm test`
- Lint: `npm run lint`
- Type Check: `tsc --noEmit`

CODE STYLE:
- Use TypeScript strict mode for all new files
- Prefer functional components with hooks over class components
- Use `zod` for runtime type validation at API boundaries

DELIVERABLE:
Working collaborative editor with real-time cursor presence, conflict-free
concurrent editing, offline support with sync-on-reconnect, and comprehensive
test coverage. Ship as a feature-flagged beta behind `ENABLE_COLLAB_EDITOR`.
```

## 4. AGENTS.md target

AGENTS.md keeps unknown sections as Markdown `##` headings.

```bash
brief --file tests/fixtures/full.brief.md emit agents-md | tail -14
```

```output
test coverage. Ship as a feature-flagged beta behind `ENABLE_COLLAB_EDITOR`.

## Commands

- Build: `npm run build`
- Test: `npm test`
- Lint: `npm run lint`
- Type Check: `tsc --noEmit`

## Code Style

- Use TypeScript strict mode for all new files
- Prefer functional components with hooks over class components
- Use `zod` for runtime type validation at API boundaries
```

## 5. JSON target

The JSON emitter serializes unknown sections into an `unknown_sections` array via serde — each with its `heading` and raw `content`.

```bash
brief --file tests/fixtures/full.brief.md emit json | python3 -c "import sys,json; d=json.load(sys.stdin); [print(f'{s[\"heading\"]}: {s[\"content\"][:50]}...') for s in d['unknown_sections']]"
```

```output
Commands: - Build: `npm run build`
- Test: `npm test`
- Lint...
Code Style: - Use TypeScript strict mode for all new files
- P...
```

## 6. Rich Markdown survives intact

Unknown sections capture the **raw** Markdown substring between `##` boundaries rather than flattening through the event stream. Code blocks, sub-headings, emphasis, links, and nested structure all survive. The fixture `tests/fixtures/rich-unknown.brief.md` puts H3 sub-headings, bold, code spans, and a fenced diagram inside its `## Architecture` and `## Workflow` sections:

```bash
sed -n "/## Architecture/,/## Deliverable/p" tests/fixtures/rich-unknown.brief.md
```

````output
## Architecture

The query layer sits between the API handlers and the database pool.

### Key Components

- `src/db/pool.rs` — Connection pool management
- `src/db/query.rs` — Query builder with **prepared statement** support
- `src/db/cache.rs` — Result cache using `lru` crate

### Data Flow

```
API Handler -> QueryBuilder -> ConnectionPool -> PostgreSQL
                  |
             ResultCache
```

## Workflow

- Start with profiling: run `cargo bench --bench query_perf`
- Make small, targeted changes — one query optimization per commit
- Run the full test suite after each change

## Deliverable
````

Emitting to claude preserves the `## Architecture` H3 sub-headings, bold `**prepared statement**`, the `` `lru` `` code span, and the fenced diagram verbatim:

```bash
brief --file tests/fixtures/rich-unknown.brief.md emit claude | awk "/## Architecture/{f=1} f"
```

````output
## Architecture

The query layer sits between the API handlers and the database pool.

### Key Components

- `src/db/pool.rs` — Connection pool management
- `src/db/query.rs` — Query builder with **prepared statement** support
- `src/db/cache.rs` — Result cache using `lru` crate

### Data Flow

```
API Handler -> QueryBuilder -> ConnectionPool -> PostgreSQL
                  |
             ResultCache
```

## Workflow

- Start with profiling: run `cargo bench --bench query_perf`
- Make small, targeted changes — one query optimization per commit
- Run the full test suite after each change
````

## 7. Skill target

Skills get the passthrough too — unknown sections append after the generated skill body.

```bash
brief --file tests/fixtures/full.brief.md skill emit | grep -A 6 "## Commands"
```

```output
## Commands

- Build: `npm run build`
- Test: `npm test`
- Lint: `npm run lint`
- Type Check: `tsc --noEmit`

```
