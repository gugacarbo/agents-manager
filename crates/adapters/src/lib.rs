pub mod antigravity;
pub mod claude_code;
pub mod codex;

use std::path::Path;

use agents_manager_core::build_agent_summary;
use agents_manager_core::domain::{AgentKind, AgentSummary};

/// Adapter interface for detecting and inspecting a specific coding agent's configuration.
pub trait AgentAdapter: Send + Sync {
    /// Returns the enum identifier for this agent.
    fn kind(&self) -> AgentKind;

    /// Returns the human-readable display name for this agent.
    fn name(&self) -> &'static str;

    /// Detects the agent's configuration under the given `home` directory.
    fn detect(&self, home: &Path) -> AgentSummary;
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
