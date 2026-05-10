# brief emit cursor — Idiomatic .cursor/rules/*.mdc Output

*2026-05-10T02:23:45Z by Showboat 0.6.1*
<!-- showboat-id: 109a26f4-40bf-4283-a43b-3f1d76ac224f -->

The `brief emit cursor` target produces a Cursor `.mdc` rule from a `.brief.md`. Cursor consumes `.cursor/rules/*.mdc` files with their own YAML frontmatter (`description`, `globs`, `alwaysApply`) — a different schema than brief's, which the emitter constructs from scratch. This demo walks through emitting to stdout, installing into `.cursor/rules/brief.mdc`, coexistence with hand-written rules, idempotency, and the design rationale for why the emitter currently produces a single bundled rule with `alwaysApply: true`.

## 1. Set up a scratch project with a brief

We'll start from an empty directory and write a realistic project brief — a real-time notifications feature with hard performance constraints, soft preferences, ask-first scope, sacred regions, and unvalidated assumptions.

```bash
mkdir -p /tmp/brief-cursor-demo && cat > /tmp/brief-cursor-demo/.brief.md << 'EOF'
---
stack: [TypeScript 5.4, React 18, PostgreSQL 16]
context: [./docs/architecture.md]
---

# Add real-time notifications

## Constraints

### Hard
- Must not degrade page load time by more than 100ms
- All notifications must be delivered within 5 seconds

### Soft
- Prefer WebSocket over polling
- Reuse the existing event-bus abstraction

### Ask First
- Changes to the notification schema

## Sacred
- `src/auth/**` — SOC2 audited authentication module
- `migrations/**` — Historical migrations, never alter

## Assumptions
- [ ] WebSocket gateway can handle 5k concurrent connections
- [x] Existing REST API supports event subscriptions

## Deliverable
Working notification system with real-time delivery, read/unread tracking, and browser push support.
EOF
cat /tmp/brief-cursor-demo/.brief.md
```

```output
---
stack: [TypeScript 5.4, React 18, PostgreSQL 16]
context: [./docs/architecture.md]
---

# Add real-time notifications

## Constraints

### Hard
- Must not degrade page load time by more than 100ms
- All notifications must be delivered within 5 seconds

### Soft
- Prefer WebSocket over polling
- Reuse the existing event-bus abstraction

### Ask First
- Changes to the notification schema

## Sacred
- `src/auth/**` — SOC2 audited authentication module
- `migrations/**` — Historical migrations, never alter

## Assumptions
- [ ] WebSocket gateway can handle 5k concurrent connections
- [x] Existing REST API supports event subscriptions

## Deliverable
Working notification system with real-time delivery, read/unread tracking, and browser push support.
```

## 2. Emit to stdout

`brief emit cursor` reads the brief and prints a Cursor-flavored `.mdc` rule. The frontmatter is Cursor's schema (`description`, `alwaysApply`), constructed from the brief's goal — not a passthrough of brief's frontmatter.

```bash
./target/debug/brief --file /tmp/brief-cursor-demo/.brief.md emit cursor
```

```output
---
description: Add real-time notifications
alwaysApply: true
---

# Add real-time notifications

**Stack:** TypeScript 5.4, React 18, PostgreSQL 16

## Context

- `./docs/architecture.md`

## Required

- Must not degrade page load time by more than 100ms
- All notifications must be delivered within 5 seconds

## Preferred

- Prefer WebSocket over polling
- Reuse the existing event-bus abstraction

## Ask First

- Changes to the notification schema

## Protected Files

- `src/auth/**` — SOC2 audited authentication module
- `migrations/**` — Historical migrations, never alter

## Verify

- WebSocket gateway can handle 5k concurrent connections

## Deliverable

Working notification system with real-time delivery, read/unread tracking, and browser push support.
```

Three things worth noticing:

- **Frontmatter rebuilt, not passed through.** `description: Add real-time notifications` comes from the brief's H1 goal. `alwaysApply: true` is set because brief currently has no glob-scoping concept; emitting an empty `globs: []` would be misleading.
- **Descriptive register, not imperative.** Hard constraints render under `## Required` as plain bullets — no `**IMPORTANT:**` prefix. That prefix is Claude-flavored; Cursor's idiom is more descriptive.
- **Validated assumptions filtered out.** Only the unvalidated `WebSocket gateway can handle 5k concurrent connections` shows up under `## Verify`. The validated REST API assumption is noise once it's confirmed.

## 3. Install into a project

`--install` writes the rule to `<cwd>/.cursor/rules/brief.mdc`, creating the directory if it doesn't exist. Unlike the CLAUDE.md and AGENTS.md installers, there are no `<brief:generated>` markers — brief owns this file end-to-end and overwrites it on every install.

```bash
cd /tmp/brief-cursor-demo && /c/Developer/Projects/brief/target/debug/brief emit cursor --install && find .cursor -type f
```

```output
Installed briefing into C:\Users\jikan\AppData\Local\Temp\brief-cursor-demo\.cursor\rules\brief.mdc
.cursor/rules/brief.mdc
```

## 4. Coexistence with hand-written rules

Other `.mdc` files in `.cursor/rules/` are untouched. Brief owns `brief.mdc` only — your team's hand-written rules sit alongside it without conflict.

```bash
cat > /tmp/brief-cursor-demo/.cursor/rules/team-style.mdc << 'EOF'
---
description: Team coding style preferences
alwaysApply: false
globs: ["src/**/*.tsx"]
---

