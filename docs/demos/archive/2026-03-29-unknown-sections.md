# Unknown Section Passthrough — Extensible Briefs Without Format Changes

*2026-03-30T04:52:48Z by Showboat 0.6.1*
<!-- showboat-id: b3549479-773e-4716-878f-607321d9b2ac -->

The `.brief.md` format now supports arbitrary sections that pass through to all emit targets with full Markdown fidelity. Users can add `## Commands`, `## Code Style`, `## Architecture`, or any other section — and it will appear in the emitted output. No parser changes needed, no new dependencies. Just write the heading and go.

This demo also covers two Claude emitter improvements: `@` references for context files and `**IMPORTANT:**` emphasis on hard constraints.

## 1. The full fixture now includes unknown sections

The test fixture `tests/fixtures/full.brief.md` now includes two unknown sections — Commands and Code Style — after the standard brief sections.

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

## 2. Claude emit: unknown sections, `@` references, and emphasis

`brief emit claude` now produces output with three improvements:

- **Unknown sections emitted** — Commands and Code Style appear at the end
- **`@` references** — context files rendered as `@docs/architecture.md` instead of backtick-wrapped paths
- **Emphasis on hard constraints** — each prefixed with `**IMPORTANT:**`

```bash
cargo run --quiet -- emit claude tests/fixtures/full.brief.md
```

