pub mod body;
pub mod frontmatter;

use anyhow::{Context, Result};

use crate::model::Brief;

/// Parse a `.brief.md` file from its raw string content into a `Brief` struct.
pub fn parse_brief(input: &str) -> Result<Brief> {
    let (frontmatter, markdown) =
        frontmatter::extract_frontmatter(input).context("Failed to extract frontmatter")?;

    let body = body::parse_body(markdown);

    let goal = body.goal.unwrap_or_default();

    Ok(Brief {
        frontmatter,
        goal,
        constraints: body.constraints,
        sacred: body.sacred,
        assumptions: body.assumptions,
        deliverable: body.deliverable,
        unknown_sections: body.unknown_sections,
    })
}