# Team Style

- Prefer named exports over default exports
- Co-locate tests with source files
EOF
cd /tmp/brief-cursor-demo && /c/Developer/Projects/brief/target/debug/brief emit cursor --install && echo "---" && echo "files in .cursor/rules:" && ls .cursor/rules/ && echo "---" && echo "team-style.mdc preserved:" && cat .cursor/rules/team-style.mdc
```

```output
Installed briefing into C:\Users\jikan\AppData\Local\Temp\brief-cursor-demo\.cursor\rules\brief.mdc
---
files in .cursor/rules:
brief.mdc
team-style.mdc
---
team-style.mdc preserved:
---
description: Team coding style preferences
alwaysApply: false
globs: ["src/**/*.tsx"]
---

# Team Style

- Prefer named exports over default exports
- Co-locate tests with source files
```

## 5. Idempotency

Running `--install` twice is byte-identical. Brief always overwrites `brief.mdc` with a fresh emit, so re-runs never accumulate drift.

```bash
cd /tmp/brief-cursor-demo && BEFORE=$(sha256sum .cursor/rules/brief.mdc | cut -d" " -f1) && /c/Developer/Projects/brief/target/debug/brief emit cursor --install >/dev/null && AFTER=$(sha256sum .cursor/rules/brief.mdc | cut -d" " -f1) && echo "before: $BEFORE" && echo "after:  $AFTER" && [ "$BEFORE" = "$AFTER" ] && echo "byte-identical re-install"
```

```output
before: 49a6ebb4076a91bd00d2c70db27de7fa8dc261c4edb8c2cac778a6595223bcfb
after:  49a6ebb4076a91bd00d2c70db27de7fa8dc261c4edb8c2cac778a6595223bcfb
byte-identical re-install
```

## 6. Design boundary: why a single bundled rule

Cursor's killer feature is per-rule glob scoping — `globs: ["src/auth/**"]` makes a rule activate only when editing matching files. Brief's current format has no scope concept: every constraint is global to the project.

The cursor backend deliberately stops at the trivial-base-case mapping:

| Cursor activation mode | Why brief doesn't use it (yet) |
|---|---|
| `alwaysApply: true` | **Used.** The honest mapping while brief is scope-flat. |
| `alwaysApply: false` + `globs` | Skipped. Brief has no per-constraint glob hints. |
| Description-only (model decides) | Skipped. Goal is too coarse to drive load decisions. |
| Manual `@rule-name` | Skipped. Brief has no command/skill split for cursor. |

When brief itself learns scoped constraints (open question in `docs/open-questions.md`), this emitter is the first place that expressivity unlocks — splitting into multiple `.mdc` files keyed on scope. Until then, the bundled `alwaysApply: true` rule keeps the integration honest and YAGNI-clean.

## 7. Cleanup

```bash
rm -rf /tmp/brief-cursor-demo && echo "demo workspace removed"
```

```output
demo workspace removed
```
