use anyhow::{Context, Result, anyhow};
use std::fs;
use std::path::PathBuf;

use crate::skill_validate::validate_skill_name;

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
        .map(|segment| {
            let mut chars = segment.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn init_skill(name: Option<String>) -> Result<()> {
    let name = match name {
        Some(n) => n,
        None => {
            let cwd = std::env::current_dir().context("Failed to get current directory")?;
            let dir_name = cwd
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| anyhow!("Could not determine current directory name"))?;
            slugify(dir_name)
        }
    };

    validate_skill_name(&name)?;

    let skill_dir = PathBuf::from(&name);
    if skill_dir.exists() {
        return Err(anyhow!("Directory {:?} already exists", skill_dir));
    }

    fs::create_dir_all(&skill_dir)
        .with_context(|| format!("Failed to create {}", skill_dir.display()))?;
    fs::create_dir_all(skill_dir.join("scripts"))?;
    fs::create_dir_all(skill_dir.join("references"))?;

    let title = title_from_name(&name);
    let skill_md_content = format!(
        "---\nname: {name}\ndescription: TODO describe what this skill does and when to use it\n---\n\n# {title}\n\nAdd instructions here.\n"
    );

    fs::write(skill_dir.join("SKILL.md"), skill_md_content)
        .with_context(|| format!("Failed to write {}", skill_dir.join("SKILL.md").display()))?;

    println!("Created skill at {}", skill_dir.display());
    Ok(())
}
