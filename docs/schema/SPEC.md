# Brief format — machine schema (v1)

**Status:** Canonical companion to [brief-format.md](../brief-format.md).  
**Format version:** `1`  
**Machine schema:** [brief-v1.schema.json](brief-v1.schema.json) (JSON Schema Draft 2020-12)  
**Frontmatter schema:** [brief-frontmatter-v1.schema.json](brief-frontmatter-v1.schema.json) — *derived* from the Rust `Frontmatter` type (schemars), embedded in the binary, drift-tested

The authoring format is Markdown with YAML frontmatter. This document specifies the **canonical JSON** that a parsed `.brief.md` occupies — the object `brief emit json` writes, and the contract emitters and other tools consume.

```
.brief.md  --parse-->  Brief (this schema)  --emit-->  CLAUDE.md / AGENTS.md / …
```

## 1. Two layers

| Layer | What it is | Schema |
|---|---|---|
| **Authoring** | `.brief.md` file: `---` YAML `---` then Markdown headings | Frontmatter: [brief-frontmatter-v1.schema.json](brief-frontmatter-v1.schema.json) (generated; `$defs.FrontmatterAuthoring` here mirrors it for single-file consumers). Body: heading grammar in [brief-format.md](../brief-format.md) §3. |
| **Canonical JSON** | Strongly-typed `Brief` after parse | Root of `brief-v1.schema.json` |

