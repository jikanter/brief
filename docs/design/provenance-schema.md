# Brief Provenance Vocabulary v0.1

**Status:** Draft for implementation
**Scope:** `brief` reads, validates, and conserves document provenance in YAML frontmatter. `brief` does not author provenance for documents it did not generate.

---

## 1. Purpose

Machine-checkability, not LLM comprehension.

An LLM parses `**Last updated:** 2026-04-11` in a prose header perfectly well. What prose cannot do is:

- be asserted on in CI
- be diffed meaningfully
- have its referenced paths checked for existence
- order a set of documents by staleness without a model pass

This vocabulary exists so `brief validate` can make those checks and `brief emit` can make trim decisions. A field that no `brief` code path consumes is decoration; it is marked **advisory** and is the first thing dropped under budget pressure.

### Non-goals

- Authoring provenance. Brief is a human→agent format; emitting provenance onto AI-generated documents is the inverse problem and out of scope.
- Specifying a content model. Following OKF's restraint, exactly one field has teeth; everything else is optional.
- Being JSON-LD. See §8 for compatibility-without-adoption.
- Replacing or competing with SKILL.md / AGENTS.md / `.agent.md` frontmatter. Those are *instruction* frontmatter (routing, tool permissions, orchestration). This is *provenance* frontmatter. They coexist in the same block.

---

## 2. The block

```yaml
---
# ═══ required if the provenance block is present at all ═══
dateModified: 2026-04-11T14:22:00-05:00

# ═══ optional, machine-validated ═══
isBasedOn: ./docs/architecture.brief.md
supersededBy: null
archive: ./archive/obsolete-features.md

# ═══ optional, advisory only ═══
dateCreated: 2026-02-23
generator: multi-agent-synthesis
revisions:
  - date: 2026-03-30T16:40:00-05:00
    note: downgraded Tier 0 proposals
    by: multi-agent-synthesis

# ═══ read-only, never written, never validated, never reordered ═══
tags: [architecture, pipeline]

# ═══ conserved passthrough — brief writes only under metadata.brief.* ═══
metadata:
  brief.source: ./docs/architecture.brief.md
---
```

---

## 3. Field reference

| Field | Type | Consumer in brief | Status |
|---|---|---|---|
| `dateModified` | ISO 8601 datetime | `validate` staleness vs. brief; `emit --budget` trim ordering | **Required** if block present |
| `isBasedOn` | path | `validate` path existence | Optional, validated |
| `supersededBy` | path \| null | `validate` dangling check; non-null ⇒ excluded from `emit` | Optional, validated |
| `archive` | path | `validate` path existence | Optional, validated |
| `dateCreated` | ISO 8601 datetime \| date | none | Advisory |
| `generator` | string | none | Advisory |
| `revisions` | list of `{date, note, by}` | none | Advisory; first dropped under `--compact` |
| `tags` | list | none | Read-only passthrough |
| `metadata` | map, string keys → string values | fall-through resolution only | Conserved passthrough |

### Deliberately excluded

**`status`.** Duplicates information already carried by `supersededBy`. Two sources of truth for one fact is the exact failure mode this vocabulary replaces in the prose header — reintroducing it inside the frontmatter is no improvement. Derive it.

Vault- and wiki-style `status:` keys appear in the wild (Obsidian, Confluence exports). They are conserved silently. See §5, rule 4.

### Naming rule

> **Borrowed names keep their source spelling. Brief-native names are bare lowercase.**

`dateModified`, `dateCreated`, and `isBasedOn` are literal schema.org / RDFS tokens. The entire value of choosing them over `modified` / `created` is exact-string pretraining priors; renaming to `date_modified` forfeits the prior and gains nothing. camelCase at the linked-data vocabulary layer is consistent across schema.org, RDFS, and JSON-LD's own vocabulary.

`supersededBy` is written in schema.org register but is effectively brief-local (schema.org defines it on Enumeration/Class/Property, not CreativeWork). `archive`, `generator`, `revisions` are brief-native. No standards-conformance claim is made for any of these.

YAML itself specifies nothing about key naming, and practice is genuinely split (Kubernetes camelCase, CircleCI snake_case, Jenkins kebab). There is no YAML-level convention to defer to, which is why the tiebreaker is quotation rather than house style.

Every **alias** (§4) is bare lowercase or snake_case. Nobody hand-writes `dateModified`; camelCase appears only on canonical forms, and canonical forms are all borrowed.

---

## 4. Aliases and normalization

### Registered aliases

```
dateModified ← modified, updated, updated_at
dateCreated  ← created, created_at
```

`updated_at` / `created_at` are registered because the convention is live in ingest pipelines that produce documents brief will read (Confluence→markdown exporters commonly emit `created_at` / `updated_at` / `synced_at` / `version`). Two entries, not a synonym table.

### Separator-insensitive matching

Canonical names additionally match after lowercasing and stripping `_` and `-`. So `date_modified`, `date-modified`, `datemodified`, and `dateModified` all resolve.

Two hard constraints:

1. Applies **only** to brief's canonical names and registered aliases.
2. Never applied to foreign top-level keys, and **never inside `metadata`**. Foreign keys pass through byte-identical or conservation is broken.

### Resolution order

```
dateModified → modified → updated → updated_at → metadata.brief.dateModified → unset
dateCreated  → created  → created_at                                          → unset
```

Canonical form wins on conflict. Top-level always beats `metadata.brief.*`. **Never merge the two addresses.**

Disagreement between two resolvable addresses is a `validate` **warning**, not an error.

