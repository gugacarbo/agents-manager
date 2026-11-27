#![allow(clippy::unwrap_used)]
use agents_manager_adapters::resolve_sync_target;
use agents_manager_core::domain::{AgentKind as A, ConfigCategory as C, ConfigScope as S};
use tempfile::tempdir;

#[test]
fn resolves_native_mcp_and_skill_destinations_by_agent_and_scope() {
    let root = tempdir().unwrap();
    for (agent, scope, category, relative) in [
        (A::ClaudeCode, S::Global, C::Mcps, ".claude.json"),
        (A::Codex, S::Global, C::Mcps, ".codex/config.toml"),
        (
            A::Antigravity,
            S::Global,
            C::Mcps,
            ".gemini/config/mcp_config.json",
        ),
        (A::ClaudeCode, S::Project, C::Mcps, ".mcp.json"),
        (
            A::Codex,
            S::Project,
            C::Skills,
            ".agents/skills/example/SKILL.md",
        ),
        (
            A::Antigravity,
            S::Project,
            C::Skills,
            ".gemini/skills/example/SKILL.md",
        ),
    ] {
        let target = resolve_sync_target(agent, scope, root.path(), category, "example").unwrap();
        assert_eq!(target.path, root.path().join(relative).to_str().unwrap());
    }
    assert!(
        resolve_sync_target(A::Codex, S::Project, root.path(), C::Skills, "../escape").is_none()
    );
}

#[cfg(unix)]
#[test]
fn rejects_symlink_in_sync_destination_ancestors() {
    use agents_manager_adapters::safe_sync_target;
    use std::fs;
    let temp = tempdir().unwrap();
    let outside = tempdir().unwrap();
    std::os::unix::fs::symlink(outside.path(), temp.path().join(".codex")).unwrap();
    assert!(!safe_sync_target(
        temp.path(),
        &temp.path().join(".codex/config.toml"),
        false
    ));
    fs::remove_file(temp.path().join(".codex")).unwrap();
    fs::create_dir_all(temp.path().join(".codex/skills")).unwrap();
    std::os::unix::fs::symlink(outside.path(), temp.path().join(".codex/skills/managed")).unwrap();
    assert!(safe_sync_target(
        temp.path(),
        &temp.path().join(".codex/skills/managed/SKILL.md"),
        true
    ));
}
