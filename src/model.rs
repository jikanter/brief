use serde::{Deserialize, Serialize};

/// The top-level briefing structure parsed from a `.brief.md` file.
#[derive(Debug, Clone, Serialize)]
pub struct Brief {
    pub frontmatter: Frontmatter,
    pub goal: String,
    pub constraints: Constraints,
    pub sacred: Vec<SacredEntry>,
    pub assumptions: Vec<Assumption>,
    pub deliverable: Option<String>,
    pub unknown_sections: Vec<UnknownSection>,
}

/// YAML frontmatter containing machine-critical structured data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frontmatter {
    #[serde(default)]
    pub stack: Vec<String>,
    #[serde(default)]
    pub context: Vec<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(default)]
    pub skill_name: Option<String>,
    #[serde(default)]
    pub skill_description: Option<String>,
}

fn default_version() -> String {
    "1".to_string()
}

impl Default for Frontmatter {
    fn default() -> Self {
        Self {
            stack: Vec::new(),
            context: Vec::new(),
            model: None,
            version: default_version(),
            skill_name: None,
            skill_description: None,
        }
    }
}

/// Constraints grouped by type.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Constraints {
    pub hard: Vec<String>,
    pub soft: Vec<String>,
    pub ask_first: Vec<String>,
}

/// A sacred region entry: a path glob and reason it must not be modified.
#[derive(Debug, Clone, Serialize)]
pub struct SacredEntry {
    pub path: String,
    pub reason: String,
    /// Whether the entry was properly formatted (backtick-wrapped path + separator).
    pub well_formed: bool,
}

/// An assumption with validation state.
#[derive(Debug, Clone, Serialize)]
pub struct Assumption {
    pub text: String,
    pub validated: bool,
    /// Whether the original entry had checkbox syntax.
    pub has_checkbox: bool,
}

/// A section with an unrecognized H2 heading, preserved for extensibility.
#[derive(Debug, Clone, Serialize)]
pub struct UnknownSection {
    pub heading: String,
    pub content: String,
}

/// Severity of a validation diagnostic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum Severity {
    Error,
    Warning,
}

/// A single validation finding.
#[derive(Debug, Clone, Serialize)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
}
