# Brief Provenance Vocabulary v0.1

**Status:** Implemented in 0.6.2 (`src/provenance.rs`, `src/provenance_check.rs`, `brief validate`)
**Scope:** `brief` reads, validates, and conserves document provenance in YAML frontmatter. `brief` does not author provenance for documents it did not generate.
**Companions:** [brief-format.md §2](../brief-format.md) and [schema/SPEC.md](../schema/SPEC.md) (the `.brief.md` `Frontmatter` schema), [frontmatter-additions.md](frontmatter-additions.md) (the YAGNI bar). §1.1 states how they fit together.

---

## 1. Purpose

Machine-checkability and a fixed place for a human to read a document's history. Not LLM comprehension.

An LLM parses `**Last updated:** 2026-04-11` in a prose header perfectly well. What prose cannot do is:

- be asserted on in CI
- be diffed meaningfully
- have its referenced paths checked for existence
- order a set of documents by staleness without a model pass

This vocabulary exists so `brief validate` can make those checks. Every consumer is in `validate`: brief emits context docs by path and never inlines them, so `emit` has no provenance to carry or trim (§7). A field that no `brief` code path consumes is marked **advisory**.

`dateModified` is the exception that is kept for the human as much as for the tool. Inside a git repo, git already knows when a file changed; the field earns its place in trees git does not cover (Obsidian vaults, SharePoint folders, Hermes skill dirs) and as a date a reader can see without running anything. It is one of the few fields in brief that is not strictly for agents.

### Non-goals

- Authoring provenance. Brief is a human→agent format; emitting provenance onto AI-generated documents is the inverse problem and out of scope.
- Specifying a content model. Following OKF's restraint, nothing is required; four keys are checked when present and the rest are advisory.
- Being JSON-LD. See §8 for compatibility-without-adoption.
- Replacing or competing with SKILL.md / AGENTS.md / `.agent.md` frontmatter. Those are *instruction* frontmatter (routing, tool permissions, orchestration). This is *provenance* frontmatter. They coexist in the same block.

### 1.1 Relationship to the `.brief.md` frontmatter schema

Two vocabularies, one YAML block, no overlap.

| | `Frontmatter` (v1) | Provenance (this doc) |
|---|---|---|
| Describes | the task | the document's history |
| Applies to | `.brief.md` only | any Markdown file brief reads: `context:` docs, SKILL.md, a `.brief.md` itself |
| Keys | `stack`, `context`, `model`, `brief_version`, `skill_name`, `skill_description` | §3 |
| Machine schema | [brief-frontmatter-v1.schema.json](../schema/brief-frontmatter-v1.schema.json), derived from the Rust type | none yet; this doc is the spec |
| Admission rule | six-check [YAGNI bar](frontmatter-additions.md) | §1: a consumer in `brief`, or the field is advisory |

Consequences:

- Provenance keys are **not** `Frontmatter` fields. They are not added to the Rust `Frontmatter` type, do not appear in the derived schema or in canonical JSON, and do not bump `brief_version`.
- In a `.brief.md` they are legal today (`additionalProperties: true`; SPEC §2 "unrecognized keys are ignored"). The brief parser ignores them; the provenance pass reads them.
- "Ignored" (SPEC, brief-format) and "conserved" (§5 rule 6) are the same policy seen from two sides: ignored on read into the `Brief` model, conserved byte-identical on any write back to the file.
- The YAGNI bar does not apply: its check 1 (task-specific) is about task fields, and these describe documents. It still governs every addition to `Frontmatter`.
- In a `.brief.md`, top-level `version` is the legacy alias of `brief_version`. Provenance never reads `version`; an exporter-written `version: 12` is a `brief validate` error only if the file is parsed as a brief.

---

## 2. The block

A document a human maintains:

```yaml
---
# ═══ optional, machine-validated ═══
dateModified: 2026-04-11T14:22:00-05:00
isBasedOn: ./architecture.brief.md
superseded_by: null

# ═══ optional, advisory only ═══
dateCreated: 2026-02-23
generator: multi-agent-synthesis

# ═══ read-only, never written, never validated, never reordered ═══
tags: [architecture, pipeline]
---
```

A file brief wrote (today: a generated `SKILL.md`). Brief's provenance lives only under `metadata.brief.*`:

```yaml
---
name: review
description: Review code
metadata:
  brief.source: ../../review.brief.md
  brief.dateModified: "2026-04-11T14:22:00-05:00"
---
```

