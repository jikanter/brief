//! Document provenance: the vocabulary specified in
//! `docs/design/provenance-schema.md`.
//!
//! This module is pure. It takes the text of any Markdown file and reports what
//! provenance keys it carries. It never touches the filesystem, runs git, or
//! decides whether a value is a problem — `brief validate` does that.
//!
//! Two rules shape the whole module:
//!
//! - **These are not `Frontmatter` fields.** They describe the document, not the
//!   task, so they stay out of the Rust `Frontmatter` type, out of the derived
//!   schema, and out of canonical JSON. The brief parser ignores them; this pass
//!   reads them.
//! - **Other people's files must never blow up.** Missing, malformed, or foreign
//!   frontmatter resolves to "nothing found", silently. Nothing here produces an
//!   error.

use serde_json::{Map, Value};

/// The flat `metadata:` keys brief owns. Everything else under `metadata` is
/// read-only passthrough.
const METADATA_PREFIX: &str = "brief.";
const METADATA_SOURCE: &str = "brief.source";
const METADATA_DATE_MODIFIED: &str = "brief.dateModified";

/// Canonical name first, then its registered aliases in resolution order.
/// A closed list: a spelling not here is a foreign key, conserved in silence.
const DATE_MODIFIED_KEYS: &[&str] = &[
    "dateModified",
    "date_modified",
    "last_modified",
    "lastmod",
    "modified",
    "updated",
    "updated_at",
];
const DATE_CREATED_KEYS: &[&str] = &["dateCreated", "date_created", "created", "created_at"];
const IS_BASED_ON_KEYS: &[&str] = &["isBasedOn", "is_based_on"];
const SUPERSEDED_BY_KEYS: &[&str] = &["superseded_by", "supersededBy"];

/// A pointer to another document: a path relative to the carrying document's
/// directory, or a URL that brief recognizes but never fetches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reference {
    Path(String),
    Url(String),
}

impl Reference {
    fn classify(raw: &str) -> Self {
        if is_url(raw) {
            Reference::Url(raw.to_string())
        } else {
            Reference::Path(raw.to_string())
        }
    }

    /// The path, when this is one. A URL gets no filesystem check.
    pub fn as_path(&self) -> Option<&str> {
        match self {
            Reference::Path(p) => Some(p),
            Reference::Url(_) => None,
        }
    }
}

/// A scheme-prefixed value is a URL: `https://…`, `ace://prefix/id`, anything
/// else shaped like `scheme://rest`. Brief checks the shape only; it does not
/// know any particular scheme's registry.
fn is_url(raw: &str) -> bool {
    let Some((scheme, rest)) = raw.split_once("://") else {
        return false;
    };
    !rest.is_empty()
        && !scheme.is_empty()
        && scheme.starts_with(|c: char| c.is_ascii_lowercase())
        && scheme
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '+' | '.' | '-'))
}

/// What a document's frontmatter says about itself.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Provenance {
    pub date_modified: Option<String>,
    pub date_created: Option<String>,
    pub is_based_on: Option<Reference>,
    pub superseded_by: Option<Reference>,
    /// True when the file carries any `metadata.brief.*` key — brief's own
    /// ownership marker, the same one `brief skill` uses.
    pub brief_written: bool,
    /// `dateModified` resolved, but not as a full timestamp with an offset.
    /// Only a complaint for a file brief wrote; on anyone else's it is normal.
    pub modified_needs_offset: bool,
    /// Human-readable notes about two addresses resolving to different values.
    pub disagreements: Vec<String>,
}

impl Provenance {
    /// The calendar day of `dateModified`, as the author wrote it. Never
    /// converted between zones: a day is a date, so every machine reads the
    /// same file the same way.
    pub fn modified_day(&self) -> Option<String> {
        self.date_modified.as_deref().and_then(calendar_day)
    }

    /// The calendar day of `dateCreated`.
    pub fn created_day(&self) -> Option<String> {
        self.date_created.as_deref().and_then(calendar_day)
    }
}

/// `YYYY-MM-DD`, if the value starts with one.
fn calendar_day(raw: &str) -> Option<String> {
    let bytes = raw.as_bytes();
    if bytes.len() < 10 {
        return None;
    }
    let shaped = bytes[..10].iter().enumerate().all(|(i, b)| match i {
        4 | 7 => *b == b'-',
        _ => b.is_ascii_digit(),
    });
    // A longer value must break cleanly after the date, so `2026-04-1142` is
    // not read as a day.
    let ends_cleanly = bytes.len() == 10 || matches!(bytes[10], b'T' | b't' | b' ');
    (shaped && ends_cleanly).then(|| raw[..10].to_string())
}

