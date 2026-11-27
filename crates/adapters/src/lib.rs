pub mod antigravity;
pub mod claude_code;
pub mod codex;

use std::path::Path;

use agents_manager_core::build_agent_summary;
use agents_manager_core::domain::{AgentKind, AgentSummary, ConfigFileEntry};

/// Adapter interface for detecting and inspecting a specific coding agent's configuration.
pub trait AgentAdapter: Send + Sync {
    /// Returns the enum identifier for this agent.
    fn kind(&self) -> AgentKind;

    /// Returns the human-readable display name for this agent.
    fn name(&self) -> &'static str;

    /// Detects the agent's configuration under the given `home` directory.
    fn detect(&self, home: &Path) -> AgentSummary;

    /// Discovers native global configuration files under the given home directory.
    fn list_global_configs(&self, home: &Path) -> Vec<ConfigFileEntry>;

    /// Discovers native configuration within a project.
    fn list_project_configs(&self, root: &Path) -> Vec<ConfigFileEntry>;
}

/// Returns instances of all supported agent adapters.
pub fn all_adapters() -> Vec<Box<dyn AgentAdapter>> {
    vec![
        Box::new(claude_code::ClaudeCodeAdapter),
        Box::new(codex::CodexAdapter),
        Box::new(antigravity::AntigravityAdapter),
    ]
}

/// Runs detection for every registered adapter against the provided `home` directory.
pub fn detect_all(home: &Path) -> Vec<AgentSummary> {
    all_adapters()
        .into_iter()
        .map(|adapter| adapter.detect(home))
        .collect()
}

pub(crate) fn detect_first_existing_dir(
    kind: AgentKind,
    name: &'static str,
    home: &Path,
    candidates: &[&str],
) -> AgentSummary {
    let config_path = candidates
        .iter()
        .map(|rel| home.join(rel))
        .find(|path| path.is_dir())
        .and_then(|path| path.to_str().map(str::to_owned));

    build_agent_summary(kind, name, config_path)
}

/// Discover the native global configuration files for one agent.
pub fn list_agent_configs(kind: AgentKind, home: &Path) -> Vec<ConfigFileEntry> {
    all_adapters()
        .into_iter()
        .find(|adapter| adapter.kind() == kind)
        .map_or_else(Vec::new, |adapter| adapter.list_global_configs(home))
}

/// Discover native configuration files for an agent in a project.
pub fn list_project_configs(kind: AgentKind, root: &Path) -> Vec<ConfigFileEntry> {
    all_adapters()
        .into_iter()
        .find(|adapter| adapter.kind() == kind)
        .map_or_else(Vec::new, |adapter| adapter.list_project_configs(root))
}

/// Resolve a canonical sync destination without accepting caller-provided paths.
pub fn resolve_sync_target(
    agent: AgentKind,
    scope: agents_manager_core::domain::ConfigScope,
    root: &Path,
    category: agents_manager_core::domain::ConfigCategory,
    slug: &str,
) -> Option<ConfigFileEntry> {
    use agents_manager_core::domain::{ConfigCategory as C, ConfigFormat as F, ConfigScope as S};
    if slug.is_empty()
        || !slug
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_')
    {
        return None;
    }
    let (relative, format) = match (agent, scope, category) {
        (AgentKind::ClaudeCode, S::Global, C::Mcps) => (".claude.json".into(), F::Json),
        (AgentKind::ClaudeCode, S::Project, C::Mcps) => (".mcp.json".into(), F::Json),
        (AgentKind::ClaudeCode, _, C::Settings) => (".claude/settings.json".into(), F::Json),
        (AgentKind::ClaudeCode, S::Global, C::Rules) => (".claude/CLAUDE.md".into(), F::Markdown),
        (AgentKind::ClaudeCode, S::Project, C::Rules) => ("CLAUDE.md".into(), F::Markdown),
        (AgentKind::ClaudeCode, _, C::Skills) => {
            (format!(".claude/skills/{slug}/SKILL.md"), F::Markdown)
        }
        (AgentKind::Codex, _, C::Mcps | C::Settings) => (".codex/config.toml".into(), F::Toml),
        (AgentKind::Codex, S::Global, C::Rules) => (".codex/AGENTS.md".into(), F::Markdown),
        (AgentKind::Codex, S::Project, C::Rules) => ("AGENTS.md".into(), F::Markdown),
        (AgentKind::Codex, S::Global, C::Skills) => {
            (format!(".codex/skills/{slug}/SKILL.md"), F::Markdown)
        }
        (AgentKind::Codex, S::Project, C::Skills) => {
            (format!(".agents/skills/{slug}/SKILL.md"), F::Markdown)
        }
        (AgentKind::Antigravity, S::Global, C::Mcps) => {
            (".gemini/config/mcp_config.json".into(), F::Json)
        }
        (AgentKind::Antigravity, S::Project, C::Mcps) => {
            (".gemini/mcp_config.json".into(), F::Json)
        }
        (AgentKind::Antigravity, S::Global, C::Settings) => (
            if root.join(".gemini/config/config.json").is_file()
                || !root.join(".gemini/settings.json").is_file()
            {
                ".gemini/config/config.json"
            } else {
                ".gemini/settings.json"
            }
            .into(),
            F::Json,
        ),
        (AgentKind::Antigravity, S::Project, C::Settings) => {
            (".gemini/settings.json".into(), F::Json)
        }
        (AgentKind::Antigravity, S::Global, C::Rules) => (".gemini/GEMINI.md".into(), F::Markdown),
        (AgentKind::Antigravity, S::Project, C::Rules) => (".gemini/GEMINI.md".into(), F::Markdown),
        (AgentKind::Antigravity, S::Global, C::Skills) => (
            format!(".gemini/config/skills/{slug}/SKILL.md"),
            F::Markdown,
        ),
        (AgentKind::Antigravity, S::Project, C::Skills) => {
            (format!(".gemini/skills/{slug}/SKILL.md"), F::Markdown)
        }
    };
    let mut entries = Vec::new();
    add(&mut entries, root, agent, category, format, &relative);
    entries.pop()
}

