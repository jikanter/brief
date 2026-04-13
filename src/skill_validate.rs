use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct SkillFrontmatter {
    name: String,
    description: String,
}

const MAX_NAME_LEN: usize = 64;
const MAX_DESCRIPTION_LEN: usize = 1024;
const MAX_LINES: usize = 500;

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
    let fm: SkillFrontmatter = serde_yaml::from_str(yaml_str)
        .context("Failed to parse YAML frontmatter in SKILL.md")?;

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

pub fn validate_skill(path: &PathBuf) -> Result<()> {
    let skill_md_path = if path.is_dir() {
        path.join("SKILL.md")
    } else {
        path.clone()
    };

    if !skill_md_path.exists() {
        return Err(anyhow!("SKILL.md not found at {:?}", skill_md_path));
    }

    let content = fs::read_to_string(&skill_md_path)
        .with_context(|| format!("Failed to read SKILL.md at {:?}", skill_md_path))?;

    validate_skill_content(&content)?;

    println!("SKILL.md at {:?} is valid.", skill_md_path);
    Ok(())
}
