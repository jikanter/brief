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

mod skill_init;
mod skill_validate;

use crate::skill_init::init_skill;
use crate::skill_validate::{validate_skill, validate_skill_content};

#[derive(Parser)]
#[command(name = "brief", about = "Structured briefings for AI coding agents")]
#[command(version)]
struct Cli {
    /// Path to the .brief.md file
    #[arg(long, global = true, default_value = ".brief.md")]
    file: PathBuf,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Clone)]
enum Commands {
    /// Analyze the current directory and scaffold a .brief.md
    Init,

    /// Validate the current .brief.md against the codebase
    Validate,

    /// Transform .brief.md into a target format
    Emit {
        /// Output target format
        #[arg(value_enum)]
        target: EmitTarget,

        /// Install the briefing into the target location
        #[arg(long)]
        install: bool,
    },

    /// Check if a file path falls within a sacred region
    Check {
        /// The file path to check
        path: String,
    },

    /// Fail if any changed file falls within a sacred region (CI gate)
    ///
    /// Examples:
    ///   brief validate-diff --base origin/main
    ///   brief validate-diff --base origin/main --json
    ///   git diff --name-only origin/main | brief validate-diff --stdin
    #[command(verbatim_doc_comment)]
    ValidateDiff {
        /// Git ref to diff against (changed files = `git diff --name-only <base>..HEAD`)
        #[arg(long, default_value = "HEAD")]
        base: String,

        /// Read newline-separated changed file paths from stdin instead of running git
        #[arg(long)]
        stdin: bool,

        /// Emit a machine-readable JSON report
        #[arg(long)]
        json: bool,
    },

    /// Show semantic differences between two briefing files
    Diff {
        /// First briefing file
        file1: PathBuf,
        /// Second briefing file
        file2: PathBuf,
    },

    /// Manage agent skills
    ///
    /// Examples:
    ///   brief skill init [name]
    ///   brief skill validate <path>
    ///   brief skill emit [--install]
    #[command(verbatim_doc_comment)]
    Skill {
        #[command(subcommand)]
        command: SkillCommands,
    },
}

#[derive(Subcommand, Clone)]
#[command(arg_required_else_help = true)]
pub enum SkillCommands {
    /// Initialize an empty skill directory with a valid skeleton
    ///
    /// Examples:
    ///   brief skill init
    ///   brief skill init my-skill-name
    #[command(verbatim_doc_comment)]
    Init {
        /// Skill name (kebab-case, max 64 chars). Defaults to current directory name.
        name: Option<String>,
    },

    /// Validate a skill against the agentskills.io spec
    ///
    /// Examples:
    ///   brief skill validate <path>
    #[command(verbatim_doc_comment)]
    Validate {
        /// Path to the skill (directory or SKILL.md file)
        path: PathBuf,
    },

    /// Emit a skill from the current briefing file
    ///
    /// Examples:
    ///   brief skill emit
    ///   brief skill emit --install
    #[command(verbatim_doc_comment)]
    Emit {
        /// Install the skill to the agent's skills directory
        #[arg(long, short)]
        install: bool,
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
    /// Emit a Cursor `.cursor/rules/brief.mdc` rule
    Cursor,
    /// Emit structured JSON
    Json,
    /// Emit Anthropic-style XML tags for API system prompts
    Xml,
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
        Commands::Validate => cmd_validate(&cli.file),
        Commands::Emit { target, install } => cmd_emit(target, &cli.file, install),
        Commands::Check { path } => cmd_check(&path, &cli.file),
        Commands::ValidateDiff { base, stdin, json } => {
            cmd_validate_diff(&cli.file, &base, stdin, json)
        }
        Commands::Diff { file1, file2 } => cmd_diff(&file1, &file2),
        Commands::Skill { command } => match command {
            SkillCommands::Init { name } => cmd_skill_init(name),
            SkillCommands::Validate { path } => cmd_skill_validate(&path),
            SkillCommands::Emit { install } => cmd_skill_emit(&cli.file, install),
        },
    }
}

fn cmd_skill_emit(file: &PathBuf, install: bool) -> Result<()> {
    cmd_emit_skill_internal(file, install)
}

fn cmd_skill_init(name: Option<String>) -> Result<()> {
    init_skill(name)
}

