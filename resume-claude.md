# Resume — Showboat Evergreen Demo Sweep

## Task
Sweep `docs/demos/*.md` showboat demos so every one re-verifies green against current CLI (`showboat verify <file>` exit 0), and documents current behavior. "Evergreen" = re-verifiable + accurate now.

## Tool facts
- `showboat` 0.6.1 at `/Users/admin/.local/bin/showboat`. Subcommands: init, note, exec, image, pop, verify, extract.
- `showboat verify <file>` → exit 0 if all code-block outputs match re-run; exit 1 on drift. `--output <new>` writes refreshed copy WITHOUT changing original.
- `showboat exec <file> <lang> <code>` appends command+captured output (captures stdout AND stderr). `--workdir` sets cwd but is NOT recorded in doc → for portable verify each exec must `cd /abs/path` itself.
- `pop` removes last entry (from END only).

## Root causes of staleness found
1. **CLI arg order changed**: `brief emit TARGET FILE` → `brief --file FILE emit TARGET` (global `--file`). Same for `validate`. `check PATH --file FILE` and `diff A B` unchanged.
2. **`brief emit skill` removed** → now `brief --file F skill emit`.
3. **`brief skill scaffold --from-doc` removed** → `--from-brief` / `--description` / `--name`.
4. **P0 framing** changed claude/prompt/agents/skill emit output: `**IMPORTANT:**` → `<rules priority="required">` + `MUST:`/`PREFER:`/`STOP and confirm`; sacred → `<protected_files>`/`<sacred_regions>`; context → `<context>`; Reference Context section now AFTER protected files.
5. **Install markers changed**: `<!-- brief:start -->`/`<!-- brief:end -->` → `<brief:generated>`/`</brief:generated>`.
6. Installed PATH `brief` was 0.4.0 (stale) — **already fixed**: ran `cargo install --path .` → now 0.5.0. (Verifier must have current `brief` on PATH; demos use bare `brief` + repo-relative paths, verify from repo ROOT.)
7. **Showboat bug**: a command body containing literal ``` fences gets mangled (writer uses only 3 backticks → premature close). Avoid ``` inside exec COMMANDS. Output blocks containing ``` are fine (showboat auto-wraps in ````output).

## Status per demo (all under docs/demos/)
- **phase-1.md** — ✅ DONE, verifies exit 0. (Edited 6 command lines, regenerated via --output, swapped.)
- **2026-04-13-skill-scaffold.md** — ✅ DONE, rebuilt from scratch via showboat (uses `--description` + `--from-brief`, self-contained `cd /tmp/brief-scaffold-demo`). exit 0.
- **2026-04-13-skill-emit.md** — ✅ DONE, rebuilt via showboat (proper header; was hand-written, lacked showboat-id → caused `**` artifact). exit 0.
- **2026-03-29-unknown-sections.md** — ✅ DONE, rebuilt §1-§8 via showboat. Added new fixture `tests/fixtures/rich-unknown.brief.md` (checked in) to avoid heredoc-with-fences bug; demo cats/emits it. exit 0.
- **2026-03-31-install.md** — ⚠️ IN PROGRESS. Edits made to SOURCE: arg-order on §7 (`--file new.brief.md emit claude --install`), §7 cp now repo-relative `tests/fixtures/minimal.brief.md`, §7 marker count now `grep -c "<brief:generated>"`, intro+§2 prose updated to name `<brief:generated>` markers. **NEXT STEP (was interrupted):** run:
  ```
  cd /Volumes/ExternalData/admin/Developer/Projects/brief
  showboat verify docs/demos/2026-03-31-install.md --output /tmp/install.new.md >/dev/null 2>&1
  cp /tmp/install.new.md docs/demos/2026-03-31-install.md
  showboat verify docs/demos/2026-03-31-install.md >/dev/null 2>&1; echo "exit: $?"   # expect 0
  grep -n 'Marker count: [0-9]' docs/demos/2026-03-31-install.md  # expect "Marker count: 1"
  ```
  Confirm Marker count = 1 (not 2 — `<brief:generated>` opening tag only; `brief:generated` alone would match both tags = 2). Prose still names markers correctly.
- **2026-05-09-cursor-emit.md** — ❌ NOT STARTED. Authored on Windows: has `C:\Users\jikan\...` and `/c/Developer/Projects/brief/target/debug/brief` paths → fails on mac. Needs full rewrite using `brief` (PATH) + `/tmp` workspace, repo-relative. ALSO §6 "Design boundary: why a single bundled rule" is now factually STALE — it says brief has no scope concept, but **P8 scoped constraints shipped** (commit e14a901). The existing correction note at top (2026-06-12) about `globs` array-vs-comma-string must be reconciled. Rewrite §6 to reflect scoped constraints now exist. Verify cursor emit current behavior first: `brief --file <f> emit cursor`.

## Gap analysis (after the 6 are green) — NEW demos to consider for full coverage
Targets with NO demo: copilot, windsurf, aider, anchor, xml. New feature with NO demo: **P8 scoped constraints** (format-level path scoping — `- [\`src/ui/**\`] WCAG 2.1 AA`, see docs/brief-format.md §8.1) and P5 `--install` enhancements (`--position`, `--full`, `--uninstall`), `validate-diff` CI gate. Headline: P8 scoped constraints deserves its own demo (exercise `check`, scoped emit). Confirm scope/priority with user before adding many.

## Conventions adopted
- Verify from repo root; bare `brief` (PATH, now 0.5.0) or `./target/debug/brief`; repo-relative fixture paths; `/tmp/<name>-demo` workspaces with `cd` embedded in each exec for portability; clean up workspace at end.
- New fixture added: `tests/fixtures/rich-unknown.brief.md`.

## Environment note
Caveman mode active (full). User's project memory: NO AI co-author trailers on commits. Nothing committed yet this session — all changes are working-tree only. `.brief.md` had pre-existing unstaged mod (M) at session start.