---

## 3. Field reference

| Field | Type | Consumer in brief | Status |
|---|---|---|---|
| `dateModified` | ISO 8601 datetime \| date | `validate` drift vs. git; opt-in hint when newer than a referencing brief (§6) | Optional, validated |
| `isBasedOn` | path \| URL | `validate` path existence | Optional, validated |
| `superseded_by` | path \| URL \| null | `validate` dangling / chain check; warning when a brief's `context:` lists the doc | Optional, validated |
| `dateCreated` | ISO 8601 datetime \| date | none | Advisory |
| `generator` | string | none | Advisory |
| `tags` | list | none | Read-only passthrough |
| `metadata` | map, string keys → string values | `brief.*` keys (rule 1) | Conserved passthrough; brief owns only `brief.*` |

`superseded_by: null` and an absent key mean the same thing.

### Paths and URLs

- A path is relative to the directory of the document that carries the key. Same rule `context:` follows against the brief's directory.
- A value with a URI scheme is a URL: `https://…`, or an ACE tag URI `ace://prefix/id`. URLs get no existence check and are never fetched (brief makes no network calls). Brief checks only that the value parses as `scheme://rest`; it does not know the ACE prefix list or the tag registry, which are versioned outside brief.

### Deliberately excluded

**`status`.** Duplicates information already carried by `superseded_by`. Two sources of truth for one fact is the exact failure mode this vocabulary replaces in the prose header. Derive it.

**`archive`.** Its only consumer was a path-existence check, and the path it named is also a link in the document body. A relative-link check over the body covers it and every other link without a field.

**`revisions`.** No consumer, largest field by bytes, and git already holds the history.

Keys with these names appear in the wild (Obsidian, Confluence exports, static-site generators). They are conserved silently. See §5, rule 6.

### Naming rule

> **Borrowed names keep their source spelling. Brief-native names are lowercase, snake_case when more than one word** (`brief_version`, `skill_name`, `superseded_by`).

`dateModified`, `dateCreated`, and `isBasedOn` are literal schema.org tokens and keep that spelling for one reason: §8, decision 3. A canonical name that is byte-equal to its source token makes a future `@context` a pure addition instead of a rename. That is the whole justification; no claim is made about how any model reads the names.

`superseded_by` and `generator` are brief-native and follow house style. schema.org's `supersededBy` is defined on Enumeration/Class/Property, not CreativeWork, so borrowing its spelling would claim a mapping that does not exist. No standards-conformance claim is made for either.

Every **alias** (§4) is bare lowercase or snake_case. Nobody hand-writes `dateModified`.

---

## 4. Aliases and resolution

### Registered aliases

A closed list. A spelling not on it is a foreign key.

```
dateModified  ← date_modified, last_modified, lastmod, modified, updated, updated_at
dateCreated   ← date_created, created, created_at
isBasedOn     ← is_based_on
superseded_by ← supersededBy
```

- `updated_at` / `created_at`: Confluence→markdown exporters and other ingest pipelines.
- `lastmod`: Hugo. `last_modified`, `modified`, `updated`: Obsidian plugins and Jekyll-family sites.
- `supersededBy`: the spelling this draft used before v0.1 settled.

There is no separator- or case-insensitive matching. A closed list can be written as serde aliases and as a JSON Schema; a fuzzy rule cannot, and it is a synonym table with no last row. Aliases are never applied inside `metadata`.

### Resolution order

```
dateModified  → aliases in the order above → metadata.brief.dateModified → unset
dateCreated   → aliases in the order above                               → unset
isBasedOn     → is_based_on                → metadata.brief.source       → unset
superseded_by → supersededBy                                             → unset
```

`metadata.brief.source` is the namespaced spelling of `isBasedOn`: one fact, and a file carries it at one address depending on who wrote the file.

Canonical form wins on conflict. Top-level always beats `metadata.brief.*`. **Never merge the two addresses.** Disagreement between two resolvable addresses is a `validate` **warning**.

**Unset is never an error.** Requiring provenance would make `brief init` hostile on every pre-existing repo doc, vault, or docs tree.

---

## 5. Rules

1. **Brief writes provenance only under `metadata.brief.*`.** Never top-level, in any file. It is where the Agent Skills spec sends tool-owned data (closed top-level field list, `metadata` for the rest, "make your key names reasonably unique"), it is how other agent tooling namespaces its own keys, and it gives one enforcement site. Keys today: `brief.source`, `brief.dateModified`. Every other `metadata` key is read-only passthrough. `brief init` scaffolds `metadata.author` / `metadata.version` once; after that they belong to the author. Top-level provenance keys are the human's; brief reads them and never writes them.

