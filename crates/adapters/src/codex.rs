use std::path::Path;

use agents_manager_core::domain::{AgentKind, AgentSummary};

use crate::{AgentAdapter, detect_first_existing_dir};

/// Adapter for OpenAI Codex CLI configuration under `~/.codex`.
#[derive(Debug, Default, Clone, Copy)]
pub struct CodexAdapter;

impl AgentAdapter for CodexAdapter {
    fn kind(&self) -> AgentKind {
        AgentKind::Codex
    }

    fn name(&self) -> &'static str {
        "Codex"
    }

    fn detect(&self, home: &Path) -> AgentSummary {
        detect_first_existing_dir(self.kind(), self.name(), home, &[".codex"])
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn detects_codex_directory_when_present() {
        let temp = tempdir().unwrap();
        let expected = temp.path().join(".codex");
        fs::create_dir(&expected).unwrap();

        let summary = CodexAdapter.detect(temp.path());
        assert_eq!(summary.kind, AgentKind::Codex);
        assert_eq!(summary.name, "Codex");
        assert!(summary.detected);
        assert_eq!(
            summary.config_path,
            Some(expected.to_string_lossy().into_owned())
        );
    }

    #[test]
    fn reports_undetected_when_missing() {
        let temp = tempdir().unwrap();
        let summary = CodexAdapter.detect(temp.path());
        assert!(!summary.detected);
        assert_eq!(summary.config_path, None);
    }
}
