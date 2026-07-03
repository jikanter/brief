# brief emit cursor — Idiomatic .cursor/rules/*.mdc Output

*2026-07-03T22:36:24Z by Showboat 0.6.1*
<!-- showboat-id: 2517180b-25c5-4486-8ec2-a3df81f86df4 -->

The `brief emit cursor` target produces a Cursor `.mdc` rule from a `.brief.md`. Cursor consumes `.cursor/rules/*.mdc` files with their own YAML frontmatter (`description`, `globs`, `alwaysApply`) — a different schema than brief's, which the emitter constructs from scratch. This demo walks through emitting to stdout, installing into `.cursor/rules/brief.mdc`, coexistence with hand-written rules, idempotency, and the design boundary: brief now has scoped constraints (shipped in P8), but the cursor emitter still bundles them into a single `alwaysApply: true` rule rather than fanning out to per-glob files.

## 1. Set up a scratch project with a brief

We start from an empty directory and write a realistic project brief — a real-time notifications feature with hard constraints (one of them **scoped** to `src/ui/**`), soft preferences, ask-first scope, a sacred region, and unvalidated assumptions.

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
- [`src/ui/**`] All notification components meet WCAG 2.1 AA

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
- [`src/ui/**`] All notification components meet WCAG 2.1 AA

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
cd /tmp/brief-cursor-demo && brief --file .brief.md emit cursor
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
- When working in `src/ui/**`: All notification components meet WCAG 2.1 AA

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

Four things worth noticing:

- **Frontmatter rebuilt, not passed through.** `description: Add real-time notifications` comes from the brief's H1 goal. `alwaysApply: true` is set because the emitter produces one always-loaded rule (see §6).
- **Scoped constraint inlined as prose.** The `src/ui/**` scope on the WCAG constraint renders as a **"When working in …"** prefix inside `## Required`. The scope is preserved in the text, not lifted into a Cursor `globs` field.
- **Descriptive register, not imperative.** Hard constraints render under `## Required` as plain bullets — no `**IMPORTANT:**` prefix. That prefix is Claude-flavored; Cursor's idiom is descriptive.
- **Validated assumptions filtered out.** Only the unvalidated `WebSocket gateway…` assumption shows under `## Verify`. The validated REST API assumption is noise once confirmed.

## 3. Install into a project

`--install` writes the rule to `<cwd>/.cursor/rules/brief.mdc`, creating the directory if needed. Unlike the CLAUDE.md and AGENTS.md installers, there are no `<brief:generated>` markers — brief owns this file end-to-end and overwrites it on every install.

```bash
cd /tmp/brief-cursor-demo && brief --file .brief.md emit cursor --install && find .cursor -type f
```

```output
Installed briefing into /private/tmp/brief-cursor-demo/.cursor/rules/brief.mdc
Installed briefing into /private/tmp/brief-cursor-demo/.cursor/rules/brief-src-ui.mdc
.cursor/rules/brief.mdc
.cursor/rules/brief-src-ui.mdc
```

## 4. Coexistence with hand-written rules

Other `.mdc` files in `.cursor/rules/` are untouched. Brief owns `brief.mdc` only — your team's hand-written rules sit alongside it without conflict. Note Cursor's `globs` is a **comma-separated string** (`globs: "src/**/*.tsx"`), not a YAML array.

```bash
cat > /tmp/brief-cursor-demo/.cursor/rules/team-style.mdc << 'EOF'
---
description: Team coding style preferences
alwaysApply: false
globs: "src/**/*.tsx"
---

# Team Style

- Prefer named exports over default exports
- Co-locate tests with source files
EOF
cd /tmp/brief-cursor-demo && brief --file .brief.md emit cursor --install && echo '---' && echo 'files in .cursor/rules:' && ls .cursor/rules/ && echo '---' && echo 'team-style.mdc preserved:' && cat .cursor/rules/team-style.mdc
```

```output
Installed briefing into /private/tmp/brief-cursor-demo/.cursor/rules/brief.mdc
Installed briefing into /private/tmp/brief-cursor-demo/.cursor/rules/brief-src-ui.mdc
---
files in .cursor/rules:
brief-src-ui.mdc
brief.mdc
team-style.mdc
---
team-style.mdc preserved:
---
description: Team coding style preferences
alwaysApply: false
globs: "src/**/*.tsx"
---

# Team Style

- Prefer named exports over default exports
- Co-locate tests with source files
```

## 5. Idempotency

Running `--install` twice is byte-identical. Brief always overwrites `brief.mdc` with a fresh emit, so re-runs never accumulate drift.

```bash
cd /tmp/brief-cursor-demo && BEFORE=$(shasum -a 256 .cursor/rules/brief.mdc | cut -d' ' -f1) && brief --file .brief.md emit cursor --install >/dev/null && AFTER=$(shasum -a 256 .cursor/rules/brief.mdc | cut -d' ' -f1) && echo "before: $BEFORE" && echo "after:  $AFTER" && [ "$BEFORE" = "$AFTER" ] && echo 'byte-identical re-install'
```

```output
before: d9e16293d6ae887e3f38bdc4c22c294a6b7e23563125cc9b06205d0a16727ba6
after:  d9e16293d6ae887e3f38bdc4c22c294a6b7e23563125cc9b06205d0a16727ba6
byte-identical re-install
```

## 6. Design boundary: why still a single bundled rule

Cursor's killer feature is per-rule glob scoping — `globs: "src/auth/**"` makes a rule activate only when editing matching files. As of P8, **brief's format does have scoped constraints**: `- [`src/ui/**`] …` carries a per-constraint glob. So the old "brief has no scope concept" rationale is retired.

But the cursor emitter deliberately stops short of fanning scope out into Cursor's native mechanism. A scoped constraint is rendered **inline as prose** — a `When working in <glob>:` prefix — inside one `alwaysApply: true` rule, so the scope survives in the text but is not lifted into a `globs` field.

| Cursor activation mode | Emitter status |
|---|---|
| `alwaysApply: true` | **Used.** One bundled rule; scopes inlined as prose. |
| `alwaysApply: false` + `globs` | Not yet. Would require splitting into per-scope `.mdc` files. |
| Description-only (model decides) | Skipped. Goal is too coarse to drive load decisions. |
| Manual `@rule-name` | Skipped. Brief has no command/skill split for cursor. |

**Why not split yet?** Cursor allows only **one `globs` set per `.mdc` file** — to key rules on different scopes you write separate files (see [design/backends/cursor/README.md](../design/backends/cursor/README.md) "One glob set per file"). A faithful scoped emit is therefore a multi-file fan-out: group constraints by scope, write one `brief-<scope>.mdc` per glob set with `alwaysApply: false` + comma-string `globs`, and leave the unscoped remainder in an always-applied base file. That install/idempotency surface (multiple owned files, deletion on scope removal) is the real work still ahead. Until then, inlining keeps every scoped constraint visible in one honest, always-loaded rule.

## 7. Cleanup

```bash
rm -rf /tmp/brief-cursor-demo && echo 'demo workspace removed'
```

```output
demo workspace removed
```
