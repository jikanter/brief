use std::path::PathBuf;
use std::process;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use colored::*;

use brief_cli::check::check_path;
use brief_cli::emit;
use brief_cli::init::scaffold_brief;
use brief_cli::model::Severity;
use brief_cli::parse::parse_brief;
use brief_cli::validate::validate;

#[derive(Parser)]
#[command(name = "brief", about = "Structured briefings for AI coding agents")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Analyze the current directory and scaffold a .brief.md
    Init,

    /// Validate the current .brief.md against the codebase
    Validate {
        /// Path to the .brief.md file
        #[arg(default_value = ".brief.md")]
        file: PathBuf,
    },

    /// Transform .brief.md into a target format
    Emit {
        /// Output target format
        target: EmitTarget,

        /// Path to the .brief.md file
        #[arg(default_value = ".brief.md")]
        file: PathBuf,

        /// For skill target: install directly to .claude/skills/<name>/SKILL.md
        #[arg(long)]
        install: bool,
    },

    /// Check if a file path falls within a sacred region
    Check {
        /// The file path to check
        path: String,

        /// Path to the .brief.md file
        #[arg(long, default_value = ".brief.md")]
        file: PathBuf,
    },

    /// Show semantic differences between two briefing files
    Diff {
        /// First briefing file
        file1: PathBuf,
        /// Second briefing file
        file2: PathBuf,
    },
}

#[derive(Clone, ValueEnum)]
enum EmitTarget {
    /// Emit a CLAUDE.md section
    Claude,
    /// Emit raw system prompt text
    Prompt,
    /// Emit an AGENTS.md section
    AgentsMd,
    /// Emit structured JSON
    Json,
    /// Emit a Claude Code SKILL.md file
    Skill,
}

fn main() {
    let cli = Cli::parse();

    if let Err(e) = run(cli) {
        eprintln!("{}: {e:#}", "error".red().bold());
        process::exit(1);
    }
}

fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Init => cmd_init(),
        Commands::Validate { file } => cmd_validate(&file),
        Commands::Emit {
            target,
            file,
            install,
        } => cmd_emit(target, &file, install),
        Commands::Check { path, file } => cmd_check(&path, &file),
        Commands::Diff { file1, file2 } => cmd_diff(&file1, &file2),
    }
}

fn cmd_init() -> Result<()> {
    let cwd = std::env::current_dir().context("Failed to get current directory")?;
    let output_path = cwd.join(".brief.md");

    if output_path.exists() {
        eprintln!(
            "{}: .brief.md already exists. Remove it first or edit it directly.",
            "warning".yellow().bold()
        );
        process::exit(1);
    }

    let content = scaffold_brief(&cwd);
    std::fs::write(&output_path, &content).context("Failed to write .brief.md")?;

    println!("{} {}", "Created".green().bold(), output_path.display());
    println!("Edit the file to fill in your goal, constraints, and sacred regions.");

    Ok(())
}

fn cmd_validate(file: &PathBuf) -> Result<()> {
    let base_dir = file
        .parent()
        .map(|p| {
            if p.as_os_str().is_empty() {
                std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
            } else {
                p.to_path_buf()
            }
        })
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let content = std::fs::read_to_string(file)
        .with_context(|| format!("Failed to read {}", file.display()))?;

    let brief = parse_brief(&content).context("Failed to parse briefing")?;
    let diagnostics = validate(&brief, &base_dir);

    if diagnostics.is_empty() {
        println!("{} briefing is valid", "✓".green().bold());
        return Ok(());
    }

    let mut has_errors = false;
    for diag in &diagnostics {
        match diag.severity {
            Severity::Error => {
                has_errors = true;
                eprintln!("{} {}", "error:".red().bold(), diag.message);
            }
            Severity::Warning => {
                eprintln!("{} {}", "warning:".yellow().bold(), diag.message);
            }
        }
    }

    if has_errors {
        process::exit(1);
    } else {
        println!(
            "{} briefing is valid (with {} warning(s))",
            "✓".green().bold(),
            diagnostics.len()
        );
    }

    Ok(())
}

fn cmd_emit(target: EmitTarget, file: &PathBuf, install: bool) -> Result<()> {
    let content = std::fs::read_to_string(file)
        .with_context(|| format!("Failed to read {}", file.display()))?;

    let brief = parse_brief(&content).context("Failed to parse briefing")?;

    let output = match target {
        EmitTarget::Claude => emit::emit_claude(&brief),
        EmitTarget::Prompt => emit::emit_prompt(&brief),
        EmitTarget::AgentsMd => emit::emit_agents_md(&brief),
        EmitTarget::Json => emit::emit_json(&brief),
        EmitTarget::Skill => emit::emit_skill(&brief),
    };

    if install {
        if !matches!(target, EmitTarget::Skill) {
            anyhow::bail!("--install is only supported for the skill target");
        }
        let name = emit::skill_name(&brief);
        let skill_dir = PathBuf::from(".claude/skills").join(&name);
        std::fs::create_dir_all(&skill_dir)
            .with_context(|| format!("Failed to create {}", skill_dir.display()))?;
        let skill_path = skill_dir.join("SKILL.md");
        std::fs::write(&skill_path, &output)
            .with_context(|| format!("Failed to write {}", skill_path.display()))?;
        println!(
            "{} {}",
            "Installed".green().bold(),
            skill_path.display()
        );
    } else {
        print!("{output}");
    }

    Ok(())
}

