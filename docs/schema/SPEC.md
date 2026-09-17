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
| `stack:` | `/frontmatter/stack` | `string[]`. Required, min length 1 when valid. |
| `context:` | `/frontmatter/context` | Paths or URLs. Optional. |
| `model:` | `/frontmatter/model` | Preferred model id. `null` if omitted. Not a session/runtime field. |
| `brief_version:` | `/frontmatter/brief_version` | Defaults to `"1"`. Legacy spelling `version:` still deserializes; canonical JSON only ever carries `brief_version`. |
| `skill_name:` / `skill_description:` | `/frontmatter/skill_*` | Optional; used by skill emit. `skill_name` is kebab-case. |
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

Additive, backward-compatible JSON fields may appear in a later 1.x; breaking changes require a new schema id (`brief-v2.schema.json`) and a new `brief_version` value.

The key was spelled `version` through 0.6.x. It collided with the project version `brief init` writes under `metadata:`, so it was renamed; `version:` remains a deserialization alias and is listed in `$defs.FrontmatterAuthoring`.

### 6.1 Regenerating the frontmatter schema

`brief-frontmatter-v1.schema.json` is generated, never hand-edited:

```sh
BLESS_SCHEMA=1 cargo test --test frontmatter_schema_tests
```

`tests/frontmatter_schema_tests.rs` fails if the committed file differs from the schema derived from `Frontmatter`, or from the copy embedded via `include_str!`.