**Unset is never an error.** Provenance is advisory at the document level; requiring it would make `brief init` hostile on every pre-existing repo doc, vault, or docs tree.

---

## 5. Rules

1. **Brief writes top-level only.** `metadata` is read-only passthrough in every code path. One enforcement site.

2. **Flat dotted keys under `metadata`.** Write `brief.source`, not nested `brief: {source: ...}`. The published Agent Skills spec describes `metadata` as a map from string keys to **string values**; nesting risks failing a strict validator on skill artifacts brief emits. Dotted keys give namespacing without depth. (Existing precedent: `metadata.brief.source`.)

3. **Timestamps: write strict, read loose.**
   - Brief **writes** ISO 8601 with an explicit UTC offset, always. OKF migrated every timestamp to explicit-offset ISO 8601 retroactively; that cost is avoidable by getting it right on write.
   - Brief **reads** date-only values (`2026-02-23`), normalizes to `T00:00:00` local, and emits a `validate` warning. Rejecting date-only would reject the single most common real-world spelling (Obsidian's `date` type is date-only by design) and break `brief init` on existing vaults.
   - Bare `2026-04-11` in a file brief wrote is a bug. In a file brief read, it is a warning.

4. **Unknown top-level keys are conserved in silence.** No "did you mean" diagnostics, no unrecognized-key warnings. A `status:` or `size:` from someone's vault must survive untouched and unremarked.

5. **`tags` is untouchable.** Read it if useful; never write, validate, reorder, or reflow it. It is the most broadly conserved key in the entire frontmatter ecosystem (Obsidian, every static-site generator, skill registries). Reordering it turns brief into diff noise in someone's vault.

6. **512-byte cap** on the provenance block, enforced by `validate`. Justified directly by downstream token budgets (see §7).

7. **Preserving round-trip.** Parse to an order-preserving representation; mutate only owned keys; splice the frontmatter **text span** back. Do not deserialize-and-reemit — that silently eats foreign keys, comments, and author key ordering, which is the precise opposite of conserved.
   - Implementation note: `serde_yaml` is archived and comment-lossy. The `saphyr` / `yaml-rust2` line provides the fidelity needed for span-level splicing.

---

## 6. Validation

`brief validate` checks, in order:

| Check | Severity |
|---|---|
| Block present but `dateModified` unresolvable | error |
| `isBasedOn` / `archive` path does not exist | warning |
| `supersededBy` non-null and path does not exist | warning |
| `dateModified` older than the `.brief.md` that references this doc as `context:` | warning |
| Timestamp lacks explicit offset | warning |
| Top-level and `metadata.brief.*` disagree | warning |
| Provenance block exceeds 512 bytes | error |
| Unknown top-level key | **silent** |

Nothing in this vocabulary produces an error on a document brief did not write, except the byte cap and an unresolvable `dateModified` in an otherwise-present block.

---

## 7. Emit behavior

| Path | Provenance |
|---|---|
| `brief emit prompt` | included |
| `brief emit claude --full` | included |
| `brief emit claude --install` | included |
| `brief emit --compact` | **stripped** |
| `brief emit anchor` | **stripped** |
| dispatched-skill emission | **stripped** |

Rationale for stripping: in compact and dispatch paths, provenance is a per-document tax against a hard token ceiling and buys nothing — no code path and no model behavior depends on it there. Downstream skill dispatchers operate under fixed token budgets (e.g. a 2048-token share); six provenance fields run roughly 40–60 tokens, which across a dispatched set is 3–8% of the budget spent on metadata nothing acts on.

**Corollary — one source of truth.** If a human-readable provenance header is rendered into the document body, it must be emitted into a managed marker region using the existing idempotent-install pattern, never hand-maintained alongside the frontmatter. If brief is unwilling to own the rendered block, ship frontmatter-only and let the prose header die. Two hand-maintained copies of the same three dates will diverge within a month.

---

## 8. JSON-LD compatibility (without adoption)

YAML-LD reached First Public Working Draft on the W3C Recommendation track in March 2026, and the JSON-LD WG charter names "static site front matter" as a target surface. That is brief's exact surface, so the door is worth keeping open — at zero cost.

**Do not add `@context`.** It taxes every document for a capability with no current consumer and collides with the 512-byte cap.

Three decisions preserve future compatibility for free:

1. **String keys only.** YAML-LD constrains YAML so any YAML-LD document is representable in JSON-LD; YAML permits non-string mapping keys and JSON does not. Flat dotted `metadata` keys already satisfy this.
2. **No `@`-leading or `$`-leading keys** in brief's namespace. `@` is a reserved YAML indicator requiring quoting (hence YAML-LD's `$`-convenience context mapping `$id`/`$base`). Both prefixes stay reserved.
3. **Canonical names stay literally equal to their source tokens.** This makes a future `@context` a pure addition rather than a rename. `dateModified` / `dateCreated` / `isBasedOn` already satisfy it; `supersededBy`, `archive`, `generator`, `revisions` are brief-local and would simply never map.

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

---

## 10. Open items

- Whether `revisions[]` earns its place at all. It has no consumer, it is the largest field by bytes, and it is the first thing `--compact` drops. Candidate for removal in v0.2 if no `validate` or `emit` use materializes.
- Whether `supersededBy: null` should be written explicitly or omitted. Explicit null is more legible to a human; omission is cheaper against the byte cap. Currently: omit on write, accept on read.
- Behavior when a document resolves `dateModified` from `metadata.brief.*` only. Currently brief does not promote it to top-level, because promotion is a write to a document brief did not author. Revisit if it proves annoying in practice.
