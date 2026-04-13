use anyhow::{anyhow, Result};
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

pub fn scaffold_skill(
    from_doc: Option<PathBuf>,
    from_workflow: Option<PathBuf>,
    interactive: bool,
) -> Result<()> {
    let (name, description, instructions) = if interactive {
        prompt_for_skill()?
    } else if let Some(doc_path) = from_doc {
        let doc_content = fs::read_to_string(&doc_path)?;
        // Simple heuristic for now: first line as name, rest as instructions
        let lines: Vec<&str> = doc_content.lines().collect();
        let name = lines.first().unwrap_or(&"new-skill").to_lowercase().replace(" ", "-");
        let description = format!("Skill generated from {}", doc_path.display());
        (name, description, doc_content)
    } else if let Some(workflow_path) = from_workflow {
        let workflow_content = fs::read_to_string(&workflow_path)?;
        let instructions = format!("Follow these steps from the observed workflow:\n\n{}", workflow_content);
        ("workflow-skill".to_string(), "Generated from workflow".to_string(), instructions)
    } else {
        return Err(anyhow!("Must specify --interactive, --from-doc <path>, or --from-workflow <path>"));
    };

    let skill_dir = PathBuf::from(&name);
    if skill_dir.exists() {
        return Err(anyhow!("Directory {:?} already exists", skill_dir));
    }

    fs::create_dir_all(&skill_dir)?;
    fs::create_dir_all(skill_dir.join("scripts"))?;
    fs::create_dir_all(skill_dir.join("references"))?;

    let skill_md_content = format!(
        "---\nname: {}\ndescription: {}\n---\n\n{}\n",
        name, description, instructions
    );

    fs::write(skill_dir.join("SKILL.md"), skill_md_content)?;

    println!("Scaffolded skill in {:?}", skill_dir);
    Ok(())
}

fn prompt_for_skill() -> Result<(String, String, String)> {
    println!("Interactive Skill Scaffolding");
    println!("----------------------------");
    
    let mut name = prompt("Skill name (kebab-case): ")?;
    while name.is_empty() || name.contains(' ') || name.chars().any(|c| c.is_uppercase()) {
        println!("Error: Name must be non-empty, kebab-case (lowercase, no spaces).");
        name = prompt("Skill name (kebab-case): ")?;
    }
    
    let mut description = prompt("Skill description (max 1024 chars): ")?;
    while description.is_empty() || description.len() > 1024 {
        println!("Error: Description must be non-empty and ≤ 1024 characters.");
        description = prompt("Skill description: ")?;
    }
    
    println!("Enter skill instructions (end with an empty line):");
    let mut instructions = String::new();
    loop {
        let line = prompt("> ")?;
        if line.is_empty() {
            break;
        }
        instructions.push_str(&line);
        instructions.push('\n');
    }
    
    if instructions.is_empty() {
        instructions = "Add instructions here.".to_string();
    }

    Ok((name, description, instructions))
}

fn prompt(label: &str) -> Result<String> {
    print!("{}", label);
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}
