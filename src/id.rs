//! `brief.id`: a content hash of the brief's objective.
//!
//! A consumer that runs work under a brief needs a key to join that work back
//! to what was asked. aichat records the id on a session and exports it with
//! a trajectory, so the id has to be reproducible from bytes alone, by code
//! that never links this crate. The contract is docs/schema/SPEC.md §7:
//!
//! 1. The **objective** is three fields of the parsed brief, exactly as
//!    `brief emit json` prints them: `goal`, `deliverable`, `constraints`.
//! 2. Its **canonical JSON** has object keys sorted, no whitespace between
//!    tokens, and strings in UTF-8 with the minimal JSON escapes.
//! 3. The id is `sha256:` plus the lowercase hex SHA-256 of those bytes.
//!
//! Everything else in a brief (frontmatter, Sacred, Assumptions, Identity,
//! unknown sections, the order sections are written in) is outside the
//! objective and never moves the id.

use crate::model::Brief;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

/// The prefix of every id, naming the hash.
pub const ID_PREFIX: &str = "sha256:";

/// The brief's objective: the three fields the id is computed over, in the
/// shape `brief emit json` gives them. All three are always present: a brief
/// without a `## Deliverable` has `null`, one without `## Constraints` has
/// three empty tiers, and one without an H1 has the empty goal the parser
/// stores (`brief validate` reports that brief as invalid).
pub fn objective(brief: &Brief) -> Value {
    json!({
        "goal": brief.goal,
        "deliverable": brief.deliverable,
        "constraints": brief.constraints,
    })
}

/// The exact bytes the id hashes: [`objective`] in canonical JSON.
pub fn objective_json(brief: &Brief) -> String {
    canonical_json(&objective(brief))
}

/// The brief's id: `sha256:` plus 64 lowercase hex digits.
pub fn brief_id(brief: &Brief) -> String {
    id_of_objective(&objective(brief))
}

/// The id of an objective given as JSON, whatever order its keys are in.
pub fn id_of_objective(objective: &Value) -> String {
    let digest = Sha256::digest(canonical_json(objective).as_bytes());
    let mut id = String::with_capacity(ID_PREFIX.len() + 64);
    id.push_str(ID_PREFIX);
    for byte in digest {
        id.push_str(&format!("{byte:02x}"));
    }
    id
}

/// Write `value` as canonical JSON: object keys sorted by code point, no
/// whitespace between tokens, strings as UTF-8 with only the escapes JSON
/// requires (`\"`, `\\`, the short forms `\b \f \n \r \t`, and lowercase
/// `\u00xx` for the other control characters).
///
/// Keys are sorted here rather than left to `serde_json`'s map, whose order
/// depends on a cargo feature (`preserve_order`) that another crate in the
/// build can switch on.
pub fn canonical_json(value: &Value) -> String {
    let mut out = String::new();
    write_canonical(value, &mut out);
    out
}

fn write_canonical(value: &Value, out: &mut String) {
    match value {
        Value::Object(map) => {
            let mut entries: Vec<(&String, &Value)> = map.iter().collect();
            entries.sort_by(|a, b| a.0.cmp(b.0));
            out.push('{');
            for (i, (key, item)) in entries.into_iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&scalar(&Value::String(key.clone())));
                out.push(':');
                write_canonical(item, out);
            }
            out.push('}');
        }
        Value::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_canonical(item, out);
            }
            out.push(']');
        }
        other => out.push_str(&scalar(other)),
    }
}

/// A string, number, boolean or null in `serde_json`'s compact form, which is
/// the canonical one for strings. The objective holds no numbers.
fn scalar(value: &Value) -> String {
    serde_json::to_string(value).expect("a JSON scalar always serializes")
}
