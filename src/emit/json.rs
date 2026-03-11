use crate::model::Brief;

/// Emit the parsed briefing as structured JSON.
pub fn emit_json(brief: &Brief) -> String {
    serde_json::to_string_pretty(brief).expect("Brief serialization should never fail")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;

    #[test]
    fn json_is_valid_and_contains_fields() {
        let brief = Brief {
            frontmatter: Frontmatter {
                stack: vec!["Rust".into()],
                context: vec!["./README.md".into()],
                model: Some("claude-sonnet-4-20250514".into()),
                version: "1".into(),
                skill_name: None,
                skill_description: None,
            },
            goal: "Build feature".into(),
            constraints: Constraints {
                hard: vec!["No breakage".into()],
                soft: vec![],
                ask_first: vec![],
            },
            sacred: vec![SacredEntry {
                path: "src/auth/**".into(),
                reason: "Auth".into(),
                well_formed: true,
            }],
            assumptions: vec![Assumption {
                text: "It works".into(),
                validated: true,
                has_checkbox: true,
            }],
            deliverable: Some("Working code".into()),
            unknown_sections: vec![],
        };
        let json_str = emit_json(&brief);
        let value: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        assert_eq!(value["goal"], "Build feature");
        assert_eq!(value["frontmatter"]["stack"][0], "Rust");
        assert_eq!(value["constraints"]["hard"][0], "No breakage");
        assert_eq!(value["sacred"][0]["path"], "src/auth/**");
        assert_eq!(value["assumptions"][0]["validated"], true);
        assert_eq!(value["deliverable"], "Working code");
    }
}
