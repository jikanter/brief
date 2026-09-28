//! A host file that *mentions* the `<brief:generated>` marker, in prose or a
//! code block, must not be taken for a managed section. Found by dogfooding:
//! a project-structure comment in brief's own CLAUDE.md read
//! `# generates the <brief:generated> section`, and `brief emit claude
//! --install` replaced everything from that comment to the real closing
//! marker, deleting the hand-written sections in between.

use assert_cmd::prelude::*;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

const BRIEF: &str = "---\nstack: [Rust]\n---\n\n# Ship it\n\n## Constraints\n\n### Hard\n- Tests pass under `cargo test`\n";

#[test]
fn install_keeps_hand_written_content_around_marker_mentions() {
    let dir = tempdir().unwrap();
    let brief = dir.path().join(".brief.md");
    fs::write(&brief, BRIEF).unwrap();
    let claude_md = dir.path().join("CLAUDE.md");
    let host = "\
# Project\n\
\n\
```\n\
.brief.md   # generates the <brief:generated> section at the end\n\
```\n\
\n\
## Hand-written section\n\
Keep me.\n\
\n\
<brief:generated>\n\
old generated body\n\
</brief:generated>\n";
    fs::write(&claude_md, host).unwrap();

    Command::cargo_bin("brief")
        .unwrap()
        .current_dir(dir.path())
        .arg("--file")
        .arg(&brief)
        .args(["emit", "claude", "--install"])
        .assert()
        .success();

    let out = fs::read_to_string(&claude_md).unwrap();
    assert!(
        out.contains(".brief.md   # generates the <brief:generated> section at the end"),
        "the code-block mention was overwritten:\n{out}"
    );
    assert!(out.contains("## Hand-written section\nKeep me."), "{out}");
    assert!(!out.contains("old generated body"), "{out}");
    assert!(out.contains("Tests pass under `cargo test`"), "{out}");
}
