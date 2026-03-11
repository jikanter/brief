# Radical UX Experiments: Pushing the Boundaries of Input

*Agent: Radical UX Explorer*

---

## Grounding: What Exists Today

The current `brief` CLI is a Rust tool that operates on `.brief.md` files -- Markdown with YAML frontmatter. The `Brief` struct in `src/model.rs` captures goal, constraints (hard/soft/ask-first), sacred regions, assumptions, and deliverables. The CLI has five commands: `init` (scaffold), `validate`, `emit` (to claude/prompt/agents-md/json), `check` (sacred path lookup), and `diff`. The interaction model is entirely file-based: you edit a `.brief.md` in a text editor, then run CLI commands against it.

The key tension: the tool is designed to reduce friction between humans and AI agents, but its own input surface -- hand-authoring structured Markdown -- introduces its own friction. The format is the product today, but it should become the *intermediate representation* while the input surface expands radically.

---

## The Ten Ideas

### 1. Gestural and Spatial Input: The Brief Canvas

**The interaction in detail:** You run `brief canvas` and a visual interface appears -- either a TUI via `ratatui` or a browser UI served from `localhost`. Your project's file tree is rendered as a spatial map, like a city layout where directories are neighborhoods. A set of concentric rings sits at the center: innermost is "Hard," middle is "Soft," outer is "Ask First," and a red border ring is "Sacred." You grab `src/auth/` from the file tree and drag it into the red ring. It becomes a sacred entry. A text prompt appears: "Why is this sacred?" You type "Proprietary tenant resolution, legally reviewed" and it attaches as the reason. You grab a text card, type "Maintain v2 API backward compatibility," and drag it into the innermost Hard ring. When you are done arranging, you press `S` to save. The canvas serializes through the existing `Brief` struct into a valid `.brief.md`.

**Why it reduces friction:** The current format requires remembering syntax: backtick-wrapped paths with em-dash separators for sacred entries, H3 headings for constraint types, checkbox syntax for assumptions. Spatial arrangement replaces syntax with physics. "Closer to center = harder constraint" is a metaphor that requires zero documentation. You are manipulating intent directly, not encoding it into text.

**What it feels like:** Like a war table briefing. You are directing, not writing. The layout *is* the plan. It feels active and commanding rather than clerical.

**Implementation sketch:** The canvas renders the project tree using the same `detect_stack` and `detect_sacred_candidates` logic from `src/init.rs`. Drag interactions mutate an in-memory `Brief` struct. Save serializes it to `.brief.md`. Layout metadata (positions, ring assignments) is stored in `.brief.canvas.json` for re-opening. The TUI path uses `ratatui` with mouse support; the browser path uses a lightweight WebSocket server with `axum` serving a single HTML page with vanilla JS drag-and-drop. No React, no build step.

---

### 2. Ambient Context Capture: The Passive Observer

**The interaction in detail:** You install a file-system watcher daemon: `brief watch start`. It runs silently, logging which files you open, how long you spend in them, and what you search for in your editor (via an optional IDE extension that reports to a local socket). It writes a rolling log to `~/.brief/activity.jsonl`. Days later, you run `brief init`. Instead of the generic scaffold with `# <Describe your goal here>`, you get:

```markdown
# Optimize the event pipeline query layer

## Sacred
- `src/auth/**` -- You haven't modified these in 3 months; high test coverage
- `src/compliance/**` -- Opened read-only 4 times this week (reviewing, not editing)

## Assumptions
- [ ] The bottleneck is in `src/pipeline/query.rs` (47 minutes spent today)
- [ ] The `EventBatch` struct needs refactoring (searched for it 12 times)
```

The brief already knows what you are thinking about because it watched you think.

**Why it reduces friction:** The blank page problem is the largest friction point. The current `scaffold_brief` function in `init.rs` can detect stack and sacred candidates from the file system, but it knows nothing about your *current intent*. Ambient context fills the gap between "what exists in the repo" and "what you are actually working on." You do not have to recall what you were investigating; the tool remembers.

**What it feels like:** Like having a really attentive pair programmer who has been sitting next to you all day. They do not interrupt, but when you ask "what should we work on?" they have a detailed, accurate answer.

**Implementation sketch:** `brief watch start` spawns a background process that monitors file access via `fswatch` (macOS) or `inotify` (Linux). It writes JSONL entries: `{"ts": "...", "event": "file_open", "path": "src/pipeline/query.rs", "duration_s": 47}`. The `scaffold_brief` function gains an optional `ActivityLog` parameter. When present, it uses file-open frequency and duration to rank sacred candidates (files opened read-only many times = likely sacred), identify the probable goal area (files edited most recently and most frequently = likely change target), and seed assumptions from repetitive access patterns. Privacy controls are essential: `brief watch --exclude '*.env,*secret*'`, all data strictly local, explicit opt-in, `brief watch clear` to purge history.

---

### 3. Emotion and Urgency Signals: The Tempo Reader

**The interaction in detail:** You run `brief init` at 2:37 AM on a Saturday. Your git log shows 14 commits in the last hour (thrashing). You type the goal as "fix prod auth bug." The tool detects urgency from these signals and generates a *tighter* scaffold than normal:

```markdown
# Fix production auth bug

## Constraints

### Hard
- Fix only -- do NOT introduce new behavior
- All existing tests must pass without modification
- Rollback plan required before deploy

### Sacred
- `src/auth/**` -- Production authentication (ELEVATED: hotfix context)
- `migrations/**` -- Never alter historical files
- `src/billing/**` -- Adjacent critical path, do not touch

### Ask First
- Any change to files outside `src/auth/`
```

In calm conditions -- Tuesday afternoon, methodical commit history, descriptive goal text -- the same `brief init` scaffolds with more room for exploration: softer constraints, fewer sacred regions, more unvalidated assumptions.

**Why it reduces friction:** Under stress, humans make bad decisions about scope. They forget to protect things. They skip constraints because "I'll be careful." The tool's job is to compensate for the human's current cognitive state. The brief gets tighter when you are less likely to be thinking clearly. This is not about capturing emotion -- it is about providing appropriate guardrails.

**What it feels like:** Like a senior engineer who says "let us slow down for 30 seconds before we touch production." Not a nanny. A guardrail that appears when the road gets narrow.

**Implementation sketch:** Add a `ContextSignals` struct computed at init time:

```rust
struct ContextSignals {
    time_of_day: NaiveTime,
    is_weekend: bool,
    recent_commit_velocity: f64,  // commits/hour in last 4 hours
    goal_text_urgency: f64,       // caps ratio, terseness, keywords like "fix", "prod", "urgent"
    branch_name_signals: Vec<String>, // "hotfix", "urgent", "prod-fix"
}
```

The `scaffold_brief` function adjusts template selection based on an urgency score derived from these signals. High urgency: expand sacred regions to include all detected patterns (not just auth/migrations), add hard constraints about test preservation and rollback, shrink the Soft section, auto-add "Ask First" for out-of-scope files. Low urgency: wider scaffold, more unvalidated assumptions, more Soft constraints. All signals are computed from `git log`, the system clock, and basic text analysis. No network calls, no telemetry, no creepiness.

---

### 4. Progressive Disclosure: The Zoom Lens Brief

**The interaction in detail:** You type:

```
brief new "Make search faster"
```

You get a file:

```yaml
---
stack: [Rust]
---
```

```markdown
# Make search faster
```

That is the whole file. It is valid. You can emit it right now. But then you run `brief expand` and the tool asks one question at a time:

```
What must NOT change? (sacred regions, or Enter to skip)
> src/auth, src/billing

Any hard constraints? (non-negotiable rules, or Enter to skip)
> Keep backward compatibility with v2 API

What does "done" look like?
> Search results return in <200ms at p95
```

Each answer appends to the brief. You can stop at any point. The brief is always valid at every level of detail. Going the other direction: `brief compress` takes a fully detailed brief and produces a one-liner summary for quick reference: `"Make search faster [3 hard, 2 sacred, 4 assumptions]"`.

**Why it reduces friction:** The current format demands all-or-nothing. The sample brief at `examples/sample.brief.md` has 36 lines of structured content. That is daunting if you just need to say "add pagination, don't touch auth." Progressive disclosure means the cost of creating a brief scales with the complexity of the task, not the complexity of the format. Simple tasks get simple briefs. Complex tasks earn detail incrementally. This is the progressive JPEG model applied to intent specification.

**What it feels like:** Like a conversation, not a form. Each question is a prompt, not a field. You talk to the tool and it builds structure around your words. The brief grows organically, like zooming in on a map -- useful at every zoom level.

**Implementation sketch:** `brief new <goal>` writes a minimal valid `.brief.md` with auto-detected frontmatter (via existing `detect_stack`) and just the H1 goal. Then `brief expand` reads the current `.brief.md`, identifies which sections are missing or empty, and runs a guided questionnaire using the `inquire` or `dialoguer` crate. Each answer is validated through the existing parser. The key invariant: `brief validate` passes after every interaction step. This requires making the validation in `src/validate.rs` lenient about missing optional sections -- which it mostly already is. The main change is adding `brief new` as a command alongside `brief init`.

---

### 5. Collaborative Brief Building: The Briefing Table

**The interaction in detail:** The tech lead runs `brief share`. A local WebSocket server starts, and a URL appears: `http://192.168.1.42:3333/brief`. The product manager opens it and types the goal: "Add enterprise SSO support." The architect opens it and drags files into the sacred zone (using a lightweight version of the canvas UI). The security engineer adds hard constraints about token handling. Everyone sees additions in real-time, color-coded by contributor. When the meeting ends, someone runs `brief lock` and the collaborative session becomes a committed `.brief.md` file with a `## Contributors` section.

**Why it reduces friction:** Briefs should not be authored by one person. The developer knows the codebase. The PM knows the requirements. The security engineer knows the risks. Right now, creating a brief requires one person to hold all this knowledge or to transcribe meeting notes into `.brief.md` format. The briefing table makes the meeting itself the authoring process. The `.brief.md` structure actually helps here -- "let's fill in the sacred regions" is a better meeting agenda item than "let's discuss what we shouldn't change."

**What it feels like:** Like writing the spec together instead of debating it and then having one person write it up afterward.

**Implementation sketch:** `brief share` starts a local server using `axum` with WebSocket support via `tokio-tungstenite`. The server holds the canonical `Brief` struct in memory behind a mutex. Each client receives the current state as JSON (via the existing `emit_json` emitter). Edits arrive as JSON patches: `{"op": "add_constraint", "type": "hard", "text": "..."}`. The server applies the patch, validates with the existing `validate` function, and broadcasts the updated state. On `brief lock`, the server serializes the `Brief` to `.brief.md` and writes it. For remote access, an optional `--tunnel` flag uses a tool like `bore` to expose the local port. This does introduce `tokio` as a dependency, which the current guidelines prohibit for Phase 1 -- so this is a Phase 2+ feature.

---

### 6. Natural Language to Structured Brief: The Interpreter

**The interaction in detail:** You type or speak:

```
brief from "I need to add pagination to the users API but don't touch
the auth middleware and make sure we keep the existing query parameter
format. We're using Python 3.12 and FastAPI. Should be done by Friday."
```

Out comes a complete `.brief.md`:

```yaml
---
stack: [Python 3.12, FastAPI]
---
```

```markdown
# Add pagination to the users API

## Constraints

### Hard
- Preserve existing query parameter format

### Soft
- Target completion by Friday

## Sacred
- `**/auth/middleware*` -- Do not modify auth middleware

## Assumptions
- [ ] The users API does not already have pagination
- [ ] FastAPI's pagination patterns are compatible with existing query params
```