fn cmd_skill_validate(path: &PathBuf) -> Result<()> {
    validate_skill(path)
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
        EmitTarget::Cursor => emit::emit_cursor(&brief),
        EmitTarget::Json => emit::emit_json(&brief),
        EmitTarget::Xml => emit::emit_xml(&brief),
    };

    if install {
        match target {
            EmitTarget::Claude => {
                let claude_md = PathBuf::from("CLAUDE.md");
                emit::install_claude(&brief, &claude_md)
                    .with_context(|| "Failed to install briefing into CLAUDE.md")?;
                println!(
                    "{} briefing into {}",
                    "Installed".green().bold(),
                    claude_md.display()
                );
            }
            EmitTarget::AgentsMd => {
                let agents_md = PathBuf::from("AGENTS.md");
                emit::install_agents_md(&brief, &agents_md)
                    .with_context(|| "Failed to install briefing into AGENTS.md")?;
                println!(
                    "{} briefing into {}",
                    "Installed".green().bold(),
                    agents_md.display()
                );
            }
            EmitTarget::Cursor => {
                let base = std::env::current_dir().context("Failed to get current directory")?;
                let written = emit::install_cursor(&brief, &base)
                    .with_context(|| "Failed to install briefing into .cursor/rules/brief.mdc")?;
                println!(
                    "{} briefing into {}",
                    "Installed".green().bold(),
                    written.display()
                );
            }
            _ => {
                anyhow::bail!(
                    "--install is only supported for the claude, agents-md, and cursor targets"
                );
            }
        }
    } else {
        print!("{output}");
    }

    Ok(())
}

fn cmd_emit_skill_internal(file: &PathBuf, install: bool) -> Result<()> {
    let content = std::fs::read_to_string(file)
        .with_context(|| format!("Failed to read {}", file.display()))?;

    let brief = parse_brief(&content).context("Failed to parse briefing")?;

    if install {
        let name = emit::skill_name(&brief);
        let skill_dir = PathBuf::from(".claude/skills").join(&name);
        std::fs::create_dir_all(&skill_dir)
            .with_context(|| format!("Failed to create {}", skill_dir.display()))?;

        let brief_canon = std::fs::canonicalize(file)
            .with_context(|| format!("Failed to canonicalize {}", file.display()))?;
        let dir_canon = std::fs::canonicalize(&skill_dir)
            .with_context(|| format!("Failed to canonicalize {}", skill_dir.display()))?;
        let source = emit::relative_path(&brief_canon, &dir_canon);

        let output = emit::emit_skill(&brief, Some(&source));
        validate_skill_content(&output)
            .context("Emitted SKILL.md would not pass agentskills.io validation")?;

        let skill_path = skill_dir.join("SKILL.md");
        std::fs::write(&skill_path, &output)
            .with_context(|| format!("Failed to write {}", skill_path.display()))?;
        println!("{} {}", "Installed".green().bold(), skill_path.display());
    } else {
        let output = emit::emit_skill(&brief, None);
        validate_skill_content(&output)
            .context("Emitted SKILL.md would not pass agentskills.io validation")?;
        print!("{output}");
    }

    Ok(())
}

fn cmd_check(path: &str, file: &PathBuf) -> Result<()> {
    let base_dir = brief_base_dir(file);

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

/// Resolve the directory a brief lives in, for interpreting relative sacred paths.
fn brief_base_dir(file: &std::path::Path) -> PathBuf {
    file.parent()
        .map(|p| {
            if p.as_os_str().is_empty() {
                std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
            } else {
                p.to_path_buf()
            }
        })
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

/// Changed files from `git diff --name-only <base>..HEAD`.
fn git_changed_files(base: &str) -> Result<Vec<String>> {
    let range = format!("{base}..HEAD");
    let output = process::Command::new("git")
        .args(["diff", "--name-only", &range])
        .output()
        .context("Failed to run git; is git installed and is this a repository?")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git diff failed: {}", stderr.trim());
    }

    Ok(parse_file_list(&String::from_utf8_lossy(&output.stdout)))
}

/// Parse a newline-separated list of file paths, dropping blanks.
fn parse_file_list(raw: &str) -> Vec<String> {
    raw.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect()
}

fn cmd_validate_diff(file: &PathBuf, base: &str, stdin: bool, json: bool) -> Result<()> {
    let base_dir = brief_base_dir(file);

    let content = std::fs::read_to_string(file)
        .with_context(|| format!("Failed to read {}", file.display()))?;
    let brief = parse_brief(&content).context("Failed to parse briefing")?;

    let files = if stdin {
        use std::io::Read;
        let mut buf = String::new();
        std::io::stdin()
            .read_to_string(&mut buf)
            .context("Failed to read changed files from stdin")?;
        parse_file_list(&buf)
    } else {
        git_changed_files(base)?
    };

    let report = brief_cli::validate_diff::check_changed_files(&brief, &files, &base_dir);

    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else if report.is_clean() {
        println!(
            "{} {} changed file(s) checked, none in a sacred region",
            "✓".green().bold(),
            report.checked
        );
    } else {
        eprintln!(
            "{} {} changed file(s) fall within sacred regions:",
            "✗".red().bold(),
            report.violations.len()
        );
        for v in &report.violations {
            eprintln!("  {} → `{}`: {}", v.file, v.pattern, v.reason);
        }
    }

    if !report.is_clean() {
        process::exit(1);
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