fn cmd_check(path: &str, file: &PathBuf) -> Result<()> {
    let base_dir = file
        .parent()
        .map(|p| {
            if p.as_os_str().is_empty() {
                std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
            } else {
                p.to_path_buf()
            }
        })
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let content = std::fs::read_to_string(file)
        .with_context(|| format!("Failed to read {}", file.display()))?;

    let brief = parse_brief(&content).context("Failed to parse briefing")?;
    let result = check_path(&brief, path, &base_dir);

    if result.is_sacred {
        eprintln!(
            "{} {} is in sacred region `{}`",
            "✗".red().bold(),
            path,
            result.matching_pattern.as_deref().unwrap_or("unknown")
        );
        if let Some(reason) = &result.reason {
            eprintln!("  {reason}");
        }
        process::exit(1);
    } else {
        println!("{} {} is not in a sacred region", "✓".green().bold(), path);
    }

    Ok(())
}

fn cmd_diff(file1: &PathBuf, file2: &PathBuf) -> Result<()> {
    let content1 = std::fs::read_to_string(file1)
        .with_context(|| format!("Failed to read {}", file1.display()))?;
    let content2 = std::fs::read_to_string(file2)
        .with_context(|| format!("Failed to read {}", file2.display()))?;

    let brief1 = parse_brief(&content1).context("Failed to parse first briefing")?;
    let brief2 = parse_brief(&content2).context("Failed to parse second briefing")?;

    let mut has_diff = false;

    // Goal
    if brief1.goal != brief2.goal {
        has_diff = true;
        println!("{}", "Goal changed:".cyan().bold());
        println!("  {} {}", "-".red(), brief1.goal);
        println!("  {} {}", "+".green(), brief2.goal);
        println!();
    }

    // Stack
    if brief1.frontmatter.stack != brief2.frontmatter.stack {
        has_diff = true;
        println!("{}", "Stack changed:".cyan().bold());
        diff_lists(&brief1.frontmatter.stack, &brief2.frontmatter.stack);
        println!();
    }

    // Hard constraints
    if brief1.constraints.hard != brief2.constraints.hard {
        has_diff = true;
        println!("{}", "Hard constraints changed:".cyan().bold());
        diff_lists(&brief1.constraints.hard, &brief2.constraints.hard);
        println!();
    }

    // Soft constraints
    if brief1.constraints.soft != brief2.constraints.soft {
        has_diff = true;
        println!("{}", "Soft constraints changed:".cyan().bold());
        diff_lists(&brief1.constraints.soft, &brief2.constraints.soft);
        println!();
    }

    // Ask first constraints
    if brief1.constraints.ask_first != brief2.constraints.ask_first {
        has_diff = true;
        println!("{}", "Ask-first constraints changed:".cyan().bold());
        diff_lists(&brief1.constraints.ask_first, &brief2.constraints.ask_first);
        println!();
    }

    // Sacred regions
    let sacred1: Vec<_> = brief1.sacred.iter().map(|s| &s.path).collect();
    let sacred2: Vec<_> = brief2.sacred.iter().map(|s| &s.path).collect();
    if sacred1 != sacred2 {
        has_diff = true;
        println!("{}", "Sacred regions changed:".cyan().bold());
        diff_lists(
            &brief1
                .sacred
                .iter()
                .map(|s| format!("`{}` — {}", s.path, s.reason))
                .collect::<Vec<_>>(),
            &brief2
                .sacred
                .iter()
                .map(|s| format!("`{}` — {}", s.path, s.reason))
                .collect::<Vec<_>>(),
        );
        println!();
    }

    // Assumptions
    let assumptions1: Vec<_> = brief1
        .assumptions
        .iter()
        .map(|a| {
            let m = if a.validated { "[x]" } else { "[ ]" };
            format!("{m} {}", a.text)
        })
        .collect();
    let assumptions2: Vec<_> = brief2
        .assumptions
        .iter()
        .map(|a| {
            let m = if a.validated { "[x]" } else { "[ ]" };
            format!("{m} {}", a.text)
        })
        .collect();
    if assumptions1 != assumptions2 {
        has_diff = true;
        println!("{}", "Assumptions changed:".cyan().bold());
        diff_lists(&assumptions1, &assumptions2);
        println!();
    }

    // Deliverable
    if brief1.deliverable != brief2.deliverable {
        has_diff = true;
        println!("{}", "Deliverable changed:".cyan().bold());
        if let Some(d) = &brief1.deliverable {
            println!("  {} {}", "-".red(), d);
        }
        if let Some(d) = &brief2.deliverable {
            println!("  {} {}", "+".green(), d);
        }
        println!();
    }

    if !has_diff {
        println!("No semantic differences found.");
    }

    Ok(())
}

fn diff_lists(old: &[impl AsRef<str>], new: &[impl AsRef<str>]) {
    for item in old.iter() {
        if !new.iter().any(|n| n.as_ref() == item.as_ref()) {
            println!("  {} {}", "-".red(), item.as_ref());
        }
    }
    for item in new.iter() {
        if !old.iter().any(|o| o.as_ref() == item.as_ref()) {
            println!("  {} {}", "+".green(), item.as_ref());
        }
    }
}
