//! Claude Code hooks integration.
//!
//! Two concerns, both pure and unit-tested here so the CLI layer stays thin:
//!
//! 1. The PreToolUse I/O protocol for `brief check --hook` — parse the event
//!    JSON Claude Code sends on stdin, and build the deny decision it expects on
//!    stdout.
//! 2. Idempotently registering the PreToolUse hook in `.claude/settings.json`
//!    for `brief emit claude --install --hooks`.

use std::path::Path;

use anyhow::{Context, Result};
use serde_json::{Value, json};

/// Tool matcher for the sacred-region hook.
pub const HOOK_MATCHER: &str = "Edit|Write";
/// Command the hook runs.
pub const HOOK_COMMAND: &str = "brief check --hook";

/// Extract the target file path from a PreToolUse hook event JSON.
///
/// Returns `None` when the event has no `tool_input.file_path` (e.g. a tool that
/// doesn't touch a file) — the caller should treat that as "nothing to guard".
pub fn extract_file_path(event_json: &str) -> Option<String> {
    let event: Value = serde_json::from_str(event_json).ok()?;
    event
        .get("tool_input")?
        .get("file_path")?
        .as_str()
        .map(str::to_string)
}

/// Make a (possibly absolute) hook file path relative to the brief's base dir,
/// so it can be matched against the relative sacred patterns. If the path is not
/// under `base_dir`, it is returned unchanged.
pub fn relativize(file_path: &str, base_dir: &Path) -> String {
    let p = Path::new(file_path);
    if let Ok(stripped) = p.strip_prefix(base_dir) {
        return stripped.to_string_lossy().to_string();
    }
    file_path.to_string()
}

/// Build the PreToolUse stdout payload that denies a tool call.
pub fn deny_json(reason: &str) -> String {
    let payload = json!({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": "deny",
            "permissionDecisionReason": reason,
        }
    });
    payload.to_string()
}

/// Return a `.claude/settings.json` string with the sacred-region PreToolUse
/// hook registered. Idempotent: if a matching hook already exists, the input is
/// returned semantically unchanged (re-serialized). Other settings are preserved.
///
/// `existing` is the current file contents, or `None` if the file doesn't exist.
pub fn ensure_pretooluse_hook(existing: Option<&str>) -> Result<String> {
    let mut root: Value = match existing {
        Some(s) if !s.trim().is_empty() => {
            serde_json::from_str(s).context("Existing .claude/settings.json is not valid JSON")?
        }
        _ => json!({}),
    };

    if !root.is_object() {
        anyhow::bail!("Existing .claude/settings.json is not a JSON object");
    }

    let obj = root.as_object_mut().unwrap();
    let hooks = obj
        .entry("hooks")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .context("`hooks` in settings.json is not an object")?;

    let pre = hooks
        .entry("PreToolUse")
        .or_insert_with(|| json!([]))
        .as_array_mut()
        .context("`hooks.PreToolUse` in settings.json is not an array")?;

    if pre.iter().any(matcher_has_brief_hook) {
        // Already present — leave as-is.
        return serde_json::to_string_pretty(&root).map_err(Into::into);
    }

    pre.push(json!({
        "matcher": HOOK_MATCHER,
        "hooks": [
            { "type": "command", "command": HOOK_COMMAND }
        ]
    }));

    serde_json::to_string_pretty(&root).map_err(Into::into)
}

/// Return `.claude/settings.json` with the brief sacred-region PreToolUse hook
/// removed (for `--uninstall`). Empty `PreToolUse` / `hooks` containers left
/// behind are pruned. Other settings are preserved. Idempotent: removing when
/// absent is a no-op re-serialize.
pub fn remove_pretooluse_hook(existing: &str) -> Result<String> {
    let mut root: Value = if existing.trim().is_empty() {
        json!({})
    } else {
        serde_json::from_str(existing)
            .context("Existing .claude/settings.json is not valid JSON")?
    };
    let Some(obj) = root.as_object_mut() else {
        anyhow::bail!("Existing .claude/settings.json is not a JSON object");
    };

    if let Some(hooks) = obj.get_mut("hooks").and_then(Value::as_object_mut) {
        if let Some(pre) = hooks.get_mut("PreToolUse").and_then(Value::as_array_mut) {
            pre.retain(|entry| !matcher_has_brief_hook(entry));
            if pre.is_empty() {
                hooks.remove("PreToolUse");
            }
        }
        if hooks.is_empty() {
            obj.remove("hooks");
        }
    }

    serde_json::to_string_pretty(&root).map_err(Into::into)
}