```output
# Briefing: Build real-time collaborative document editor

**Stack:** TypeScript 5.4, React 18, PostgreSQL 16, Redis 7, AWS ECS

## Reference Context

Read these files for background before starting work:
- @docs/architecture.md
- @docs/api-spec.yaml
- @README.md

## Constraints

### Hard (Non-negotiable)
- **IMPORTANT:** WebSocket connections must support 10k concurrent users per node
- **IMPORTANT:** All data mutations go through event sourcing, no direct DB writes
- **IMPORTANT:** WCAG 2.1 AA compliance on all new UI components
- **IMPORTANT:** Must pass existing E2E test suite before merge

### Soft (Preferred)
- Prefer Yjs over Automerge for CRDT implementation
- Keep bundle size under 200KB gzipped for editor module
- Use server-sent events for read-only viewers when possible

### Ask First (Requires approval)
- Changes to the shared state schema
- New npm dependencies over 50KB
- Modifications to the WebSocket gateway
- Any changes to authentication flow

## Sacred Regions (Do Not Modify)
- `src/core/crdt-engine/**` — Battle-tested CRDT implementation, 2 years of edge case fixes
- `src/auth/**` — SOC2 audited authentication module
- `migrations/**` — Historical migrations must never be altered
- `e2e/` — End-to-end test suite, modify only by adding new tests

## Assumptions
- [ ] Redis pub/sub can handle cross-node message fanout at 10k users
- [ ] Yjs document size stays under 5MB for typical documents
- [x] Existing REST API can coexist with WebSocket gateway
- [ ] Browser IndexedDB is sufficient for offline draft storage

## Deliverable
Working collaborative editor with real-time cursor presence, conflict-free
concurrent editing, offline support with sync-on-reconnect, and comprehensive
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

Notice three improvements in the output:

| Before | After |
|--------|-------|
| `- \`./docs/architecture.md\`` | `- @docs/architecture.md` |
| `- WebSocket connections must...` | `- **IMPORTANT:** WebSocket connections must...` |
| Commands and Code Style silently dropped | Commands and Code Style sections emitted |

## 3. Prompt emit: unknown sections with uppercase labels

The prompt emitter formats unknown section headings in uppercase, matching its existing convention for known sections like `HARD CONSTRAINTS:` and `DO NOT MODIFY:`.

```bash
brief emit prompt tests/fixtures/full.brief.md | tail -14
```

```output
Working collaborative editor with real-time cursor presence, conflict-free
concurrent editing, offline support with sync-on-reconnect, and comprehensive
test coverage. Ship as a feature-flagged beta behind `ENABLE_COLLAB_EDITOR`.

COMMANDS:
- Build: `npm run build`
- Test: `npm test`
- Lint: `npm run lint`
- Type Check: `tsc --noEmit`

CODE STYLE:
- Use TypeScript strict mode for all new files
- Prefer functional components with hooks over class components
- Use `zod` for runtime type validation at API boundaries
```

## 4. AGENTS.md emit: unknown sections as Markdown headings

```bash
brief emit agents-md tests/fixtures/full.brief.md | tail -14
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

## 5. JSON emit: unknown sections already worked

The JSON emitter always serialized unknown sections via serde. No change was needed here, but we can confirm the structure:

```bash
brief emit json tests/fixtures/full.brief.md | python3 -c "import sys,json; d=json.load(sys.stdin); [print(f'{s[\"heading\"]}: {s[\"content\"][:50]}...') for s in d['unknown_sections']]"
```

```output
Commands: - Build: `npm run build`
- Test: `npm test`
- Lint...
Code Style: - Use TypeScript strict mode for all new files
- P...
```

## 6. Raw Markdown preservation

The key parser change: unknown sections now capture the raw Markdown substring between H2 boundaries, instead of flattening through pulldown-cmark events. This means code blocks, sub-headings, emphasis, links, and nested structure all survive intact.

Here is a brief with rich Markdown in its unknown sections:

```bash
cat /tmp/rich-unknown.brief.md
```

````output
---
stack: [Rust, PostgreSQL 16]
context: [./docs/architecture.md]
---

# Optimize database query layer

## Constraints

### Hard
- Do not break backward compatibility with v2 API

## Sacred
- `src/auth/**` — SOC2 audited authentication

## Architecture

The query layer sits between the API handlers and the database pool.

### Key Components

- `src/db/pool.rs` — Connection pool management
- `src/db/query.rs` — Query builder with **prepared statement** support
- `src/db/cache.rs` — Result cache using `lru` crate

### Data Flow

```
API Handler → QueryBuilder → ConnectionPool → PostgreSQL
                  ↓
             ResultCache
```

## Workflow

- Start with profiling: run `cargo bench --bench query_perf`
- Make small, targeted changes — one query optimization per commit
- Run the full test suite after each change

## Deliverable
Query response time under 50ms at p95 for the `/search` endpoint.
````

The `## Architecture` section contains H3 sub-headings, bold text, code spans, and a fenced code block. The `## Workflow` section has em-dash separated items. All of this should survive emission:

```bash
brief emit claude /tmp/rich-unknown.brief.md
```

````output
# Briefing: Optimize database query layer

**Stack:** Rust, PostgreSQL 16

## Reference Context

Read these files for background before starting work:
- @docs/architecture.md

## Constraints

### Hard (Non-negotiable)
- **IMPORTANT:** Do not break backward compatibility with v2 API

## Sacred Regions (Do Not Modify)
- `src/auth/**` — SOC2 audited authentication

## Deliverable
Query response time under 50ms at p95 for the `/search` endpoint.

## Architecture

The query layer sits between the API handlers and the database pool.

### Key Components

- `src/db/pool.rs` — Connection pool management
- `src/db/query.rs` — Query builder with **prepared statement** support
- `src/db/cache.rs` — Result cache using `lru` crate

### Data Flow

```
API Handler → QueryBuilder → ConnectionPool → PostgreSQL
                  ↓
             ResultCache
```

## Workflow

- Start with profiling: run `cargo bench --bench query_perf`
- Make small, targeted changes — one query optimization per commit
- Run the full test suite after each change
````

The `## Architecture` section passes through intact: H3 sub-headings (`### Key Components`, `### Data Flow`), bold text (`**prepared statement**`), code spans (`\`lru\``), and the fenced code block with the ASCII diagram all survive verbatim. Previously, all of this would have been flattened to plain text or dropped entirely.

## 7. Skill emit: unknown sections pass through too

```bash
brief emit skill tests/fixtures/full.brief.md | grep -A 20 '## Commands'
```

```output
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

## 8. All tests pass

69 tests across unit, integration, parse, and validation suites — including 6 new tests for unknown section handling.

## 8. All tests pass

69 tests across unit, integration, parse, and validation suites — including 6 new tests for unknown section handling.

```bash
cargo test 2>&1 | grep '^test result'
```

```output
test result: ok. 57 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```
