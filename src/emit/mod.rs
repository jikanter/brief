pub mod agents_md;
pub mod claude;
pub mod cursor;
pub mod json;
pub mod markers;
pub mod prompt;
pub mod skill;
pub mod xml;

pub use agents_md::{emit_agents_md, install_agents_md};
pub use claude::{emit_claude, install_claude};
pub use cursor::{emit_cursor, install_cursor};
pub use json::emit_json;
pub use prompt::emit_prompt;
pub use skill::{emit_skill, relative_path, skill_name};
pub use xml::emit_xml;
