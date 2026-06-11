use anyhow::{Context, Result, anyhow};
use std::fs;
use std::path::Path;

// The spec checks live in the library (src/skill.rs) so the P7 commands and the
// CLI share one implementation. Re-exported here for the existing call sites.
pub use brief_cli::skill::{validate_skill_content, validate_skill_name};

pub fn validate_skill(path: &Path) -> Result<()> {
    let skill_md_path = if path.is_dir() {
        path.join("SKILL.md")
    } else {
        path.to_path_buf()
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

#[cfg(test)]
mod metadata_tests {
    use super::*;

    #[test]
    fn validator_accepts_metadata_block_with_brief_source() {
        let content = "---\n\
name: review\n\
description: Review code\n\
metadata:\n  \
  brief.source: ../../some.brief.md\n\
---\n\n\
Body.\n";
        validate_skill_content(content)
            .expect("metadata block with brief.source should be accepted");
    }
}
