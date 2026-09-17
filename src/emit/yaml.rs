//! Writing a YAML scalar that a YAML parser will read back.
//!
//! Every target whose output opens with frontmatter needs this, and for a while
//! only Cursor had it: `brief skill emit` wrote the goal into `description:`
//! unquoted, so a goal carrying a colon -- which an English sentence usually
//! does -- produced a SKILL.md whose frontmatter would not parse.

/// Quote a YAML scalar if it contains characters that would confuse the parser.
pub fn yaml_scalar(s: &str) -> String {
    let needs_quoting = s.chars().any(|c| {
        matches!(
            c,
            ':' | '#'
                | '['
                | ']'
                | '{'
                | '}'
                | ','
                | '&'
                | '*'
                | '!'
                | '|'
                | '>'
                | '\''
                | '"'
                | '%'
                | '@'
                | '`'
                | '\n'
        )
    });
    if needs_quoting {
        // Double-quoted form: escape backslashes and double quotes.
        let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
        format!("\"{escaped}\"")
    } else {
        s.to_string()
    }
}
