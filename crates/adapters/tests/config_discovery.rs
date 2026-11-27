#![allow(clippy::unwrap_used)]

use agents_manager_adapters::{allowed_config, list_agent_configs};
use agents_manager_core::domain::{AgentKind, ConfigCategory};
use std::fs;
use tempfile::tempdir;

#[test]
fn lists_fixed_and_discovered_files_and_fallback() {
    let home = tempdir().unwrap();
    fs::create_dir_all(home.path().join(".codex/skills/example")).unwrap();
    fs::write(home.path().join(".codex/skills/example/SKILL.md"), "skill").unwrap();
    let entries = list_agent_configs(AgentKind::Codex, home.path());
    assert_eq!(
        entries
            .iter()
            .filter(|e| e.category == ConfigCategory::Skills && e.exists)
            .count(),
        1
    );
    assert!(
        entries
            .iter()
            .any(|e| e.path.ends_with(".codex/config.toml") && !e.exists)
    );
    fs::create_dir_all(home.path().join(".gemini")).unwrap();
    fs::write(home.path().join(".gemini/settings.json"), "{}").unwrap();
    let entries = list_agent_configs(AgentKind::Antigravity, home.path());
    assert!(
        entries
            .iter()
            .any(|e| e.path.ends_with(".gemini/settings.json") && e.exists)
    );
}

#[test]
fn rejects_unlisted_and_symlinked_paths() {
    let home = tempdir().unwrap();
    fs::create_dir_all(home.path().join(".codex")).unwrap();
    let target = home.path().join("outside");
    fs::write(&target, "secret").unwrap();
    assert!(allowed_config(home.path(), &target).is_none());
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&target, home.path().join(".codex/config.toml")).unwrap();
        assert!(allowed_config(home.path(), &home.path().join(".codex/config.toml")).is_none());
    }
}

#[test]
fn discovers_all_native_files_and_rejects_unlisted_siblings() {
    let home = tempdir().unwrap();
    let fixtures = [
        (AgentKind::ClaudeCode, ".claude/settings.json"),
        (AgentKind::ClaudeCode, ".claude.json"),
        (AgentKind::ClaudeCode, ".claude/.claude.json"),
        (AgentKind::ClaudeCode, ".claude/CLAUDE.md"),
        (AgentKind::ClaudeCode, ".claude/skills/example/SKILL.md"),
        (AgentKind::Codex, ".codex/config.toml"),
        (AgentKind::Codex, ".codex/AGENTS.md"),
        (AgentKind::Codex, ".codex/skills/example/SKILL.md"),
        (AgentKind::Antigravity, ".gemini/config/config.json"),
        (AgentKind::Antigravity, ".gemini/config/mcp_config.json"),
        (AgentKind::Antigravity, ".gemini/GEMINI.md"),
        (AgentKind::Antigravity, ".gemini/config/rules/example.md"),
        (
            AgentKind::Antigravity,
            ".gemini/config/skills/example/SKILL.md",
        ),
        (AgentKind::Antigravity, ".gemini/skills/example/SKILL.md"),
    ];
    for (_, relative) in fixtures {
        let path = home.path().join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "content").unwrap();
    }
    for (kind, relative) in fixtures {
        let path = home.path().join(relative);
        assert!(
            list_agent_configs(kind, home.path())
                .iter()
                .any(|entry| entry.exists && entry.path == path.to_str().unwrap()),
            "missing {relative}"
        );
        assert!(
            allowed_config(home.path(), &path).is_some(),
            "denied {relative}"
        );
    }
    assert!(allowed_config(home.path(), &home.path().join(".codex/../.claude.json")).is_none());
    assert!(
        allowed_config(
            home.path(),
            &home.path().join(".gemini/config/rules/other.txt")
        )
        .is_none()
    );
}

#[test]
fn discovers_project_paths_and_only_allows_registered_roots() {
    use agents_manager_adapters::{allowed_project_config, list_project_configs};
    let root = tempdir().unwrap();
    let path = root.path().join(".claude/skills/example/SKILL.md");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, "skill").unwrap();
    let configs = list_project_configs(AgentKind::ClaudeCode, root.path());
    assert!(
        configs
            .iter()
            .any(|e| e.path == path.to_str().unwrap() && e.exists)
    );
    assert!(
        configs
            .iter()
            .any(|e| e.path.ends_with(".mcp.json") && !e.exists)
    );
    assert!(allowed_project_config(&[], &path).is_none());
    assert!(allowed_project_config(&[root.path().to_path_buf()], &path).is_some());
    assert!(
        allowed_project_config(&[root.path().to_path_buf()], &root.path().join("secret")).is_none()
    );
}

#[test]
fn project_discovery_covers_every_native_path() {
    use agents_manager_adapters::list_project_configs;
    let root = tempdir().unwrap();
    let fixtures = [
        (AgentKind::ClaudeCode, ".mcp.json"),
        (AgentKind::ClaudeCode, ".claude/settings.json"),
        (AgentKind::ClaudeCode, ".claude/settings.local.json"),
        (AgentKind::ClaudeCode, "CLAUDE.md"),
        (AgentKind::ClaudeCode, "CLAUDE.local.md"),
        (AgentKind::ClaudeCode, ".claude/skills/a/SKILL.md"),
        (AgentKind::Codex, ".codex/config.toml"),
        (AgentKind::Codex, "AGENTS.md"),
        (AgentKind::Codex, "AGENTS.override.md"),
        (AgentKind::Codex, ".agents/skills/a/SKILL.md"),
        (AgentKind::Codex, ".codex/skills/a/SKILL.md"),
        (AgentKind::Antigravity, ".gemini/settings.json"),
        (AgentKind::Antigravity, ".gemini/mcp_config.json"),
        (AgentKind::Antigravity, "AGENTS.md"),
        (AgentKind::Antigravity, ".gemini/GEMINI.md"),
        (AgentKind::Antigravity, ".agents/rules/a.md"),
        (AgentKind::Antigravity, ".agents/skills/a/SKILL.md"),
        (AgentKind::Antigravity, ".gemini/skills/a/SKILL.md"),
    ];
    for (_, relative) in fixtures {
        let path = root.path().join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "content").unwrap();
    }
    for (kind, relative) in fixtures {
        let path = root.path().join(relative);
        assert!(
            list_project_configs(kind, root.path())
                .iter()
                .any(|entry| entry.exists && entry.path == path.to_str().unwrap()),
            "missing {relative}"
        );
    }
}

#[cfg(unix)]
#[test]
fn project_root_replaced_with_symlink_is_not_authorized() {
    use agents_manager_adapters::allowed_project_config;
    let temp = tempdir().unwrap();
    let root = temp.path().join("project");
    let external = temp.path().join("external");
    fs::create_dir(&root).unwrap();
    fs::create_dir(&external).unwrap();
    fs::write(external.join(".mcp.json"), "{}").unwrap();
    fs::remove_dir(&root).unwrap();
    std::os::unix::fs::symlink(&external, &root).unwrap();
    assert!(allowed_project_config(std::slice::from_ref(&root), &root.join(".mcp.json")).is_none());
}