/// Return `.claude/settings.json` with each entry in `entries` present in
/// `permissions.allow` (deduplicated, order-preserving). Used by
/// `--install --full` to pre-allow the project's known-safe `## Commands`.
pub fn ensure_permissions_allow(existing: Option<&str>, entries: &[String]) -> Result<String> {
    let mut root: Value = match existing {
        Some(s) if !s.trim().is_empty() => {
            serde_json::from_str(s).context("Existing .claude/settings.json is not valid JSON")?
        }
        _ => json!({}),
    };
    if !root.is_object() {
        anyhow::bail!("Existing .claude/settings.json is not a JSON object");
    }
    let obj = root.as_object_mut().unwrap();

    let allow = obj
        .entry("permissions")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .context("`permissions` in settings.json is not an object")?
        .entry("allow")
        .or_insert_with(|| json!([]))
        .as_array_mut()
        .context("`permissions.allow` in settings.json is not an array")?;

    for entry in entries {
        let present = allow.iter().any(|v| v.as_str() == Some(entry.as_str()));
        if !present {
            allow.push(Value::String(entry.clone()));
        }
    }

    serde_json::to_string_pretty(&root).map_err(Into::into)
}

/// True when a PreToolUse matcher entry already runs the brief sacred-region hook.
fn matcher_has_brief_hook(entry: &Value) -> bool {
    entry
        .get("hooks")
        .and_then(Value::as_array)
        .map(|inner| {
            inner.iter().any(|h| {
                h.get("command")
                    .and_then(Value::as_str)
                    .is_some_and(|c| c.contains(HOOK_COMMAND))
            })
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_file_path_from_edit_event() {
        let ev = r#"{"hook_event_name":"PreToolUse","tool_name":"Edit","tool_input":{"file_path":"src/auth/h.rs"}}"#;
        assert_eq!(extract_file_path(ev).as_deref(), Some("src/auth/h.rs"));
    }

    #[test]
    fn missing_file_path_returns_none() {
        let ev = r#"{"tool_name":"Bash","tool_input":{"command":"ls"}}"#;
        assert_eq!(extract_file_path(ev), None);
        assert_eq!(extract_file_path("not json"), None);
    }

    #[test]
    fn relativize_strips_base_dir() {
        let base = Path::new("/repo");
        assert_eq!(relativize("/repo/src/auth/h.rs", base), "src/auth/h.rs");
        // Outside the base dir: returned unchanged.
        assert_eq!(relativize("src/auth/h.rs", base), "src/auth/h.rs");
        assert_eq!(relativize("/other/x.rs", base), "/other/x.rs");
    }

    #[test]
    fn deny_json_has_exact_protocol_fields() {
        let out = deny_json("sacred: auth");
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["hookSpecificOutput"]["hookEventName"], "PreToolUse");
        assert_eq!(v["hookSpecificOutput"]["permissionDecision"], "deny");
        assert_eq!(
            v["hookSpecificOutput"]["permissionDecisionReason"],
            "sacred: auth"
        );
    }

    #[test]
    fn ensure_hook_adds_to_empty() {
        let out = ensure_pretooluse_hook(None).unwrap();
        let v: Value = serde_json::from_str(&out).unwrap();
        let entry = &v["hooks"]["PreToolUse"][0];
        assert_eq!(entry["matcher"], "Edit|Write");
        assert_eq!(entry["hooks"][0]["type"], "command");
        assert_eq!(entry["hooks"][0]["command"], "brief check --hook");
    }

    #[test]
    fn ensure_hook_is_idempotent() {
        let first = ensure_pretooluse_hook(None).unwrap();
        let second = ensure_pretooluse_hook(Some(&first)).unwrap();
        let v: Value = serde_json::from_str(&second).unwrap();
        assert_eq!(
            v["hooks"]["PreToolUse"].as_array().unwrap().len(),
            1,
            "hook must not be duplicated on re-install"
        );
    }

    #[test]
    fn ensure_hook_preserves_existing_settings() {
        let existing = r#"{"model":"opus","hooks":{"PreToolUse":[{"matcher":"Bash","hooks":[{"type":"command","command":"echo hi"}]}]}}"#;
        let out = ensure_pretooluse_hook(Some(existing)).unwrap();
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["model"], "opus");
        let arr = v["hooks"]["PreToolUse"].as_array().unwrap();
        assert_eq!(
            arr.len(),
            2,
            "existing Bash hook preserved, brief hook added"
        );
        assert!(
            arr.iter()
                .any(|e| e["hooks"][0]["command"] == "brief check --hook")
        );
        assert!(arr.iter().any(|e| e["hooks"][0]["command"] == "echo hi"));
    }

    #[test]
    fn ensure_hook_errors_on_invalid_json() {
        assert!(ensure_pretooluse_hook(Some("{ not valid")).is_err());
    }

    #[test]
    fn remove_hook_drops_brief_entry_and_prunes_containers() {
        let with = ensure_pretooluse_hook(None).unwrap();
        let out = remove_pretooluse_hook(&with).unwrap();
        let v: Value = serde_json::from_str(&out).unwrap();
        // PreToolUse was the only hook → both containers pruned.
        assert!(
            v.get("hooks").is_none(),
            "empty hooks container should be pruned"
        );
    }

    #[test]
    fn remove_hook_preserves_other_hooks_and_settings() {
        let existing = r#"{"model":"opus","hooks":{"PreToolUse":[{"matcher":"Bash","hooks":[{"type":"command","command":"echo hi"}]}]}}"#;
        let with = ensure_pretooluse_hook(Some(existing)).unwrap();
        let out = remove_pretooluse_hook(&with).unwrap();
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["model"], "opus");
        let arr = v["hooks"]["PreToolUse"].as_array().unwrap();
        assert_eq!(arr.len(), 1, "non-brief hook survives");
        assert_eq!(arr[0]["hooks"][0]["command"], "echo hi");
        assert!(!out.contains("brief check --hook"));
    }

    #[test]
    fn remove_hook_is_noop_when_absent() {
        let out = remove_pretooluse_hook(r#"{"model":"opus"}"#).unwrap();
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["model"], "opus");
    }

    #[test]
    fn permissions_allow_adds_and_dedupes() {
        let first = ensure_permissions_allow(None, &["Bash(cargo build:*)".into()]).unwrap();
        let v: Value = serde_json::from_str(&first).unwrap();
        assert_eq!(v["permissions"]["allow"][0], "Bash(cargo build:*)");

        // Re-applying the same entry does not duplicate it.
        let second = ensure_permissions_allow(
            Some(&first),
            &["Bash(cargo build:*)".into(), "Bash(cargo test:*)".into()],
        )
        .unwrap();
        let v2: Value = serde_json::from_str(&second).unwrap();
        let allow = v2["permissions"]["allow"].as_array().unwrap();
        assert_eq!(allow.len(), 2);
    }

    #[test]
    fn permissions_allow_preserves_existing_settings() {
        let existing = r#"{"model":"opus","permissions":{"allow":["Bash(ls:*)"]}}"#;
        let out =
            ensure_permissions_allow(Some(existing), &["Bash(cargo build:*)".into()]).unwrap();
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["model"], "opus");
        let allow = v["permissions"]["allow"].as_array().unwrap();
        assert_eq!(allow.len(), 2);
    }
}
