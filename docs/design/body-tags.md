# Body Tags: a declarative `<brief:*>` overlay

**Status:** Draft v0.1, not implemented. The one thing that is implemented is the
floor: `tests/body_tags_tests.rs` pins today's output for every tag-free fixture
and every stdout target, so the first rule below is enforced before any parser
work starts.

**Decision needed:** this touches the body grammar, which is an Ask First item in
`.brief.md`. Nothing in this doc is built until that is given.

---

## 1. What this is

A small, fixed vocabulary of XML-ish tags in the `brief:` namespace that an
author may put in the Markdown body. Each tag has two meanings, read by the
same compiler pass:

1. **A compiler instruction.** It changes what `brief emit` does with the text
   it wraps: which target gets it, whether it goes in the anchor.
2. **A rendering.** Each target renders the tag in its own register: a Claude
   Code section gets an XML element in the shape Anthropic documents, a README
   gets an admonition, a system prompt gets a caps label.

One tag, two readings. That is the overlay.

### 1.1 What it is not

It is **not MDX.** MDX is Markdown plus JSX plus JavaScript expressions that
execute. brief never executes (charter, Hard). So there are no expressions, no
imports, no components an author defines, no runtime. The nearer prior art is
Markdoc: declarative tags, literal attributes, a schema the compiler validates
against, no code. We use XML-ish syntax rather than Markdoc's `{% %}` because
the Claude emitter already speaks that dialect (`<rules priority="required">`)
and because pulldown-cmark already passes raw HTML through to item text
(`src/parse/body.rs`, the `Event::Html | Event::InlineHtml` arm), so the
syntax enters the grammar without a parser rewrite.

It is **not a new source format.** `.brief.md` stays Markdown plus YAML. Tags
are an optional refinement inside list items and block content. The headings
remain the grammar.

---

## 2. The rules that keep it cheap

The charter's first Hard constraint is that a brief takes under sixty seconds
to author, and a Soft one is that an author should not need the format doc
open. Tags are the first feature with real potential to break both. These
rules exist to stop that, and each one is testable.

### R1. Zero tags is today's brief, byte for byte

A body with no `<brief:*>` tag emits, for every target, exactly the bytes it
emits now. Pinned by `tests/body_tags_tests.rs` across every fixture and every
stdout target. An author who never learns a tag loses nothing and sees nothing.

### R2. Infer before you ask

Where a plain-Markdown idiom already carries the signal, the compiler reads the
idiom and no tag exists. The vocabulary is the residue that cannot be inferred.

| Need | Inferred from plain Markdown | A tag only when |
|---|---|---|
| Rules for the `anchor` target | Hard constraints whose `framing::classify` polarity is `Prohibition`, then the sacred summary, up to the line cap. Today it is the first N hard rules in authoring order (`src/emit/anchor.rs`), which drops scoped and later rules silently. | the inferred pick is wrong for one rule |
| An example | a fenced code block under `## Deliverable` | the example sits elsewhere |
| A callout | a blockquote opening `> **Note:**` or `> **Warning:**` (the GitHub admonition idiom) | never: the blockquote is the component |
| Text for one runtime only | nothing | genuinely per-target wording |

So `example` and `note` are renderings of Markdown the author already writes.
Only `anchor` and `only` need syntax.

### R3. Short forms, and a self-closing line marker

The common shape is a marker at the start of a list item, with no closing tag
to balance:

```markdown
### Hard
- <brief:anchor/> No network crates: `tokio`, `reqwest`, `hyper`, `ureq`.
- <brief:only target="claude"/> Use `.claude/rules` for path-scoped work.
```

It reads like `- [ ]` and costs one token to type. The wrapping form
(`<brief:anchor>…</brief:anchor>`) is accepted for a span inside an item but
is never required. Every attribute has a default: `<brief:anchor/>` alone
means "include me", and `priority` exists only for ordering.

### R4. Forgiving parse, strict validate, never-failing emit

- Tag names are case-insensitive. An unclosed wrapping tag closes at the end
  of its item. An unknown attribute is a warning.
- `brief validate` owns the schema. An unknown name in the `brief:` namespace
  is an error with a suggestion (`<brief:anchr>: unknown tag, did you mean
  <brief:anchor>?`). Any other namespace, or none, is not brief's and passes
  through as text, which is the existing unknown-section extensibility rule
  applied at the inline level.
- `brief emit` never fails on a tag. A tag it cannot interpret degrades to its
  text content with a warning on stderr. A typo cannot block an install.
- Attribute values are string literals. Nothing is evaluated.

### R5. Discoverable from the CLI, not the doc

- `brief init` writes one commented example line into the scaffold, deletable.
- `brief tags` prints the vocabulary with a one-line meaning and a one-line
  example for each. That is the whole reference.
