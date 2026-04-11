---
name: frontmatter-yagni
description: Walk the YAGNI bar before adding a new field to .brief.md frontmatter. Six checks; all must pass. Use when proposing, evaluating, or revisiting a frontmatter field addition.
---

You are evaluating a proposal to add a new field to `.brief.md` frontmatter. Brief has rejected several proposed fields (`extends`, `environment`, `permissions.deny`) because they failed checks like the ones below. Your job is to walk the proposer through the bar and produce a verdict.

## The bar (all six must pass)

1. **Task-specific.** Does the value change task-to-task within the same project? If it would be the same across every brief in this repo, it's standing context — belongs in CLAUDE.md, not `.brief.md`.

2. **No existing carrier.** Can the data fit in any existing field — `stack`, `context`, `model`, `## Constraints`, `## Sacred`, `## Assumptions`, or an unknown `## <Heading>` section? Unknown sections preserve raw Markdown and pass through every emitter, including JSON (as `unknown_sections[].content`). The bar for "unknown sections aren't enough" is therefore high.

3. **Concrete consumer exists or is imminent.** Is there a real downstream tool — being built or on near-horizon roadmap — that needs this in structured form? Brief's JSON emit already exposes unknown-section content as strings, so "a JSON consumer might want this" only counts if string-parsing the equivalent unknown section is demonstrably insufficient.

4. **Shape is obvious.** Would three reasonable engineers propose the same shape? Disagreement (flat string map vs. map of objects vs. list of objects, singular vs. plural) means the underlying need isn't yet understood — wait for the consumer in check 3 to pin it down.

5. **Stays inside brief's mission.** Brief is task-specific structured intent (goal, constraints, sacred regions, assumptions). Does this field move brief toward being an infrastructure manifest, package descriptor, or code-style guide? Wedge fields — where one acceptance invites neighbors like `environments`, `services`, `deployment_targets` — need extra justification.

6. **At least one current user flow fails without it.** Can you complete this sentence? "Today, a user is trying to X with brief and cannot because field Y doesn't exist." Concrete pain, not anticipated convenience. If you can't fill it in, defer.

## Output format

Walk through each check with a one-paragraph answer. Mark each:
- ✓ pass
- ✗ fail
- ~ marginal (treat as fail unless an adjacent check is overwhelmingly strong)

Then produce a verdict:

- **All six pass** → recommend adding. Specify exact field name, type, default value, and which existing serde struct it lives on (`Frontmatter` in `src/model.rs`).
- **Any check fails** → recommend defer. State which checks failed and the precise trigger condition that would flip them. Always state a trigger — "defer indefinitely" with no condition is a smell that the proposal should be rejected outright, not deferred.

Always produce a verdict; never punt with "it depends." If checks 3 and 4 are weak, the answer is almost always defer.

## Reference

- Past decisions table and rationale: [docs/design/frontmatter-additions.md](../../../docs/design/frontmatter-additions.md)
- Worked example (`commands:`, 0/6): [docs/open-questions.md](../../../docs/open-questions.md) `[format]` Frontmatter-Only Commands
- Augment-not-replace principle (load-bearing for check 1): [docs/design-decisions.md](../../../docs/design-decisions.md)
