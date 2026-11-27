use std::path::Path;

use agents_manager_core::domain::{
    AgentKind, AgentSummary, ConfigCategory as C, ConfigFileEntry, ConfigFormat as F,
};

use crate::{AgentAdapter, add, add_children, detect_first_existing_dir};

/// Adapter for Antigravity configuration under `~/.gemini/antigravity`.
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
        detect_first_existing_dir(self.kind(), self.name(), home, &[".gemini/antigravity"])
    }

    fn list_global_configs(&self, home: &Path) -> Vec<ConfigFileEntry> {
        let kind = self.kind();
        let mut entries = Vec::new();
        let settings = if home.join(".gemini/config/config.json").is_file()
            || !home.join(".gemini/settings.json").is_file()
        {
            ".gemini/config/config.json"
        } else {
            ".gemini/settings.json"
        };
        add(&mut entries, home, kind, C::Settings, F::Json, settings);
        add(
            &mut entries,
            home,
            kind,
            C::Mcps,
            F::Json,
            ".gemini/config/mcp_config.json",
        );
        add(
            &mut entries,
            home,
            kind,
            C::Rules,
            F::Markdown,
            ".gemini/GEMINI.md",
        );
        add_children(
            &mut entries,
            home,
            kind,
            C::Rules,
            ".gemini/config/rules",
            false,
        );
        add_children(
            &mut entries,
            home,
            kind,
            C::Skills,
            ".gemini/config/skills",
            true,
        );
        add_children(&mut entries, home, kind, C::Skills, ".gemini/skills", true);
        entries
    }
    fn list_project_configs(&self, root: &Path) -> Vec<ConfigFileEntry> {
        let mut entries = Vec::new();
        let kind = self.kind();
        add(
            &mut entries,
            root,
            kind,
            C::Settings,
            F::Json,
            ".gemini/settings.json",
        );
        add(
            &mut entries,
            root,
            kind,
            C::Mcps,
            F::Json,
            ".gemini/mcp_config.json",
        );
        add(&mut entries, root, kind, C::Rules, F::Markdown, "AGENTS.md");
        add(
            &mut entries,
            root,
            kind,
            C::Rules,
            F::Markdown,
            ".gemini/GEMINI.md",
        );
        add_children(&mut entries, root, kind, C::Rules, ".agents/rules", false);
        add_children(&mut entries, root, kind, C::Skills, ".agents/skills", true);
        add_children(&mut entries, root, kind, C::Skills, ".gemini/skills", true);
        entries
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn detects_antigravity_subdirectory_when_present() {
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
    fn reports_undetected_for_gemini_cli_only() {
        let temp = tempdir().unwrap();
        let gemini_dir = temp.path().join(".gemini");
        fs::create_dir(&gemini_dir).unwrap();
        fs::write(gemini_dir.join("settings.json"), "{}").unwrap();

        let summary = AntigravityAdapter.detect(temp.path());
        assert_eq!((summary.detected, summary.config_path), (false, None));
    }

    #[test]
    fn reports_undetected_when_missing() {
        let temp = tempdir().unwrap();
        let summary = AntigravityAdapter.detect(temp.path());
        assert!(!summary.detected);
        assert_eq!(summary.config_path, None);
    }
}