/// Read the provenance a document declares about itself.
///
/// `content` is the whole file. Anything unreadable — no frontmatter, an
/// unclosed fence, YAML that does not parse — resolves to an empty result.
pub fn resolve(content: &str) -> Provenance {
    let Some(map) = frontmatter_map(content) else {
        return Provenance::default();
    };

    let metadata = map.get("metadata").and_then(Value::as_object);
    let brief_written = metadata.is_some_and(|m| {
        m.keys()
            .any(|k| k.starts_with(METADATA_PREFIX) && k.len() > METADATA_PREFIX.len())
    });

    let mut disagreements = Vec::new();

    let date_modified = resolve_key(
        &map,
        DATE_MODIFIED_KEYS,
        metadata.and_then(|m| string_at(m, METADATA_DATE_MODIFIED)),
        METADATA_DATE_MODIFIED,
        &mut disagreements,
    );
    let date_created = resolve_key(&map, DATE_CREATED_KEYS, None, "", &mut disagreements);
    let is_based_on = resolve_key(
        &map,
        IS_BASED_ON_KEYS,
        metadata.and_then(|m| string_at(m, METADATA_SOURCE)),
        METADATA_SOURCE,
        &mut disagreements,
    );
    let superseded_by = resolve_key(&map, SUPERSEDED_BY_KEYS, None, "", &mut disagreements);

    let modified_needs_offset = brief_written
        && date_modified
            .as_deref()
            .is_some_and(|v| !has_explicit_offset(v));

    Provenance {
        date_modified,
        date_created,
        is_based_on: is_based_on.as_deref().map(Reference::classify),
        superseded_by: superseded_by.as_deref().map(Reference::classify),
        brief_written,
        modified_needs_offset,
        disagreements,
    }
}

/// A full ISO 8601 timestamp carrying a zone: `…Z` or `…±HH:MM`.
fn has_explicit_offset(raw: &str) -> bool {
    let Some(time) = raw.split_once(['T', 't']).map(|(_, t)| t) else {
        return false;
    };
    time.ends_with('Z')
        || time.ends_with('z')
        || time
            .rsplit_once(['+', '-'])
            .is_some_and(|(_, off)| off.len() == 5 && off.contains(':') || off.len() == 4)
}

/// Resolve one logical field: canonical name, then aliases in order, then the
/// `metadata.brief.*` fall-through. Every address that resolves to a different
/// value is reported; the winner is the first one found.
fn resolve_key(
    map: &Map<String, Value>,
    keys: &[&str],
    namespaced: Option<&str>,
    namespaced_name: &str,
    disagreements: &mut Vec<String>,
) -> Option<String> {
    let mut winner: Option<(&str, String)> = None;

    for key in keys {
        let Some(value) = string_at(map, key) else {
            continue;
        };
        match &winner {
            None => winner = Some((key, value.to_string())),
            Some((won_key, won_value)) if won_value != value => {
                disagreements.push(format!(
                    "`{won_key}` says {won_value}, `{key}` says {value}; `{won_key}` wins"
                ));
            }
            Some(_) => {}
        }
    }

    if let Some(value) = namespaced {
        match &winner {
            None => winner = Some((namespaced_name, value.to_string())),
            Some((won_key, won_value)) if won_value != value => {
                disagreements.push(format!(
                    "`{won_key}` says {won_value}, `metadata.{namespaced_name}` says {value}; \
                     top-level wins"
                ));
            }
            Some(_) => {}
        }
    }

    winner.map(|(_, v)| v)
}

/// A string value at `key`. Anything else — a list, a map, a number, a null —
/// is not a provenance value and resolves as absent.
fn string_at<'a>(map: &'a Map<String, Value>, key: &str) -> Option<&'a str> {
    map.get(key).and_then(Value::as_str)
}

/// The frontmatter block as a map, or `None` when there is not a readable one.
fn frontmatter_map(content: &str) -> Option<Map<String, Value>> {
    let trimmed = content.trim_start();
    let after_opening = trimmed.strip_prefix("---")?;
    let after_opening = after_opening.strip_prefix('\n').unwrap_or(after_opening);
    let end = after_opening.find("\n---")?;

    match serde_saphyr::from_str::<Value>(&after_opening[..end]) {
        Ok(Value::Object(map)) => Some(map),
        _ => None,
    }
}
