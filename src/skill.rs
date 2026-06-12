//! Skill authoring across the brief-owned / user-owned boundary (P7).
//!
//! A `SKILL.md` is a co-edit surface. brief owns exactly two regions and the
//! user owns everything else:
//!
//! - **`metadata.brief.source`** — the single load-bearing key. A pointer back
//!   to the originating `.brief.md` (or the literal `--description` text). It is
//!   the ownership marker: its presence is what `install`/`uninstall` read to
//!   know a skill is brief-managed.
//! - the optional **`<brief:generated>` body fence** — if brief injects body
//!   content, it lives between these markers (the same XML-style markers
//!   `brief emit claude --install` uses). Everything outside the fence is the
//!   user's.
//!
//! Encoding this separation once is the forced primitive behind all four P7
//! commands ([`search`], [`scaffold`], [`install`], [`uninstall`]): only with it
//! can any of them be re-run safely on a hand-edited skill. The frontmatter
//! edits here are deliberately *surgical* — line-level, not a YAML re-serialize —
//! so user formatting, comments, and key order survive byte-for-byte.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use serde::Deserialize;

use crate::emit::markers::{find_marker_pairs, inject_section, wrap_with_markers};
use crate::emit::skill::{emit_skill, relative_path};
use crate::parse::parse_brief;

/// The flat metadata key brief owns inside a SKILL.md frontmatter `metadata:`
/// map. Kept as a dotted scalar key (not a nested `brief: { source }`) to match
/// what `emit_skill` has always written.
pub const BRIEF_SOURCE_KEY: &str = "brief.source";

const MAX_NAME_LEN: usize = 64;
const MAX_DESCRIPTION_LEN: usize = 1024;
const MAX_LINES: usize = 500;

// ---------------------------------------------------------------------------
// Validation (the agentskills.io spec checks; messages must stay stable —
// tests/skill_commands.rs asserts on these substrings).
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct SkillFrontmatter {
    name: String,
    description: String,
}

/// Validate a skill name against the agentskills.io spec.
pub fn validate_skill_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(anyhow!("Invalid name format: name must not be empty"));
    }
    if name.len() > MAX_NAME_LEN {
        return Err(anyhow!(
            "Invalid name format '{}': too long ({} chars, max {})",
            name,
            name.len(),
            MAX_NAME_LEN
        ));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(anyhow!(
            "Invalid name format '{}': must be lowercase alphanumeric with hyphens (kebab-case)",
            name
        ));
    }
    if name.starts_with('-') || name.ends_with('-') {
        return Err(anyhow!(
            "Invalid name format '{}': must not start or end with a hyphen",
            name
        ));
    }
    if name.contains("--") {
        return Err(anyhow!(
            "Invalid name format '{}': must not contain consecutive hyphens",
            name
        ));
    }
    Ok(())
}

