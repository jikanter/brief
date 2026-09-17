use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The top-level briefing structure parsed from a `.brief.md` file.
#[derive(Debug, Clone, Serialize)]
pub struct Brief {
    pub frontmatter: Frontmatter,
    pub goal: String,
    pub identity: Option<Identity>,
    pub constraints: Constraints,
    pub sacred: Vec<SacredEntry>,
    pub assumptions: Vec<Assumption>,
    pub deliverable: Option<String>,
    pub unknown_sections: Vec<UnknownSection>,
}

/// YAML frontmatter containing machine-critical structured data.
///
/// This is the authoring shape of the `---` block at the top of a `.brief.md`.
/// Every key is optional at parse time — the parser is tolerant, and validity
/// (non-empty `stack`, a known `brief_version`) is `brief validate`'s job.
/// Unrecognized keys are ignored, which keeps older tools reading newer briefs.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(
    extend("$id" = FRONTMATTER_SCHEMA_ID),
    extend("additionalProperties" = true)
)]
pub struct Frontmatter {
    /// Technologies, languages, frameworks. Required and non-empty on a valid brief.
    #[serde(default)]
    pub stack: Vec<String>,
    /// File paths or URLs of reference material. Optional.
    #[serde(default)]
    pub context: Vec<String>,
    /// Preferred model identifier for the task. Not a session or runtime config.
    #[serde(default)]
    pub model: Option<String>,
    /// `.brief.md` format version. Authored as `brief_version:`; the legacy
    /// `version:` spelling still deserializes (it collided with the project
    /// version `brief init` writes under `metadata:`).
    #[serde(default = "default_brief_version", alias = "version")]
    pub brief_version: String,
    /// kebab-case name for an emitted Agent Skill.
    #[serde(default)]
    pub skill_name: Option<String>,
    /// One-line description for an emitted Agent Skill.
    #[serde(default)]
    pub skill_description: Option<String>,
}

/// `$id` of the derived frontmatter schema — the raw URL of the committed copy.
pub const FRONTMATTER_SCHEMA_ID: &str = "https://raw.githubusercontent.com/jikanter/brief/main/docs/schema/brief-frontmatter-v1.schema.json";

/// The only format version this build understands.
pub const SUPPORTED_BRIEF_VERSION: &str = "1";

fn default_brief_version() -> String {
    SUPPORTED_BRIEF_VERSION.to_string()
}

impl Default for Frontmatter {
    fn default() -> Self {
        Self {
            stack: Vec::new(),
            context: Vec::new(),
            model: None,
            brief_version: default_brief_version(),
            skill_name: None,
            skill_description: None,
        }
    }
}

/// A single constraint: its text plus an optional path scope.
///
/// An unscoped constraint applies project-wide (the historical model). A scoped
/// constraint applies only when working in files matching one of its globs —
/// the text-layer foundation for emitting to glob-frontmatter targets
/// (`.claude/rules`, Cursor `globs`, Copilot `applyTo`, Windsurf `trigger: glob`).
/// See docs/open-questions.md "Scoped Constraints".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Constraint {
    pub text: String,
    /// Glob patterns the constraint is scoped to. Empty = project-wide.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scope: Vec<String>,
}

impl Constraint {
    /// An unscoped (project-wide) constraint.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            scope: Vec::new(),
        }
    }

    /// A constraint scoped to one or more glob patterns.
    pub fn scoped(text: impl Into<String>, scope: Vec<String>) -> Self {
        Self {
            text: text.into(),
            scope,
        }
    }

    /// True when the constraint carries at least one glob scope.
    pub fn is_scoped(&self) -> bool {
        !self.scope.is_empty()
    }

    /// True when `glob` is a clean directory prefix (e.g. `src/api/**` or
    /// `src/api/`) rather than an arbitrary glob (`**/*.test.ts`). A directory
    /// prefix has a native home in *both* the glob-frontmatter axis and the
    /// directory-hierarchy axis (nested CLAUDE.md/AGENTS.md); an arbitrary glob
    /// only lives in the glob-frontmatter axis. The emitter uses this to pick
    /// hierarchy vs. glob-file vs. prose. See docs/open-questions.md.
    pub fn is_directory_prefix(glob: &str) -> bool {
        let core = glob.trim_end_matches("/**").trim_end_matches('/');
        !core.is_empty()
            && !core
                .chars()
                .any(|c| matches!(c, '*' | '?' | '[' | ']' | '{' | '}'))
    }
}

