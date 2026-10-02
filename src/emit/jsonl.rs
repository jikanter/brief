//! One JSON line per brief: the id and the objective it was computed over.
//!
//! This is the join table for training data. aichat's trajectories carry only
//! a `brief_id`; a consumer (aichat's label table, rocky) appends these lines
//! into a stream and joins on `id` to recover the goal, the deliverable and
//! the constraints the work ran under.
//!
//! The line is the canonical JSON of the objective plus `id`, so dropping `id`
//! from a parsed line and writing it back in canonical form gives the exact
//! bytes the id hashes (see [`crate::id`]).

use crate::id::{brief_id, canonical_json, objective};
use crate::model::Brief;
use serde_json::Value;

/// Emit the brief as one newline-terminated JSON line.
pub fn emit_jsonl(brief: &Brief) -> String {
    let mut line = objective(brief);
    if let Value::Object(fields) = &mut line {
        fields.insert("id".to_string(), Value::String(brief_id(brief)));
    }
    let mut out = canonical_json(&line);
    out.push('\n');
    out
}
