use anyhow::{anyhow, Context, Result};
use std::fs;
use std::path::PathBuf;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct SkillFrontmatter {
    name: String,
    description: String,
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

    // 1. Line count validation (< 500 lines)
    let line_count = content.lines().count();
    if line_count >= 500 {
        return Err(anyhow!("SKILL.md is too long: {} lines (must be < 500 lines)", line_count));
    }

    // 2. Frontmatter validation
    if !content.starts_with("---") {
        return Err(anyhow!("Missing YAML frontmatter in SKILL.md"));
    }

    let after_opening = &content[3..];
    let end_pos = after_opening.find("\n---")
        .ok_or_else(|| anyhow!("Unclosed frontmatter in SKILL.md"))?;
    
    let yaml_str = &after_opening[..end_pos];
    let fm: SkillFrontmatter = serde_yaml::from_str(yaml_str)
        .context("Failed to parse YAML frontmatter in SKILL.md")?;

    // 3. Name format validation (kebab-case)
    if !fm.name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
         return Err(anyhow!("Invalid name format: '{}'. Name must be lowercase, alphanumeric, and use hyphens (kebab-case).", fm.name));
    }

    // 4. Description length validation (<= 1024 chars)
    if fm.description.len() > 1024 {
        return Err(anyhow!("Invalid description length: {} chars (must be ≤1024 chars)", fm.description.len()));
    }

    println!("SKILL.md at {:?} is valid.", skill_md_path);
    Ok(())
}
