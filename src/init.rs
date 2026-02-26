use std::path::Path;

/// Analyze a directory and scaffold a `.brief.md` with sensible defaults.
pub fn scaffold_brief(dir: &Path) -> String {
    let stack = detect_stack(dir);
    let context = detect_context(dir);
    let sacred_candidates = detect_sacred_candidates(dir);

    let mut out = String::new();

    // Frontmatter
    out.push_str("---\n");
    if stack.is_empty() {
        out.push_str("stack: []\n");
    } else {
        out.push_str(&format!("stack: [{}]\n", stack.join(", ")));
    }
    if !context.is_empty() {
        out.push_str(&format!("context: [{}]\n", context.join(", ")));
    }
    out.push_str("---\n\n");

    // Goal placeholder
    out.push_str("# <Describe your goal here>\n\n");

    // Constraints
    out.push_str("## Constraints\n\n");
    out.push_str("### Hard\n");
    out.push_str("- <non-negotiable constraint>\n\n");
    out.push_str("### Soft\n");
    out.push_str("- <preferred but flexible constraint>\n\n");
    out.push_str("### Ask First\n");
    out.push_str("- <requires human approval before proceeding>\n\n");

    // Sacred
    out.push_str("## Sacred\n");
    if sacred_candidates.is_empty() {
        out.push_str("- `<path/to/protected/code>` — <reason>\n");
    } else {
        for (path, reason) in &sacred_candidates {
            out.push_str(&format!("- `{path}` — {reason}\n"));
        }
    }
    out.push('\n');

    // Assumptions
    out.push_str("## Assumptions\n");
    out.push_str("- [ ] <assumption to validate>\n\n");

    // Deliverable
    out.push_str("## Deliverable\n");
    out.push_str("<Describe what \"done\" looks like>\n");

    out
}

/// Detect the technology stack by looking for well-known config files.
fn detect_stack(dir: &Path) -> Vec<String> {
    let mut stack = Vec::new();

    let detectors: &[(&str, &str)] = &[
        ("Cargo.toml", "Rust"),
        ("go.mod", "Go"),
        ("Gemfile", "Ruby"),
        ("pom.xml", "Java"),
        ("build.gradle", "Java"),
        ("build.gradle.kts", "Kotlin"),
    ];

    for (file, tech) in detectors {
        if dir.join(file).exists() {
            stack.push(tech.to_string());
        }
    }

    // Python — check for version hint in pyproject.toml
    if dir.join("pyproject.toml").exists() {
        stack.push(detect_python_version(dir).unwrap_or_else(|| "Python".to_string()));
    } else if dir.join("requirements.txt").exists() {
        stack.push("Python".to_string());
    }

    // Node/TypeScript
    if dir.join("package.json").exists() {
        if dir.join("tsconfig.json").exists() {
            stack.push("TypeScript".to_string());
        } else {
            stack.push("Node.js".to_string());
        }
    }

    // Docker
    if dir.join("docker-compose.yml").exists() || dir.join("docker-compose.yaml").exists() {
        stack.push("Docker".to_string());
    }

    stack
}

fn detect_python_version(dir: &Path) -> Option<String> {
    let content = std::fs::read_to_string(dir.join("pyproject.toml")).ok()?;
    // Look for requires-python = ">=3.12" or similar
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("requires-python") {
            // Extract version number
            if let Some(version) = extract_version_number(trimmed) {
                return Some(format!("Python {version}"));
            }
        }
    }
    Some("Python".to_string())
}

fn extract_version_number(s: &str) -> Option<&str> {
    // Find the first digit sequence like "3.12"
    let start = s.find(|c: char| c.is_ascii_digit())?;
    let rest = &s[start..];
    let end = rest
        .find(|c: char| !c.is_ascii_digit() && c != '.')
        .unwrap_or(rest.len());
    let version = &rest[..end];
    if version.is_empty() {
        None
    } else {
        Some(version)
    }
}

/// Look for common context files (README, architecture docs).
fn detect_context(dir: &Path) -> Vec<String> {
    let candidates = [
        "README.md",
        "docs/architecture.md",
        "docs/ARCHITECTURE.md",
        "CONTRIBUTING.md",
        "docs/design.md",
    ];

    candidates
        .iter()
        .filter(|f| dir.join(f).exists())
        .map(|f| format!("./{f}"))
        .collect()
}

/// Detect directories commonly considered sacred.
fn detect_sacred_candidates(dir: &Path) -> Vec<(String, String)> {
    // (directory to check, glob pattern to emit, reason)
    let candidates: &[(&str, &str, &str)] = &[
        ("src/auth", "src/auth/**", "Authentication logic"),
        ("auth", "auth/**", "Authentication logic"),
        (
            "migrations",
            "migrations/**",
            "Database migrations — never alter historical files",
        ),
        (
            "src/migrations",
            "src/migrations/**",
            "Database migrations — never alter historical files",
        ),
    ];

    candidates
        .iter()
        .filter(|(check_dir, _, _)| dir.join(check_dir).is_dir())
        .map(|(_, pattern, reason)| (pattern.to_string(), reason.to_string()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn detect_rust_from_cargo_toml() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();
        let stack = detect_stack(tmp.path());
        assert!(stack.contains(&"Rust".to_string()));
    }

    #[test]
    fn detect_typescript_from_package_json_and_tsconfig() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("package.json"), "{}").unwrap();
        std::fs::write(tmp.path().join("tsconfig.json"), "{}").unwrap();
        let stack = detect_stack(tmp.path());
        assert!(stack.contains(&"TypeScript".to_string()));
    }

    #[test]
    fn detect_nodejs_from_package_json_only() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("package.json"), "{}").unwrap();
        let stack = detect_stack(tmp.path());
        assert!(stack.contains(&"Node.js".to_string()));
    }

    #[test]
    fn scaffold_has_required_sections() {
        let tmp = TempDir::new().unwrap();
        let output = scaffold_brief(tmp.path());
        assert!(output.contains("---\n"));
        assert!(output.contains("stack:"));
        assert!(output.contains("# <Describe your goal here>"));
        assert!(output.contains("## Constraints"));
        assert!(output.contains("### Hard"));
        assert!(output.contains("### Soft"));
        assert!(output.contains("### Ask First"));
        assert!(output.contains("## Sacred"));
        assert!(output.contains("## Assumptions"));
        assert!(output.contains("## Deliverable"));
    }

    #[test]
    fn scaffold_includes_detected_context() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("README.md"), "# Hello").unwrap();
        let output = scaffold_brief(tmp.path());
        assert!(output.contains("./README.md"));
    }

    #[test]
    fn scaffold_detects_sacred_auth_dir() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join("src/auth")).unwrap();
        std::fs::write(tmp.path().join("src/auth/handler.rs"), "").unwrap();
        let output = scaffold_brief(tmp.path());
        assert!(output.contains("src/auth/**"));
    }
}