- `docs/brief-format.md` gets an "Extensions" section at the end, after the
  grammar, marked optional.

### R6. The vocabulary is capped and each entry earns its place

At most five tags in the `brief:` namespace. A new tag passes the same bar as
a frontmatter field (`frontmatter-additions.md`), with one check added: it must
fix a defect a plain brief cannot express, or carry a reading no Markdown idiom
carries. Growth past five is a sign the format is becoming the DSL the charter
rejected, and is the point to stop.

---

## 3. The vocabulary

| Tag | Compiler meaning | `claude` / `xml` | `agents-md`, `cursor`, `copilot`, `windsurf`, `aider`, `readme` | `prompt` | `anchor` | `json` / `jsonl` |
|---|---|---|---|---|---|---|
| `<brief:anchor/>` | mark the rule for the anchor target, overriding R2's inference | stripped, text kept | stripped | stripped | included, in authored order | `anchor: true` on the constraint |
| `<brief:only target="…"/>` | emit the item only for the listed targets; `not:` prefix inverts | gated | gated | gated | gated | kept, with `only: [...]` |
| `<brief:include src="…"/>` | splice another `.brief.md` fragment at compile time | inline | inline | inline | inline | inline, provenance in a sibling field |

`example` and `note` are not tags (R2). Their rendering table, since the
compiler owns it either way:

| Idiom | `claude` / `xml` | Markdown targets | `prompt` |
|---|---|---|---|
| fenced block under Deliverable | `<example>…</example>` | fenced block, unchanged | `EXAMPLE:` then the block |
| `> **Note:**` / `> **Warning:**` | `<note kind="warning">…</note>` | blockquote, unchanged | `NOTE:` / `WARNING:` |

`include` is listed for completeness and is last in the order below:
composition has its own design questions (cycle detection, relative paths,
what the content hash `id` covers) and the earlier `extends:` proposal was
rejected (`frontmatter-additions.md`, past decisions).

---

## 4. Where it lands in the code

- **IR.** A constraint's text becomes a span list rather than a `String`.
  `src/parse/body.rs` already assembles `ItemSegment` spans (`Text`, `Code`);
  this adds `Tag { name, attrs, children }`. `Constraint` keeps a `text()`
  accessor that flattens spans, so an emitter that knows nothing about tags is
  unchanged (R1 falls out of this).
- **Validation.** The tag schema lives in `src/validate.rs` beside the section
  checks. Unknown `brief:` name is `Severity::Error`; unknown attribute is
  `Warning`.
- **Emit.** Each emitter gets one small match over `Tag` for the tags it
  renders differently, and falls through to the text for the rest. The `xml`
  target keeps escaping every authored `<` except an interpreted `brief:` tag,
  which it rewrites into its own element; that is the one place escaping grows
  a second path, and the real-XML-parser test in `tests/emit_xml_tests.rs`
  covers it.
- **Byte stability.** Everything is declarative. The content hash `id`
  (`docs/schema/SPEC.md` §7) hashes the flattened text today; whether tags
  join the hash is decided with `include`, since that is the first tag that
  changes what the text *is*.
- **`brief_version`.** Stays `"1"`. A version-1 parser reading a tagged brief
  degrades to text (R4), which is exactly the behaviour the charter's
  extensibility constraint asks for. `docs/brief-format.md` states that rule
  in the Extensions section.

---

## 5. Order of work

1. **Floor.** `tests/body_tags_tests.rs` goldens, blessed from today's output.
   Done in this draft.
2. **Inference only.** Anchor picks `Prohibition` rules first; blockquote
   admonitions and Deliverable fences render per target. No new syntax yet,
   and the goldens for tag-free fixtures change only where the anchor pick
   changes, which is reviewed as its own diff.
3. **`<brief:anchor/>`.** Span IR, validation, `brief tags`, the `init`
   example line. Fixes the inventory defect where the anchor drops scoped and
   late rules.
4. **`<brief:only/>`.** Resolves per-target wording without the `emit:`
   frontmatter map that `emit-targets-reference.md` Implication 2 names as the
   alternative.
5. **`<brief:include/>`.** Separate design.

Each step is its own PR with its own golden diff.

---

## 6. Open questions

- Does the goal H1 accept tags? Proposed: no. List items and block content
  only. The goal is a sentence, and the one place a tag would hurt a reader.
- `only` on a whole `##` section, not an item? Proposed: defer until an
  author asks. Items cover the known cases.
- Should `anchor` on a Soft constraint be an error or a promotion? Proposed:
  a warning and no effect. The anchor is for rules an agent must not drift
  from; a preference does not belong there.