The parser is **tolerant**: unrecognized H2s become `unknown_sections`; missing H1 or `stack` still parse. **Validity** is a second pass (`brief validate` and this schema's `ValidityConstraints`): a well-formed brief has a non-empty `stack`, a non-empty `goal`, well-formed sacred entries, and checkbox assumptions.

Root `$ref` composition:

- `$defs.ParsedBrief` — serde shape of `brief emit json` (including empty/`null` fields).
- `$defs.ValidityConstraints` — the error band for malformed files.
- The document root is `allOf` both: **a valid Brief**.

## 2. Authoring → JSON map

| Authoring | JSON path | Notes |
|---|---|---|
| *(derived, never authored)* | `/id` | `sha256:` plus 64 lowercase hex digits: a content hash of `goal`, `deliverable` and `constraints` (§7). |
| `stack:` | `/frontmatter/stack` | `string[]`. Required, min length 1 when valid. |
| `context:` | `/frontmatter/context` | Paths or URLs. Optional. |
| `model:` | `/frontmatter/model` | Preferred model id. `null` if omitted. Not a session/runtime field. |
| `brief_version:` | `/frontmatter/brief_version` | Defaults to `"1"`. Legacy spelling `version:` still deserializes; canonical JSON only ever carries `brief_version`. |
| `skill_name:` / `skill_description:` | `/frontmatter/skill_*` | Optional; used by skill emit. `skill_name` is kebab-case. |
| `cassettes:` | `/frontmatter/cassettes` | Optional astrophage cassette set names. Omitted from JSON when empty. A `## Fixtures` section stays in `/unknown_sections[]`. |
| `# …` (H1) | `/goal` | Exactly one. |
| `## Identity` | `/identity` | Optional. `{ heading: "Identity", content }`. `null` if absent. |
| `## Constraints` / `### Hard` | `/constraints/hard` | Array of `Constraint`. |
| `### Soft` | `/constraints/soft` | |
| `### Ask First` | `/constraints/ask_first` | |
| Constraint item `` [`glob`] text `` | `Constraint.scope` | Omitted from JSON when empty (project-wide). |
| `## Sacred` `` `glob` — reason `` | `/sacred[]` | `{ path, reason, well_formed }`. |
| `## Assumptions` `- [ ]` / `- [x]` | `/assumptions[]` | `{ text, validated, has_checkbox }`. |
| `## Deliverable` | `/deliverable` | Prose string, or `null`. |
| Other `##` headings | `/unknown_sections[]` | `{ heading, content }` raw Markdown. |

Unrecognized YAML keys are **ignored** (forward-compatible). They do not appear in canonical JSON, and any command that rewrites the file conserves them byte-identical. New frontmatter fields must pass the [YAGNI bar](../design/frontmatter-additions.md).

Document-provenance keys (`dateModified`, `isBasedOn`, …) are such unrecognized keys as far as this schema is concerned. They are specified separately in [provenance-schema.md](../design/provenance-schema.md) §1.1 and are not part of `Frontmatter` or of canonical JSON.

## 3. Constraint object

```json
{ "text": "Must pass CI" }
{ "text": "WCAG 2.1 AA", "scope": ["src/ui/**"] }
```

- `text` is the authored sentence. Emitters reframe (RFC-2119, etc.); the schema does not store emit register.
- `scope` is a non-empty list of globs, or omitted.

Tiers (normative, for emit mapping):

| Tier | Meaning | Typical emit register |
|---|---|---|
| `hard` | Non-negotiable; comply or abort | `MUST` / `NEVER` |
| `soft` | Preferred unless there is a good reason not to | `PREFER` / `AVOID` |
| `ask_first` | Stop and get human approval before proceeding | `STOP and confirm before` |

## 4. Validity error band

A document that parses but fails this schema (and `brief validate` errors) is in this band:

| Condition | JSON Schema | `brief validate` |
|---|---|---|
| Empty / missing `stack` | `/frontmatter/stack` `minItems: 1` | Error: missing `stack` |
| Missing H1 | `/goal` `minLength: 1` | Error: missing H1 |
| Sacred not `` `path` — reason `` | `/sacred/n/well_formed` `const: true` | Error: malformed sacred entry |
| Assumption without `- [ ]` / `- [x]` | `/assumptions/n/has_checkbox` `const: true` | Error: missing checkbox |

Filesystem checks (context path exists, sacred/scope globs match files) and lints (vague constraints) are **not** in this schema; they require a repo.

## 5. Non-goals (schema guardrails)

The schema describes **task-specific structured intent**. It does not model execution:

- No session, tool-loop, decoding, or MCP/tool-I/O fields.
- `additionalProperties: false` on the canonical `Brief` object. Execution-shaped keys cannot be smuggled through JSON emit.
- YAML unknown keys remain ignored at parse time (authoring forward-compat); they are dropped, not promoted into the machine model.

`model` in frontmatter is a **preferred model identifier** for the task, not runtime session config.

## 6. Versioning

The format version is the `brief_version:` frontmatter key. Current value: `"1"`; omitted means `"1"`.

`brief validate` **errors** on a `brief_version` this build does not understand (`Unsupported \`brief_version\`: \`2\` (this build understands \`1\`)`), exit 1 — the E1-S2 error band. Parsing stays tolerant: a newer brief still parses, it just does not validate, so tooling can read it without emitting it wrongly.

Additive, backward-compatible JSON fields may appear in a later 1.x (`/id` is the first, added 2026-10); breaking changes require a new schema id (`brief-v2.schema.json`) and a new `brief_version` value.

The key was spelled `version` through 0.6.x. It collided with the project version `brief init` writes under `metadata:`, so it was renamed; `version:` remains a deserialization alias and is listed in `$defs.FrontmatterAuthoring`.

### 6.1 Regenerating the frontmatter schema

`brief-frontmatter-v1.schema.json` is generated, never hand-edited:

```sh
BLESS_SCHEMA=1 cargo test --test frontmatter_schema_tests
```

`tests/frontmatter_schema_tests.rs` fails if the committed file differs from the schema derived from `Frontmatter`, or from the copy embedded via `include_str!`.

## 7. The id

`/id` is a content hash of the brief's **objective**. A consumer that runs work under a brief records the id, and the work can later be joined back to what was asked: aichat stores it as `session.start.brief_id` and exports it with a trajectory. The id is derived at emit time. It is never authored, and there is no `id:` frontmatter key.

Because other repos recompute it, the id is defined over bytes, not over this crate's types.

### 7.1 Objective

Three fields of the canonical JSON, exactly as `brief emit json` prints them:

```json
{ "goal": …, "deliverable": …, "constraints": { "hard": […], "soft": […], "ask_first": […] } }
```

A constraint is `{ "text": … }`, with `"scope": […]` only when it is scoped (§3).

All three are always present, in the parsed brief's own spelling of "missing":

| Brief lacks | Objective carries | Why not `null` everywhere |
|---|---|---|
| `## Deliverable` | `"deliverable": null` | It is `null` in canonical JSON. |
| `## Constraints`, or a tier | `"constraints"` with all three tiers, the missing ones `[]` | The parsed brief always has the three arrays. |
| The H1 | `"goal": ""` | The parser stores `""`, and `brief validate` rejects the brief. It still hashes. |

So the objective is a projection of `brief emit json`, and a mirror needs nothing else: pick the three fields, canonicalize, hash.

### 7.2 Canonical form

1. Object keys sorted by Unicode code point, at every depth. (Every key in the objective is ASCII.)
2. No whitespace between tokens.
3. Strings in UTF-8, escaped only where JSON requires it: `\"`, `\\`, the short forms `\b` `\f` `\n` `\r` `\t`, and lowercase `\u00xx` for the other characters below U+0020. Nothing else is escaped: non-ASCII characters, `/` and U+007F are written as themselves.
4. Array order is kept. The order of constraints within a tier is part of the objective.
5. No Unicode normalization: strings are hashed as parsed.

The objective holds only strings, arrays, objects and `null`, and for those this is RFC 8785 (JCS). In Python it is `json.dumps(objective, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode("utf-8")`.

The id is `sha256:` followed by the lowercase hex SHA-256 of those bytes.

### 7.3 What moves the id

| Changes the id | Does not |
|---|---|
| The goal or the deliverable text | Frontmatter (`stack`, `context`, `model`, `cassettes`, …) |
| A constraint's text, scope, tier, or position within its tier | `## Identity`, `## Sacred`, `## Assumptions`, unknown sections |
| | The order sections are written in, list-marker style, blank lines |
| | `--compact` |

### 7.4 Golden pair

[`tests/fixtures/brief-id.brief.md`](../../tests/fixtures/brief-id.brief.md) has this objective, stored without a trailing newline in [`tests/fixtures/golden/brief-id.objective.json`](../../tests/fixtures/golden/brief-id.objective.json) (315 bytes):

```json
{"constraints":{"ask_first":[{"text":"Adding a dependency"}],"hard":[{"text":"Never touch `unsafe` blocks"},{"scope":["src/ui/**"],"text":"WCAG 2.1 AA"}],"soft":[{"text":"Prefer small diffs"}]},"deliverable":"A review comment per finding.\nEach names a file and a line.","goal":"Review Rust changes — \"safely\""}
```

and this id (`shasum -a 256` of that file):

```
sha256:2b939f8f5f0c3337e269db05822810adb2402b3d2405c43f161f9983f294851a
```

`tests/brief_id_tests.rs` pins both.

### 7.5 Where the id travels

| Carrier | Form |
|---|---|
| `brief emit json` | `/id` |
| `brief emit jsonl` | One line per brief: the canonical JSON of the objective plus `id`. Drop `id` from a parsed line, write it back in canonical form, and you have the hashed bytes. |
| `brief emit promptfoo` | Request header `x-brief-id` on every provider; `AICHAT_BRIEF_ID=<id>` for a CLI provider ([brief-format.md](../brief-format.md) §8.3). |