impl AsRef<str> for Constraint {
    fn as_ref(&self) -> &str {
        &self.text
    }
}

impl From<&str> for Constraint {
    fn from(s: &str) -> Self {
        Constraint::new(s)
    }
}

impl From<String> for Constraint {
    fn from(s: String) -> Self {
        Constraint::new(s)
    }
}

/// Display renders the bare constraint text (scope is an emit-layer concern,
/// handled explicitly by each emitter).
impl std::fmt::Display for Constraint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.text)
    }
}

/// Deref to the constraint text so functions taking `&str` (framing, lints)
/// accept `&Constraint` via coercion without churn.
impl std::ops::Deref for Constraint {
    type Target = str;
    fn deref(&self) -> &str {
        &self.text
    }
}

/// Equality against string literals compares the text only — keeps existing
/// `assert_eq!(constraints.hard, vec!["..."])` style tests working.
impl PartialEq<&str> for Constraint {
    fn eq(&self, other: &&str) -> bool {
        self.text == *other
    }
}

impl PartialEq<str> for Constraint {
    fn eq(&self, other: &str) -> bool {
        self.text == other
    }
}

impl PartialEq<String> for Constraint {
    fn eq(&self, other: &String) -> bool {
        &self.text == other
    }
}

/// Constraints grouped by type.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Constraints {
    pub hard: Vec<Constraint>,
    pub soft: Vec<Constraint>,
    pub ask_first: Vec<Constraint>,
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
/// A section for project identity
#[derive(Debug, Clone, Serialize)]
pub struct Identity {
    pub heading: String,
    pub content: String,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unscoped_constraint_from_str() {
        let c: Constraint = "Must pass CI".into();
        assert!(!c.is_scoped());
        assert_eq!(c, "Must pass CI");
        assert_eq!(c.to_string(), "Must pass CI");
    }

    #[test]
    fn scoped_constraint_carries_globs() {
        let c = Constraint::scoped("WCAG 2.1 AA", vec!["src/ui/**".into()]);
        assert!(c.is_scoped());
        assert_eq!(c.scope, vec!["src/ui/**".to_string()]);
        // Display is still the bare text — scope is an emit concern.
        assert_eq!(c.to_string(), "WCAG 2.1 AA");
    }

    #[test]
    fn deref_exposes_text_as_str() {
        let c = Constraint::scoped("No `unsafe`", vec!["src/**".into()]);
        // Deref coercion: &Constraint usable where &str is expected.
        fn takes_str(s: &str) -> usize {
            s.len()
        }
        assert_eq!(takes_str(&c), "No `unsafe`".len());
    }

    #[test]
    fn directory_prefix_detection() {
        assert!(Constraint::is_directory_prefix("src/api/**"));
        assert!(Constraint::is_directory_prefix("src/api/"));
        assert!(Constraint::is_directory_prefix("src/api"));
        assert!(!Constraint::is_directory_prefix("**/*.test.ts"));
        assert!(!Constraint::is_directory_prefix("src/**/*.rs"));
        assert!(!Constraint::is_directory_prefix(""));
        assert!(!Constraint::is_directory_prefix("**"));
    }

    #[test]
    fn unscoped_scope_is_omitted_from_json() {
        let c = Constraint::new("Must pass CI");
        let json = serde_json::to_string(&c).unwrap();
        assert!(
            !json.contains("scope"),
            "unscoped should omit scope: {json}"
        );
    }

    #[test]
    fn scoped_scope_serializes() {
        let c = Constraint::scoped("WCAG", vec!["src/ui/**".into()]);
        let json = serde_json::to_string(&c).unwrap();
        assert!(json.contains("scope"));
        assert!(json.contains("src/ui/**"));
    }
}
