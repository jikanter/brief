# Frontmatter Additions: The YAGNI Bar

A six-check bar for adding fields to `.brief.md` frontmatter. Add a field only when **all six** checks pass. Any "no" defers the decision until that check flips.

For in-conversation use, invoke the `frontmatter-yagni` skill ([.claude/skills/frontmatter-yagni/SKILL.md](../../.claude/skills/frontmatter-yagni/SKILL.md)) — it walks proposers through the checks. This doc is the reference and rationale.

## The bar

1. **Task-specific.** The value changes task-to-task within the same project. If it would be the same across every brief in this repo, it's standing context — belongs in CLAUDE.md, not `.brief.md`. (See [../design-decisions.md](../design-decisions.md) "Augment, Not Replace.")

2. **No existing carrier.** None of `stack`, `context`, `model`, the constraint sections, sacred regions, or unknown sections can hold the data naturally. Unknown sections preserve raw Markdown and pass through every emitter — JSON includes them as `unknown_sections[].content`. The bar for "unknown sections aren't enough" is therefore high.

3. **Concrete consumer exists or is imminent.** A real downstream tool needs the data in structured form. "Someday a tool will want this" doesn't count. Because JSON emit already exposes unknown-section content as strings, this check requires showing that string-parsing the equivalent unknown section is genuinely insufficient for the consumer.

4. **Shape is obvious.** Three reasonable engineers would propose the same shape. Disagreement (flat string map vs. map of objects vs. list of objects, singular vs. plural, etc.) means the underlying need isn't yet understood — wait for a consumer to pin it down.

5. **Stays inside brief's mission.** Brief is task-specific structured intent. Fields that drift toward infrastructure manifest, package descriptor, or code-style guide fail this check. Wedge fields — where one acceptance invites neighbors like `environments`, `services`, `deployment_targets` — need extra justification.

6. **At least one current user flow fails without it.** Concrete pain, not anticipated convenience. Test: complete the sentence "Today, a user is trying to X and cannot because field Y doesn't exist." If you can't, defer.

## Past decisions

| Field | Status | Notes |
|---|---|---|
| `stack`, `context`, `model` | ✅ added | All six checks pass |
| `skill_name`, `skill_description` | ✅ added | Driven by skill emit target |
| `extends:` (composition) | ❌ rejected | Fails 2 (two-file composition exists), 4 (ambiguous semantics), 5 (wrong direction) |
| `environment:` | ❌ rejected | Fails 1 (standing context), 5 (infrastructure drift) |
| `commands:` | Deferred | 0/6 pass — see [../open-questions.md](../open-questions.md) `[format]` Frontmatter-Only Commands for the walkthrough |
| `cassettes:` / `## Fixtures` | Deferred | 2/6 fail firmly, 3 soft — eval-replay binding for the astrophage tool; see [../open-questions.md](../open-questions.md) `[format]` Cassette / `## Fixtures` Field for the walkthrough. Companion to aichat `SPEC-astrophage.md`. |

The pattern holds: every accepted field passes all six; every rejection fails at least three. The bar isn't post-hoc rationalization — it's the implicit reasoning from [../analysis/archive/obsolete-features.md](../analysis/archive/obsolete-features.md) and [../analysis/phase2-synthesis.md](../analysis/phase2-synthesis.md) "What NOT To Do" organized into a checklist.

## Using the bar

A proposal is a six-check walkthrough, not a feature description. One paragraph per check, then a verdict. The walkthrough format forces the right questions in the right order; proposals that skip it get pushed back.
