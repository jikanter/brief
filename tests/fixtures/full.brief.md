---
stack: [TypeScript 5.4, React 18, PostgreSQL 16, Redis 7, AWS ECS]
context: [./docs/architecture.md, ./docs/api-spec.yaml, ./README.md]
model: claude-sonnet-4-20250514
version: "1"
---

# Build real-time collaborative document editor

## Constraints

### Hard
- WebSocket connections must support 10k concurrent users per node
- All data mutations go through event sourcing, no direct DB writes
- WCAG 2.1 AA compliance on all new UI components
- Must pass existing E2E test suite before merge

### Soft
- Prefer Yjs over Automerge for CRDT implementation
- Keep bundle size under 200KB gzipped for editor module
- Use server-sent events for read-only viewers when possible

### Ask First
- Changes to the shared state schema
- New npm dependencies over 50KB
- Modifications to the WebSocket gateway
- Any changes to authentication flow

## Sacred
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
