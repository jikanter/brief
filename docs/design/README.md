# docs/design

| Doc | Status |
|---|---|
| [frontmatter-additions.md](frontmatter-additions.md) | Live. YAGNI bar for `Frontmatter` task fields. |
| [provenance-schema.md](provenance-schema.md) | Draft v0.1, not implemented. Document-provenance keys; §1.1 relates it to the `Frontmatter` schema. |
| [new-surfaces-design-doc.md](new-surfaces-design-doc.md) | Draft v2. Eridian + Hermes emit targets. |
| [new-surfaces-epics.md](new-surfaces-epics.md) | Draft v2. Delivery plan for the above. |
| [backends/](backends/README.md) | Live. Per-backend emit target facts. |

The shipped format lives outside this folder: [../brief-format.md](../brief-format.md), [../schema/SPEC.md](../schema/SPEC.md).

## archive/

Kept for history. Nothing here is current.

- `shipped-briefs/version-schema.brief.md` — shipped in 6cc4578 (`brief_version` + derived frontmatter schema).
- `shipped-briefs/emit-xml.brief.md` — shipped in 3194559 (`brief emit xml`).
- `spike-intent.md` — intent-compiler spike diagram; superseded by the emit model in new-surfaces-design-doc.md.
- `brainstorming/` — early ideation.

The `context:` paths inside the shipped briefs are relative to the repo root, where the briefs were run.
