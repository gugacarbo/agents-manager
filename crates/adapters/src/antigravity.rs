use std::path::Path;

use agents_manager_core::domain::{AgentKind, AgentSummary};

use crate::{AgentAdapter, detect_first_existing_dir};

/// Adapter for Antigravity configuration under `~/.gemini/antigravity` or `~/.gemini`.
#[derive(Debug, Default, Clone, Copy)]
pub struct AntigravityAdapter;

impl AgentAdapter for AntigravityAdapter {
    fn kind(&self) -> AgentKind {
        AgentKind::Antigravity
    }

    fn name(&self) -> &'static str {
        "Antigravity"
    }

    fn detect(&self, home: &Path) -> AgentSummary {
        detect_first_existing_dir(
            self.kind(),
            self.name(),
            home,
            &[".gemini/antigravity", ".gemini"],
        )
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn prefers_antigravity_subdirectory_when_present() {
        let temp = tempdir().unwrap();
        let expected = temp.path().join(".gemini/antigravity");
        fs::create_dir_all(&expected).unwrap();

        let summary = AntigravityAdapter.detect(temp.path());
        assert_eq!(summary.kind, AgentKind::Antigravity);
        assert_eq!(summary.name, "Antigravity");
        assert!(summary.detected);
        assert_eq!(
            summary.config_path,
            Some(expected.to_string_lossy().into_owned())
        );
    }

    #[test]
    fn falls_back_to_gemini_directory() {
        let temp = tempdir().unwrap();
        let expected = temp.path().join(".gemini");
        fs::create_dir(&expected).unwrap();

        let summary = AntigravityAdapter.detect(temp.path());
        assert!(summary.detected);
        assert_eq!(
            summary.config_path,
            Some(expected.to_string_lossy().into_owned())
        );
    }

    #[test]
    fn reports_undetected_when_missing() {
        let temp = tempdir().unwrap();
        let summary = AntigravityAdapter.detect(temp.path());
        assert!(!summary.detected);
        assert_eq!(summary.config_path, None);
    }
}