/// Validate a SKILL.md content string against the agentskills.io spec.
pub fn validate_skill_content(content: &str) -> Result<()> {
    let line_count = content.lines().count();
    if line_count >= MAX_LINES {
        return Err(anyhow!(
            "SKILL.md is too long: {} lines (must be < {} lines)",
            line_count,
            MAX_LINES
        ));
    }

    if !content.starts_with("---") {
        return Err(anyhow!("Missing YAML frontmatter in SKILL.md"));
    }

    let after_opening = &content[3..];
    let end_pos = after_opening
        .find("\n---")
        .ok_or_else(|| anyhow!("Unclosed frontmatter in SKILL.md"))?;

    let yaml_str = &after_opening[..end_pos];
    let fm: SkillFrontmatter =
        serde_yaml::from_str(yaml_str).context("Failed to parse YAML frontmatter in SKILL.md")?;

    validate_skill_name(&fm.name)?;

    if fm.description.is_empty() {
        return Err(anyhow!("Description must not be empty"));
    }
    if fm.description.len() > MAX_DESCRIPTION_LEN {
        return Err(anyhow!(
            "Invalid description length: {} chars (must be ≤{} chars)",
            fm.description.len(),
            MAX_DESCRIPTION_LEN
        ));
    }
    if fm.description.contains('<') || fm.description.contains('>') {
        return Err(anyhow!(
            "Invalid description: must not contain angle brackets ('<' or '>')"
        ));
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// The boundary: surgical frontmatter edits + body-fence handling.
// ---------------------------------------------------------------------------

/// The inner line range `[start, close)` of the YAML frontmatter, given the
/// document split into lines. `None` if the doc does not open with `---`.
fn frontmatter_bounds(lines: &[String]) -> Option<(usize, usize)> {
    if lines.first().map(String::as_str) != Some("---") {
        return None;
    }
    let close = (1..lines.len()).find(|&i| lines[i] == "---")?;
    Some((1, close))
}

/// Does a line look like a child of a YAML block (indented, non-empty)?
fn is_indented(line: &str) -> bool {
    !line.trim().is_empty() && line.starts_with([' ', '\t'])
}

/// Read `metadata.brief.source` from a SKILL.md, if present.
pub fn get_brief_source(content: &str) -> Option<String> {
    let lines: Vec<String> = content.lines().map(String::from).collect();
    let (start, close) = frontmatter_bounds(&lines)?;
    let meta_idx = (start..close).find(|&i| lines[i].trim_start().starts_with("metadata:"))?;
    let mut i = meta_idx + 1;
    while i < close && is_indented(&lines[i]) {
        if let Some(rest) = lines[i]
            .trim_start()
            .strip_prefix(&format!("{BRIEF_SOURCE_KEY}:"))
        {
            return Some(unquote_yaml(rest.trim()));
        }
        i += 1;
    }
    None
}

/// True when the SKILL.md carries brief's ownership marker.
pub fn is_brief_owned(content: &str) -> bool {
    get_brief_source(content).is_some()
}

/// Set (or insert) `metadata.brief.source`, preserving every other line of the
/// document byte-for-byte. The brief-owned key is the only thing that moves.
pub fn set_brief_source(content: &str, source: &str) -> String {
    let trailing_nl = content.ends_with('\n');
    let mut lines: Vec<String> = content.lines().map(String::from).collect();
    let value = format!("{BRIEF_SOURCE_KEY}: {}", quote_yaml(source));

    if let Some((start, close)) = frontmatter_bounds(&lines) {
        if let Some(meta_idx) =
            (start..close).find(|&i| lines[i].trim_start().starts_with("metadata:"))
        {
            // Find an existing brief.source child to replace.
            let mut i = meta_idx + 1;
            let mut existing = None;
            while i < close && is_indented(&lines[i]) {
                if lines[i]
                    .trim_start()
                    .starts_with(&format!("{BRIEF_SOURCE_KEY}:"))
                {
                    existing = Some(i);
                    break;
                }
                i += 1;
            }
            match existing {
                Some(i) => lines[i] = format!("  {value}"),
                None => lines.insert(meta_idx + 1, format!("  {value}")),
            }
        } else {
            // No metadata block yet — add one just before the closing `---`.
            lines.insert(close, format!("  {value}"));
            lines.insert(close, "metadata:".to_string());
        }
    } else {
        // No frontmatter at all — synthesize a minimal one.
        let mut prefixed = vec![
            "---".to_string(),
            "metadata:".to_string(),
            format!("  {value}"),
            "---".to_string(),
        ];
        prefixed.extend(lines);
        lines = prefixed;
    }

    let mut out = lines.join("\n");
    if trailing_nl {
        out.push('\n');
    }
    out
}

/// Replace (or insert) the `<brief:generated>` body fence with `body`, or strip
/// it when `body` is `None`. User content outside the fence is untouched.
pub fn set_brief_body(content: &str, body: Option<&str>) -> String {
    match body {
        Some(b) => {
            let wrapped = wrap_with_markers(b);
            let (out, _pairs) = inject_section(content, &wrapped);
            out
        }
        None => {
            let pairs = find_marker_pairs(content);
            if pairs.is_empty() {
                return content.to_string();
            }
            let mut out = String::with_capacity(content.len());
            let mut cursor = 0;
            for p in pairs {
                out.push_str(&content[cursor..p.pair_start]);
                cursor = p.pair_end;
            }
            out.push_str(&content[cursor..]);
            out
        }
    }
}

/// Minimal YAML scalar quoting: wrap in double quotes when the value contains
/// characters that would confuse a bare scalar. Mirrors the cursor emitter.
fn quote_yaml(s: &str) -> String {
    let needs = s.is_empty()
        || s.chars().any(|c| {
            matches!(
                c,
                ':' | '#'
                    | '['
                    | ']'
                    | '{'
                    | '}'
                    | ','
                    | '&'
                    | '*'
                    | '!'
                    | '|'
                    | '>'
                    | '\''
                    | '"'
                    | '%'
                    | '@'
                    | '`'
                    | '\n'
            )
        })
        || s.starts_with(' ')
        || s.ends_with(' ');
    if needs {
        format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
    } else {
        s.to_string()
    }
}

/// Inverse of [`quote_yaml`] for the narrow cases we emit.
fn unquote_yaml(s: &str) -> String {
    let t = s.trim();
    if t.len() >= 2 && t.starts_with('"') && t.ends_with('"') {
        t[1..t.len() - 1]
            .replace("\\\"", "\"")
            .replace("\\\\", "\\")
    } else {
        t.to_string()
    }
}

// ---------------------------------------------------------------------------
// scaffold
// ---------------------------------------------------------------------------

/// Where a scaffolded skill draws its name/description/body from.
pub struct ScaffoldOptions {
    pub description: Option<String>,
    pub from_brief: Option<PathBuf>,
    pub name: Option<String>,
}

/// Generate a spec-compliant skill skeleton under `parent/<name>/`.
///
/// Source precedence: `--from-brief` → `--description` → the active `.brief.md`
/// (`active_brief`) if one is in scope. Stamps `metadata.brief.source`, writes
/// `SKILL.md`, and creates empty `scripts/` + `references/`. Returns the created
/// skill directory.
pub fn scaffold(
    opts: &ScaffoldOptions,
    active_brief: Option<&Path>,
    parent: &Path,
) -> Result<PathBuf> {
    // Resolve the brief-first source (from_brief, else active brief).
    let brief_path = opts
        .from_brief
        .clone()
        .or_else(|| active_brief.map(Path::to_path_buf));

    enum Source {
        Brief(PathBuf),
        Description(String),
    }
    let source = match (&brief_path, &opts.description) {
        (Some(p), _) => Source::Brief(p.clone()),
        (None, Some(d)) => Source::Description(d.clone()),
        (None, None) => {
            return Err(anyhow!(
                "scaffold needs a source: pass --from-brief <file>, --description \"<text>\", or run where a .brief.md is in scope"
            ));
        }
    };

    // Determine the skill name (explicit --name wins).
    let name = match (&opts.name, &source) {
        (Some(n), _) => n.clone(),
        (None, Source::Brief(p)) => {
            let content = std::fs::read_to_string(p)
                .with_context(|| format!("Failed to read {}", p.display()))?;
            let brief = parse_brief(&content).context("Failed to parse briefing")?;
            crate::emit::skill_name(&brief)
        }
        (None, Source::Description(d)) => slugify(d),
    };
    validate_skill_name(&name)?;

    let skill_dir = parent.join(&name);
    if skill_dir.exists() {
        return Err(anyhow!("Directory {} already exists", skill_dir.display()));
    }
    std::fs::create_dir_all(skill_dir.join("scripts"))?;
    std::fs::create_dir_all(skill_dir.join("references"))?;

    let skill_md = match &source {
        Source::Brief(p) => {
            let content = std::fs::read_to_string(p)?;
            let brief = parse_brief(&content).context("Failed to parse briefing")?;
            // Relative pointer from the SKILL.md back to the brief.
            let brief_canon = std::fs::canonicalize(p)
                .with_context(|| format!("Failed to canonicalize {}", p.display()))?;
            let dir_canon = std::fs::canonicalize(&skill_dir)?;
            let rel = relative_path(&brief_canon, &dir_canon);
            emit_skill(&brief, Some(&rel))
        }
        Source::Description(d) => description_skeleton(&name, d),
    };

    validate_skill_content(&skill_md)
        .context("Scaffolded SKILL.md would not pass agentskills.io validation")?;
    std::fs::write(skill_dir.join("SKILL.md"), &skill_md)?;

    Ok(skill_dir)
}

/// Build a minimal, spec-valid SKILL.md for the description-first path. The
/// literal description is stamped as `brief.source` per the P7 spec.
fn description_skeleton(name: &str, description: &str) -> String {
    let title = title_from_name(name);
    format!(
        "---\nname: {name}\ndescription: {desc}\nmetadata:\n  {key}: {src}\n---\n\n# {title}\n\nAdd instructions here.\n",
        desc = quote_yaml(description),
        key = BRIEF_SOURCE_KEY,
        src = quote_yaml(description),
    )
}

fn slugify(input: &str) -> String {
    input
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn title_from_name(name: &str) -> String {
    name.split('-')
        .filter(|s| !s.is_empty())
        .map(|seg| {
            let mut chars = seg.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

// ---------------------------------------------------------------------------
// install / uninstall
// ---------------------------------------------------------------------------

/// Install a skill directory into `<dest_root>/.claude/skills/<name>/`.
///
/// Idempotent across the boundary: when the destination already exists, only
/// `metadata.brief.source` and the `<brief:generated>` body fence are synced
/// from the source; all other destination fields and body content are preserved
/// byte-for-byte. On a first install the source `SKILL.md` is copied wholesale.
/// Returns the destination SKILL.md path.
pub fn install(src_dir: &Path, dest_root: &Path) -> Result<PathBuf> {
    let src_md = src_dir.join("SKILL.md");
    let src_content = std::fs::read_to_string(&src_md)
        .with_context(|| format!("Failed to read {}", src_md.display()))?;
    validate_skill_content(&src_content)
        .context("Source SKILL.md would not pass agentskills.io validation")?;

    let name = read_name(&src_content)?;
    let dest_dir = dest_root.join(".claude").join("skills").join(&name);
    let dest_md = dest_dir.join("SKILL.md");

    let merged = if dest_md.exists() {
        // Re-install: keep the destination's user regions, sync only brief's.
        let dest_content = std::fs::read_to_string(&dest_md)?;
        let mut out = dest_content;
        if let Some(src) = get_brief_source(&src_content) {
            out = set_brief_source(&out, &src);
        }
        let src_body = extract_brief_body(&src_content);
        out = set_brief_body(&out, src_body.as_deref());
        out
    } else {
        src_content
    };

    std::fs::create_dir_all(&dest_dir)?;
    std::fs::write(&dest_md, &merged)?;
    Ok(dest_md)
}

/// Uninstall a brief-managed skill from `<root>/.claude/skills/<name>/`.
///
/// Refuses (without `force`) to remove a skill that lacks brief's ownership
/// marker — that is a hand-authored skill brief did not create. v1 gates on the
/// `metadata.brief.source` marker; finer "edited outside brief regions"
/// detection would need a stored baseline hash and is deferred per YAGNI.
pub fn uninstall(name: &str, root: &Path, force: bool) -> Result<PathBuf> {
    let dir = root.join(".claude").join("skills").join(name);
    let md = dir.join("SKILL.md");
    if !md.exists() {
        return Err(anyhow!(
            "No installed skill named '{name}' at {}",
            dir.display()
        ));
    }
    let content = std::fs::read_to_string(&md)?;
    if !is_brief_owned(&content) && !force {
        return Err(anyhow!(
            "Skill '{name}' is not brief-managed (no metadata.{BRIEF_SOURCE_KEY}); refusing to remove. Re-run with --force to override."
        ));
    }
    std::fs::remove_dir_all(&dir).with_context(|| format!("Failed to remove {}", dir.display()))?;
    Ok(dir)
}

/// The content inside the `<brief:generated>` body fence, if any.
fn extract_brief_body(content: &str) -> Option<String> {
    let pairs = find_marker_pairs(content);
    let first = pairs.first()?;
    Some(content[first.content_start..first.content_end].to_string())
}

fn read_name(content: &str) -> Result<String> {
    let lines: Vec<String> = content.lines().map(String::from).collect();
    let (start, close) =
        frontmatter_bounds(&lines).ok_or_else(|| anyhow!("SKILL.md has no YAML frontmatter"))?;
    for line in &lines[start..close] {
        if let Some(rest) = line.strip_prefix("name:") {
            return Ok(unquote_yaml(rest.trim()));
        }
    }
    Err(anyhow!("SKILL.md frontmatter has no `name` field"))
}

// ---------------------------------------------------------------------------
// search
// ---------------------------------------------------------------------------

/// A local skill discovered during [`search`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillHit {
    pub name: String,
    pub description: String,
    pub path: PathBuf,
    pub score: u32,
}

/// The local skill roots searched, in order. `~` resolves via `home`.
pub fn default_skill_roots(cwd: &Path, home: Option<&Path>) -> Vec<PathBuf> {
    let mut roots = vec![
        cwd.join(".claude").join("skills"),
        cwd.join(".agents").join("skills"),
    ];
    if let Some(h) = home {
        roots.push(h.join(".claude").join("skills"));
    }
    roots
}

/// Search local skill roots for skills whose name or description matches
/// `query` (case-insensitive substring). Local-only, no network. Results are
/// ranked: a name match outweighs a description match.
pub fn search(query: &str, roots: &[PathBuf]) -> Vec<SkillHit> {
    let q = query.to_lowercase();
    let mut hits = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for root in roots {
        let Ok(entries) = std::fs::read_dir(root) else {
            continue;
        };
        for entry in entries.flatten() {
            let md = entry.path().join("SKILL.md");
            let Ok(content) = std::fs::read_to_string(&md) else {
                continue;
            };
            let Some((name, description)) = read_name_and_description(&content) else {
                continue;
            };
            if !seen.insert(name.clone()) {
                continue; // earlier root wins for a duplicate name
            }
            let name_hit = name.to_lowercase().contains(&q);
            let desc_hit = description.to_lowercase().contains(&q);
            let score = u32::from(name_hit) * 2 + u32::from(desc_hit);
            if score > 0 {
                hits.push(SkillHit {
                    name,
                    description,
                    path: md,
                    score,
                });
            }
        }
    }

    hits.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.name.cmp(&b.name)));
    hits
}

fn read_name_and_description(content: &str) -> Option<(String, String)> {
    let lines: Vec<String> = content.lines().map(String::from).collect();
    let (start, close) = frontmatter_bounds(&lines)?;
    let mut name = None;
    let mut description = None;
    for line in &lines[start..close] {
        if let Some(rest) = line.strip_prefix("name:") {
            name = Some(unquote_yaml(rest.trim()));
        } else if let Some(rest) = line.strip_prefix("description:") {
            description = Some(unquote_yaml(rest.trim()));
        }
    }
    Some((name?, description?))
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- boundary: get/set brief.source --

    #[test]
    fn get_brief_source_reads_the_key() {
        let md = "---\nname: x\ndescription: d\nmetadata:\n  brief.source: ../../x.brief.md\n---\n\nBody\n";
        assert_eq!(get_brief_source(md).as_deref(), Some("../../x.brief.md"));
        assert!(is_brief_owned(md));
    }

    #[test]
    fn get_brief_source_absent_is_none() {
        let md = "---\nname: x\ndescription: d\n---\n\nBody\n";
        assert_eq!(get_brief_source(md), None);
        assert!(!is_brief_owned(md));
    }

    #[test]
    fn set_brief_source_inserts_metadata_block_when_absent() {
        let md = "---\nname: x\ndescription: d\n---\n\nBody\n";
        let out = set_brief_source(md, "../a.brief.md");
        assert_eq!(get_brief_source(&out).as_deref(), Some("../a.brief.md"));
        assert!(out.contains("name: x"));
        assert!(out.contains("description: d"));
        assert!(out.ends_with("Body\n"));
    }

    #[test]
    fn set_brief_source_replaces_existing_value_only() {
        let md = "---\nname: x\ndescription: d\nlicense: MIT\nmetadata:\n  brief.source: old\n  other: keep\n---\n\nBody\n";
        let out = set_brief_source(md, "new");
        assert_eq!(get_brief_source(&out).as_deref(), Some("new"));
        assert!(!out.contains("brief.source: old"));
        // User fields preserved byte-for-byte.
        assert!(out.contains("license: MIT"));
        assert!(out.contains("  other: keep"));
    }

    #[test]
    fn set_brief_source_is_idempotent() {
        let md = "---\nname: x\ndescription: d\n---\n\nBody\n";
        let once = set_brief_source(md, "src");
        let twice = set_brief_source(&once, "src");
        assert_eq!(once, twice);
    }

    // -- boundary: body fence --

    #[test]
    fn set_brief_body_appends_then_replaces() {
        let md = "---\nname: x\ndescription: d\n---\n\nUser body.\n";
        let with = set_brief_body(md, Some("generated v1\n"));
        assert!(with.contains("<brief:generated>"));
        assert!(with.contains("generated v1"));
        assert!(with.contains("User body."));

        let updated = set_brief_body(&with, Some("generated v2\n"));
        assert!(updated.contains("generated v2"));
        assert!(!updated.contains("generated v1"));
        assert_eq!(updated.matches("<brief:generated>").count(), 1);
    }

    #[test]
    fn set_brief_body_none_strips_fence_keeps_user_content() {
        let md = "Intro\n<brief:generated>\ngen\n</brief:generated>\nOutro\n";
        let out = set_brief_body(md, None);
        assert!(!out.contains("<brief:generated>"));
        assert!(!out.contains("gen"));
        assert!(out.contains("Intro"));
        assert!(out.contains("Outro"));
    }

    // -- yaml quoting round-trip --

    #[test]
    fn quote_unquote_round_trip() {
        for s in ["plain", "has: colon", "review PRs", "a\"b"] {
            let q = quote_yaml(s);
            assert_eq!(unquote_yaml(&q), s, "round-trip failed for {s:?}");
        }
    }

    #[test]
    fn set_brief_source_quotes_value_with_colon() {
        let md = "---\nname: x\ndescription: d\n---\n\nBody\n";
        let out = set_brief_source(md, "review: PRs for security");
        // Must still parse + read back exactly.
        assert_eq!(
            get_brief_source(&out).as_deref(),
            Some("review: PRs for security")
        );
    }

    // -- name reading --

    #[test]
    fn read_name_handles_quoted_and_plain() {
        assert_eq!(read_name("---\nname: foo\n---\n").unwrap(), "foo");
        assert_eq!(read_name("---\nname: \"foo\"\n---\n").unwrap(), "foo");
    }

    // -- search ranking --

    #[test]
    fn search_ranks_name_over_description() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join(".claude").join("skills");
        write_skill(&root, "review-code", "Helps review pull requests");
        write_skill(&root, "deploy", "review code before deploy");

        let hits = search("review", std::slice::from_ref(&root));
        assert_eq!(hits.len(), 2);
        // name match (review-code) outranks description-only match (deploy).
        assert_eq!(hits[0].name, "review-code");
        assert_eq!(hits[1].name, "deploy");
    }

    #[test]
    fn search_no_match_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join(".claude").join("skills");
        write_skill(&root, "deploy", "ship the service");
        assert!(search("nonexistent", &[root]).is_empty());
    }

    fn write_skill(root: &Path, name: &str, description: &str) {
        let d = root.join(name);
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(
            d.join("SKILL.md"),
            format!("---\nname: {name}\ndescription: {description}\n---\n\nBody\n"),
        )
        .unwrap();
    }

    // -- scaffold --

    #[test]
    fn scaffold_from_description_creates_valid_skill() {
        let dir = tempfile::tempdir().unwrap();
        let opts = ScaffoldOptions {
            description: Some("Review PRs for security issues".into()),
            from_brief: None,
            name: Some("security-review".into()),
        };
        let created = scaffold(&opts, None, dir.path()).unwrap();
        assert_eq!(created, dir.path().join("security-review"));
        assert!(created.join("scripts").is_dir());
        assert!(created.join("references").is_dir());
        let md = std::fs::read_to_string(created.join("SKILL.md")).unwrap();
        assert!(md.contains("name: security-review"));
        assert!(is_brief_owned(&md));
        validate_skill_content(&md).unwrap();
    }

    #[test]
    fn scaffold_requires_a_source() {
        let dir = tempfile::tempdir().unwrap();
        let opts = ScaffoldOptions {
            description: None,
            from_brief: None,
            name: None,
        };
        assert!(scaffold(&opts, None, dir.path()).is_err());
    }

    #[test]
    fn scaffold_refuses_existing_dir() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("taken")).unwrap();
        let opts = ScaffoldOptions {
            description: Some("desc".into()),
            from_brief: None,
            name: Some("taken".into()),
        };
        let err = scaffold(&opts, None, dir.path()).unwrap_err();
        assert!(err.to_string().contains("already exists"));
    }

    // -- install / uninstall --

    #[test]
    fn install_copies_then_preserves_user_edits_on_reinstall() {
        let dir = tempfile::tempdir().unwrap();
        // A scaffolded source skill.
        let src = dir.path().join("src-skill");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(
            src.join("SKILL.md"),
            "---\nname: review\ndescription: Review code\nmetadata:\n  brief.source: ../../review.brief.md\n---\n\nGenerated guidance.\n",
        )
        .unwrap();

        let dest_root = dir.path().join("project");
        let installed = install(&src, &dest_root).unwrap();
        assert!(installed.ends_with(".claude/skills/review/SKILL.md"));

        // User hand-edits the installed description + body.
        std::fs::write(
            &installed,
            "---\nname: review\ndescription: Review code MY WAY\nmetadata:\n  brief.source: ../../review.brief.md\n---\n\nMy own guidance.\n",
        )
        .unwrap();

        // Re-install with a changed source pointer: only brief.source syncs.
        std::fs::write(
            src.join("SKILL.md"),
            "---\nname: review\ndescription: Review code\nmetadata:\n  brief.source: ../../moved.brief.md\n---\n\nGenerated guidance.\n",
        )
        .unwrap();
        install(&src, &dest_root).unwrap();

        let after = std::fs::read_to_string(&installed).unwrap();
        assert!(after.contains("Review code MY WAY"), "user field preserved");
        assert!(after.contains("My own guidance."), "user body preserved");
        assert_eq!(
            get_brief_source(&after).as_deref(),
            Some("../../moved.brief.md"),
            "brief.source synced"
        );
    }

    #[test]
    fn uninstall_removes_brief_owned_skill() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let skills = root.join(".claude").join("skills").join("review");
        std::fs::create_dir_all(&skills).unwrap();
        std::fs::write(
            skills.join("SKILL.md"),
            "---\nname: review\ndescription: d\nmetadata:\n  brief.source: x\n---\n\nB\n",
        )
        .unwrap();

        uninstall("review", root, false).unwrap();
        assert!(!skills.exists());
    }

    #[test]
    fn uninstall_refuses_non_brief_skill_without_force() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let skills = root.join(".claude").join("skills").join("handmade");
        std::fs::create_dir_all(&skills).unwrap();
        std::fs::write(
            skills.join("SKILL.md"),
            "---\nname: handmade\ndescription: d\n---\n\nB\n",
        )
        .unwrap();

        let err = uninstall("handmade", root, false).unwrap_err();
        assert!(err.to_string().contains("not brief-managed"));
        assert!(skills.exists(), "must not remove without --force");

        // With --force it goes.
        uninstall("handmade", root, true).unwrap();
        assert!(!skills.exists());
    }
}