The tool parsed "don't touch" into a sacred entry, inferred that a deadline is a soft constraint (negotiable), extracted stack from explicit mentions, and generated assumptions that are logically implied but were not stated.

**Why it reduces friction:** This is the ultimate friction reduction because it eliminates the format entirely as an authoring concern. You say what you want however is natural, and the tool handles structure. The `.brief.md` format becomes a machine-readable intermediate representation -- important for agents and for review, but not something humans must write. This inverts the value proposition: the format is not the product. The *translation* is.

**What it feels like:** Like dictating to an extremely competent assistant who knows your codebase. You speak loosely and your words come back structured. Reviewing the generated brief takes 10 seconds. Authoring it from scratch would have taken 2 minutes.

**Implementation sketch:** `brief from <text>` or `brief from --stdin` sends the input to an LLM (configured via `model` field in `~/.brief/config.toml` or the current `.brief.md`). The system prompt includes the full `.brief.md` format specification from `CLAUDE.md` plus the current repo's `detect_stack` output as grounding context. The LLM response is validated by piping it through the existing `parse_brief` function. If parsing fails, the raw output is shown with diagnostics. For voice input: `brief listen` captures audio via the system microphone, transcribes via Whisper (local `whisper.cpp` or a cloud API), and pipes the transcript through `brief from`. This requires network calls, so it is a Phase 2 feature.