2. **Flat dotted keys, string values.** Write `brief.source`, not nested `brief: {source: ...}`. The Agent Skills spec describes `metadata` as a map from string keys to **string values**; nesting or a bare YAML timestamp risks failing a strict validator. Quote what brief writes.

3. **A file is brief-written iff it has any `metadata.brief.*` key.** Same marker `brief skill` already uses to decide a skill is brief-managed. Format complaints (rule 4) apply only to brief-written files. On anyone else's file an unparseable date resolves as unset, silently.

4. **Timestamps: write strict, read loose, compare by calendar day.**
   - Brief **writes** ISO 8601 with an explicit UTC offset. Costs nothing now and avoids a retroactive migration if instants are ever needed.
   - Brief **reads** full timestamps and date-only values (`2026-02-23`) alike, with no warning. Date-only is the most common real-world spelling (Obsidian's `date` type).
   - Every comparison is between calendar days. A timestamp's day is the date as written, in its own offset; it is never converted to local time, so a laptop and a UTC CI runner agree.
   - A missing offset or a date-only value in a brief-written file is a warning (a bug in brief). In any other file it is nothing.

5. **Stamp only on content change.** `brief.dateModified` is rewritten only when the generated content, compared with the stamp excluded, differs from what is on disk. Calendar-day comparison does not replace this: without it a re-run on a later day is still a diff, which breaks idempotent install.

6. **Unknown top-level keys are conserved in silence.** No "did you mean" diagnostics, no unrecognized-key warnings. A `status:` or `size:` from someone's vault must survive untouched and unremarked. This is the write-side half of SPEC §2's "unrecognized keys are ignored" (§1.1).

7. **`tags` is untouchable.** Read it if useful; never write, validate, reorder, or reflow it. Reordering it turns brief into diff noise in someone's vault.

8. **Preserving round-trip.** Mutate only owned keys and splice text back. Do not deserialize-and-reemit a file a human owns — that eats foreign keys, comments, and key ordering.
   - Shipped: `serde-saphyr` replaced the archived `serde_yaml` as the single YAML parser for both `Frontmatter` and the provenance pass (one parser, one set of scalar-typing rules). Writes are line splices through `set_brief_metadata` in `src/skill.rs`; `serde-saphyr`'s `Spanned<T>` is available if a write ever needs a span. One behavior change came with the swap: a plain `null` inside a `Vec<String>` is now a parse error rather than the literal string `"null"`.

---

## 6. Validation

`brief validate` reads the frontmatter of the brief and of each local `context:` doc, and checks:

| Check | Severity |
|---|---|
| `isBasedOn` path does not exist | warning |
| `superseded_by` path does not exist | warning |
| `superseded_by` chain is cyclic or longer than 8 | warning |
| A brief's `context:` lists a doc whose `superseded_by` is non-null | warning, names the end of the chain |
| `dateModified` is an earlier calendar day than the file's last git commit | warning |
| `dateModified` of a `context:` doc is a later calendar day than the referencing `.brief.md` | hint, opt-in |
| Top-level and `metadata.brief.*` disagree | warning |
| Brief-written file: timestamp is date-only or lacks an offset | warning |
| URL-valued `isBasedOn` / `superseded_by` | no check |
| Unknown top-level key | **silent** |

Nothing in this vocabulary produces an error. There is no required field and no "block present" state: each key is checked on its own when it resolves.

**Git drift check.** A human sets `dateModified` and then commits, so the commit is always a little later; comparing days absorbs that. Skipped outside a git work tree and for untracked files. A rename or bulk reformat commit will trip it; that is accepted noise, the fix is to bump the date. Applies to human-owned `dateModified` only; a brief-written stamp is kept honest by §5 rule 5 instead.

**Newer-than-brief hint.** Shown only under `brief validate --hints`; never affects the exit code. Says the brief may rest on a doc that has since moved. Off by default because long-lived briefs (a repo's root `.brief.md`) would trip it on every doc edit. The brief's side of the comparison is its own `dateModified` if set, else its last git commit date; with neither, the hint is skipped.

**Dropped: "context doc older than the brief."** It fires on every stable reference doc and carries no information.

`superseded_by` does not change emit output. Emitters stay pure (`&Brief` in, `String` out) and do not open context files; the human acts on the warning by editing `context:`.

---

## 7. Emit behavior

None. Brief emits context, it never inlines it: a `context:` entry reaches the target as a path reference (`- @README.md`) and the runtime opens the file. No emit path reads a context doc's frontmatter, so there is nothing to include, strip, or order by date, and no token cost to cap.

The files brief generates are themselves context docs from the runtime's side. Most have no place for provenance: `AGENTS.md` and the `<brief:generated>` region of `CLAUDE.md` have no frontmatter, and the Cursor / Copilot / Windsurf rule files have frontmatter their vendors read. A generated `SKILL.md` carries `metadata.brief.source` and `metadata.brief.dateModified` (§5 rules 1, 5).

**One source of truth.** If a human-readable provenance header is ever rendered into a document body, it goes into a managed marker region using the existing idempotent-install pattern, never hand-maintained alongside the frontmatter.

---

## 8. JSON-LD compatibility (without adoption)

YAML-LD reached First Public Working Draft on the W3C Recommendation track in March 2026, and the JSON-LD WG charter names "static site front matter" as a target surface. That is brief's exact surface, so the door is worth keeping open — at zero cost.

**Do not add `@context`.** It taxes every document for a capability with no current consumer.

Three decisions preserve future compatibility for free:

1. **String keys only.** YAML-LD constrains YAML so any YAML-LD document is representable in JSON-LD; YAML permits non-string mapping keys and JSON does not. Flat dotted `metadata` keys already satisfy this.
2. **No `@`-leading or `$`-leading keys** in brief's namespace. `@` is a reserved YAML indicator requiring quoting (hence YAML-LD's `$`-convenience context mapping `$id`/`$base`). Both prefixes stay reserved.
3. **Canonical names stay literally equal to their source tokens.** This makes a future `@context` a pure addition rather than a rename. `dateModified` / `dateCreated` / `isBasedOn` already satisfy it; `superseded_by` and `generator` are brief-local and would simply never map. This is the sole reason the three borrowed names are camelCase (§3).

---

## 9. Ecosystem positioning

No standard exists for document provenance in LLM-facing frontmatter. Every established format in the space is instruction frontmatter:

| Format | What its frontmatter does |
|---|---|
| SKILL.md / Agent Skills | skill discovery, routing, tool permissions (`name`, `description`, `allowed-tools`, `model`) |
| AGENTS.md | nothing — plain markdown, no required fields, no frontmatter |
| AGENTS.md frontmatter proposal | conditional context injection (`globs`, `alwaysApply`) |
| `.agent.md` | orchestration wiring (`handoffs`, `agents`, `user-invokable`) |
| OKF | knowledge-graph typing — one required field, `type` |
| Obsidian | vault-local vocabulary; only `tags` / `aliases` / `cssclasses` reserved |
| Static-site generators | rendering and routing (`layout`, `permalink`, `draft`) |

None describe the document's own history. This vocabulary claims no conformance with any of them.

Three-pool convergence, stated plainly:

- **schema.org / RDFS** → canonical field names and camelCase spelling
- **Obsidian + static-site + ingest pipelines** → accepted aliases, all bare lowercase or snake_case
- **OKF** → timestamp discipline, and the discipline of not specifying a content model

### Graph consumers

Turning `superseded_by` and `isBasedOn` into edges of a document graph (graphify, a knowledge graph, a traversal index) is out of scope for brief. It belongs in a corpus-wide consumer such as the ACE db. Brief reads only a brief and its `context:` docs, so it cannot own node identity across a corpus, and emitters stay pure (§6, §7).

This document is the contract such a consumer reads: the field names, the closed alias list (§4), and the path rule (§3: relative to the carrying document's directory; a `scheme://` value is a URL). A consumer should also derive staleness the same way brief does. The `superseded_by` chain is directed from old to new, its end is the current document, and there is no authored `status`. `brief validate` remains the integrity check on the source side.

---

## 10. Open items

- `dateCreated` and `generator` have no consumer. They stay as advisory because they cost nothing to conserve; candidates for removal from the field table in v0.2.
- A relative-link check over document bodies in `brief validate` (the replacement for `archive`). Not part of this vocabulary; tracked separately.
- A derived JSON Schema for the provenance keys, the way `Frontmatter` has one. Possible now that aliases are a closed list; wait for a consumer.
