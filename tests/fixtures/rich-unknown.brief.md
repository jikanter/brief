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
API Handler -> QueryBuilder -> ConnectionPool -> PostgreSQL
                  |
             ResultCache
```

## Workflow

- Start with profiling: run `cargo bench --bench query_perf`
- Make small, targeted changes — one query optimization per commit
- Run the full test suite after each change

## Deliverable
Query response time under 50ms at p95 for the `/search` endpoint.