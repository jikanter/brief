# Potential Ideas for brief 

**Created: 2026-04-27**
**Last Updated: 2026-04-27**

## Note 
This was synthesized in a cleanup of the archive documentation.

## Large necessary improvements

- Time constraint boundaries - Investigate the idea of some time constraints being added to emitted brief content.
Example would be the ability to create a block (much like '<brief:generated>'), that time constraints a specific type of context.
- Some type of artifact-definition subsystem. This could allow for specific, context-dependent generation
- Could we create some sort of Problem/Solution/Diagnosis/TeamContext/Verification/Anti-Patterns/Phases sections for complex tasks?
- Verify steps could potentially be skipped - but output verification could be useful. 
- Template-based emitters for each backend 
- Make intermediate parse tree for emitters so that we do not need to touch **every** emitter every time the frontend format changes 
- Create a base emitter trait or template system (could use the intermediate parse tree for it)
- Org-wide policy section, transparent enforcement within brief (Could be all types of rules)

### Format and Rich Generator
- Preserve markdown structure in briefs. 
- MDX native generation 

### Error handling and Validation
- **No constraint syntax validation** — empty, duplicate, trivial, or contradictory constraints not detected
- **No assumption consistency checks** — assumptions that contradict sacred regions not caught
- **No goal validation** — extremely vague goals or goal-deliverable contradictions not flagged
- **No cross-field validation** — stack lists Python but deliverable says "npm package" not caught
- **No model-specific validation** — invalid or outdated model identifiers not checked
- **No constraint redundancy detection** — same text in both Hard and Soft sections not warned