/// Reject symlink traversal in destination ancestors, while leaving the final link
/// (or skill directory link) for the sync engine to validate.
pub fn safe_sync_target(root: &Path, target: &Path, skill: bool) -> bool {
    let Some(parent) = target.parent() else {
        return false;
    };
    let ancestor = if skill {
        parent.parent().unwrap_or(parent)
    } else {
        parent
    };
    ancestor == root || safe_relative(root, ancestor)
}

/// Resolve a path only when it belongs to a registered project and is discovered.
pub fn allowed_project_config(
    roots: &[std::path::PathBuf],
    path: &Path,
) -> Option<ConfigFileEntry> {
    roots
        .iter()
        .filter(|root| {
            root.canonicalize()
                .is_ok_and(|canonical| canonical == **root)
                && safe_relative(root, path)
        })
        .flat_map(|root| {
            [
                AgentKind::ClaudeCode,
                AgentKind::Codex,
                AgentKind::Antigravity,
            ]
            .into_iter()
            .flat_map(move |kind| list_project_configs(kind, root))
        })
        .find(|entry| Path::new(&entry.path) == path)
}

fn safe_relative(root: &Path, path: &Path) -> bool {
    let Ok(relative) = path.strip_prefix(root) else {
        return false;
    };
    let mut current = root.to_path_buf();
    for component in relative.components() {
        if !matches!(component, std::path::Component::Normal(_)) {
            return false;
        }
        current.push(component);
        if std::fs::symlink_metadata(&current)
            .is_ok_and(|metadata| metadata.file_type().is_symlink())
        {
            return false;
        }
    }
    true
}

fn add(
    entries: &mut Vec<ConfigFileEntry>,
    home: &Path,
    agent: AgentKind,
    category: agents_manager_core::domain::ConfigCategory,
    format: agents_manager_core::domain::ConfigFormat,
    relative: &str,
) {
    let path = home.join(relative);
    entries.push(agents_manager_core::domain::ConfigFileEntry {
        agent,
        category,
        format,
        label: path.file_name().map_or_else(
            || relative.to_owned(),
            |name| name.to_string_lossy().into_owned(),
        ),
        path: path.to_string_lossy().into_owned(),
        exists: path.is_file(),
    });
}

fn add_children(
    entries: &mut Vec<ConfigFileEntry>,
    home: &Path,
    agent: AgentKind,
    category: agents_manager_core::domain::ConfigCategory,
    relative: &str,
    skills: bool,
) {
    let Ok(children) = std::fs::read_dir(home.join(relative)) else {
        return;
    };
    let mut paths: Vec<_> = children
        .filter_map(Result::ok)
        .map(|child| child.path())
        .collect();
    paths.sort();
    for child in paths {
        let path = if skills {
            child.join("SKILL.md")
        } else {
            child
        };
        if !path.is_file() || (!skills && path.extension().is_none_or(|ext| ext != "md")) {
            continue;
        }
        let Ok(relative_path) = path.strip_prefix(home) else {
            continue;
        };
        add(
            entries,
            home,
            agent,
            category,
            agents_manager_core::domain::ConfigFormat::Markdown,
            &relative_path.to_string_lossy(),
        );
    }
}

/// Resolve only an exact discovered path, rejecting symbolic-link traversal.
pub fn allowed_config(home: &Path, path: &Path) -> Option<ConfigFileEntry> {
    if !safe_relative(home, path) {
        return None;
    }
    [
        AgentKind::ClaudeCode,
        AgentKind::Codex,
        AgentKind::Antigravity,
    ]
    .into_iter()
    .flat_map(|kind| list_agent_configs(kind, home))
    .find(|entry| Path::new(&entry.path) == path)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn detect_all_returns_all_agents_when_none_present() {
        let temp = tempdir().unwrap();
        let summaries = detect_all(temp.path());

        assert_eq!(summaries.len(), 3);
        assert!(summaries.iter().all(|summary| !summary.detected));
        assert!(
            summaries
                .iter()
                .all(|summary| summary.config_path.is_none())
        );
    }

    #[test]
    fn detect_all_reports_detected_agents_only() {
        let temp = tempdir().unwrap();
        fs::create_dir(temp.path().join(".claude")).unwrap();
        fs::create_dir_all(temp.path().join(".gemini/antigravity")).unwrap();

        let summaries = detect_all(temp.path());
        assert_eq!(summaries.len(), 3);

        let claude = summaries
            .iter()
            .find(|s| s.kind == AgentKind::ClaudeCode)
            .unwrap();
        assert!(claude.detected);
        assert!(claude.config_path.as_deref().unwrap().ends_with(".claude"));

        let codex = summaries
            .iter()
            .find(|s| s.kind == AgentKind::Codex)
            .unwrap();
        assert!(!codex.detected);
        assert!(codex.config_path.is_none());

        let antigravity = summaries
            .iter()
            .find(|s| s.kind == AgentKind::Antigravity)
            .unwrap();
        assert!(antigravity.detected);
        assert!(
            antigravity
                .config_path
                .as_deref()
                .unwrap()
                .ends_with(".gemini/antigravity")
        );
    }
}