---

### 7. Brief by Example: The Pattern Learner

**The interaction in detail:**

```
brief like pr#247
```

The tool fetches PR #247's diff, analyzes which files changed and which did not, looks at the PR description, and cross-references with any `.brief.md` that existed at that commit. It generates:

```markdown
# <Goal similar to PR#247: API endpoint refactoring>

## Constraints

### Hard
- Maintain backward compatibility (PR#247 kept all existing endpoints)
- Test coverage >= 85% (PR#247 achieved 87%)

## Sacred
- `src/auth/**` -- Not modified in PR#247 despite touching adjacent code
- `src/billing/**` -- Not modified in PR#247

## Assumptions
- [ ] The same API versioning strategy applies
- [x] Database schema is unchanged (PR#247 was schema-stable)
```

Files that were *adjacent to the change but untouched* are strong sacred candidates. Patterns from the PR's commit messages and description are extracted as constraints.

**Why it reduces friction:** Experienced teams have implicit patterns. "When we touch the API layer, we never modify auth." These patterns live in tribal knowledge and code review comments, not in any structured artifact. Brief by Example extracts patterns from git history and makes them explicit. It is organizational memory made actionable.

**What it feels like:** Like having a team wiki that actually works -- one derived from behavior rather than aspirational documentation. You say "do it like last time" and the tool knows what "last time" meant structurally.

**Implementation sketch:** `brief like <ref>` accepts a PR number, commit SHA, or branch name. It uses `git log` and `git diff` to analyze the change. Files modified become scope context. Files *adjacent but untouched* become sacred candidates (using directory proximity as a heuristic). Commit message patterns are parsed for constraint keywords: "must," "never," "always," "don't" signal hard constraints; "prefer," "try to," "ideally" signal soft constraints. For PR integration, `gh pr view <number> --json files,body,title` provides the PR description. If a `.brief.md` existed at that commit (`git show <sha>:.brief.md`), it is used as the primary template. The analysis logic goes in a new `src/learn.rs` module. This requires only `git` and `gh` CLI -- no LLM needed.

---

### 8. Anti-Briefs: The Negative Space

**The interaction in detail:**

```
brief not "Don't add new dependencies. Don't change the database schema.
Don't modify anything in /core. Don't break the existing API."
```

Generates:

```markdown
# <Goal not yet specified>

## Constraints

### Hard
- No new dependencies added to Cargo.toml / package.json / requirements.txt
- Database schema must remain unchanged (no new migrations)
- Existing API contracts must be preserved

## Sacred
- `src/core/**` -- Explicitly excluded from modification scope
```

There is no goal yet. The anti-brief defines boundary conditions without specifying an objective. This is a legitimate and useful state. Many senior engineers think exactly this way: "I don't care what you build, just don't break these things."

**Why it reduces friction:** Sometimes you know precisely what you are afraid of but only vaguely what you want. The current format requires a goal -- the H1 heading is mandatory, and validation errors without it. But the cognitive process is often fear first, desire second. "Don't break prod" is clearer in your mind than "implement feature X" when you are handing off work to an agent. Anti-briefs honor this cognitive pattern.

**What it feels like:** Like telling a house sitter "don't let the dog out, don't use the good china, don't touch the thermostat." That is sufficient instruction. The positive goals can come later.

**Implementation sketch:** `brief not <text>` parses negation-oriented language. At the simplest level (no LLM required), keyword matching handles common patterns: "don't change" / "don't modify" / "don't touch" maps to Sacred entries. "Don't add" maps to Hard constraints about additions. "Don't break" maps to Hard constraints about preservation. The generated `.brief.md` has a placeholder H1 goal. The model change: the `goal` field in `Brief` (currently `String` in `src/model.rs`) becomes `Option<String>`. Validation emits a `Warning` (not `Error`) when the goal is missing but constraints or sacred regions are present. This reflects the legitimate use case where boundaries are known before objectives.

---

### 9. The Blank Page Killer: Predictive Briefs

**The interaction in detail:** You start your day and run `brief suggest`:

```
Based on your recent activity, here are suggested briefs:

1. [HIGH] Continue: Optimize event pipeline query layer
   Working on src/pipeline/query.rs yesterday. 3 TODOs remain.
   Last commit: "WIP: batch query optimization"
   > brief init --continue

2. [MEDIUM] From issue: Add rate limiting to public API (#142)
   Assigned to you 2 days ago. No branch exists yet.
   > brief init --issue 142

3. [LOW] Tech debt: 4 files have TODO(your-name) comments
   src/api/routes.rs:47, src/pipeline/batch.rs:112, ...
   > brief init --todos
```

You pick option 2. The tool fetches the issue, parses its description for implicit constraints, checks its labels ("security" -> auth is sacred), and scaffolds a populated `.brief.md`.

**Why it reduces friction:** The blank page problem is not just about formatting. It is about deciding what to work on and then setting up context for that work. `brief suggest` does the deciding for you, based on signals that already exist: git state, issue trackers, TODO comments, recent file activity. Setup time drops from minutes to seconds.

**What it feels like:** Like your project manager distilled overnight and left you a perfect handoff note. You walk in, read three options, pick one, and you are working.

**Implementation sketch:** `brief suggest` aggregates signals from multiple local sources: (1) `git log` and `git branch` for recent WIP branches and uncommitted work, (2) `git stash list` for stashed context, (3) `gh issue list --assignee @me --state open` for assigned issues, (4) grep for `TODO` and `FIXME` comments containing the current git user's name or email. Each source produces a `Suggestion` struct with a priority score, description, and the `brief init` invocation command. Ranking heuristic: in-progress work (WIP branches with recent commits) ranks highest, followed by recently assigned issues, followed by old TODOs. For issue-based briefs, the issue body is parsed for constraint keywords. All of this uses only `git` and `gh` CLI -- no LLM, no external services.

---

### 10. Physical World Bridges: Brief from the Analog

**The interaction in detail:**

**Whiteboard photo.** Your team sketches architecture on a whiteboard. Boxes for services, arrows for data flow, red X marks on things not to touch. You photograph it: `brief from-image whiteboard.jpg`. The tool uses a vision model to identify: boxes as components (stack entries and scope), arrows as data flow (context files), red marks as danger zones (sacred regions), and handwritten text (goals, constraints). Out comes a scaffolded `.brief.md` with structure inferred from spatial arrangement and visual cues.

**Walking conversation.** You are on a walk discussing the next sprint. You tap: "Hey brief, start listening." You talk for 8 minutes. Back at your desk: `brief transcribe ~/recordings/latest.m4a`. The tool transcribes, identifies speakers if possible, extracts goals from declarative statements ("we need to..."), constraints from conditional statements ("but we can't..."), and sacred regions from protective statements ("whatever we do, don't touch...").

**Sticky notes.** Physical planning with Post-Its. Pink = constraints. Green = goals. Yellow = assumptions. You arrange them on a table, take a photo, and `brief from-image` interprets the color-coding.

**Why it reduces friction:** The best thinking happens away from editors. Conversations, whiteboards, walks -- these are where strategy forms. But there is a lossy translation step: insight -> memory -> typing -> structured format. Each step loses fidelity. Physical world bridges eliminate intermediate steps. The conversation *is* the brief. The whiteboard *is* the brief.

**What it feels like:** Like your tools finally understand that you are a person who exists in physical space, not just a keyboard operator. The capture moment becomes invisible.

**Implementation sketch:** These are all *input adapters* that produce text, which then flows through the `brief from` natural language pathway (idea #6). `brief from-image <path>` sends the image to a vision model with a prompt: "This is a planning artifact. Identify components, relationships, danger zones, and readable text. Map to: stack, sacred, constraints, goal." `brief transcribe <audio>` uses Whisper (via `whisper.cpp` for local, or a cloud API) to produce text, then pipes through `brief from`. The core `Brief` struct and parser do not change. Only the input surface expands. This is Phase 3 -- dependent on both LLM integration (Phase 2) and vision/audio capabilities.

---

## Synthesis: The Input Spectrum

These ideas form a spectrum from low-infrastructure to high-infrastructure:

| Idea | Requires LLM | Requires Network | Complexity | Phase |
|------|-------------|-------------------|------------|-------|
| Progressive Disclosure | No | No | Low | 1.5 |
| Anti-Briefs | No | No | Low | 1.5 |
| Tempo Reader | No | No | Medium | 2 |
| Predictive Briefs | No | gh CLI only | Medium | 2 |
| Ambient Context | No | No | Medium | 2 |
| Brief by Example | No | gh CLI only | Medium-High | 2 |
| Natural Language | Yes | Yes | Medium | 2 |
| Spatial Input | No | No (TUI) / localhost (web) | High | 2-3 |
| Collaborative | No | Local network / tunnel | High | 3 |
| Physical World | Yes | Yes | Very High | 3 |

**Recommended sequencing for maximum impact:**

1. **Progressive Disclosure** and **Anti-Briefs** (Phase 1.5). These require minimal code changes -- a new `brief new` command, making `goal` optional in the model, and an interactive `brief expand` flow. They immediately solve the blank page problem and the "I know my fears not my desires" problem. Highest ROI.

2. **Predictive Briefs** and **Tempo Reader** (Phase 2, early). These use only local signals (git, system clock, file system) and transform the tool from passive to active. `brief suggest` is the command that makes people realize this tool thinks about their workflow, not just their files.

3. **Natural Language** (Phase 2, mid). This is the keystone. Once `brief from <text>` works, every subsequent input modality is just a front-end that produces text for `brief from` to consume. Ambient context feeds it. Voice feeds it. Whiteboard OCR feeds it. The architecture becomes: `[any input] -> text -> brief from -> Brief struct -> .brief.md`.

4. **Brief by Example** and **Ambient Context** (Phase 2, late). These build organizational memory. They make the tool more valuable the longer you use it.

5. **Spatial Input**, **Collaborative**, and **Physical World** (Phase 3). These are the ambitious bets that require significant new infrastructure but redefine what "authoring a brief" means.

**The deepest insight:** The `.brief.md` format should be an *output* format, not an *input* format. Humans should never need to write YAML frontmatter or remember em-dash separators. They should speak, gesture, point, exclude, and converse. The tool produces valid `.brief.md` from whatever signal it receives. The format is for machines and for review. The interaction is for humans. The model.rs `Brief` struct is already the right intermediate representation -- every idea above ultimately produces a `Brief` that gets serialized. The innovation is entirely on the input side.
