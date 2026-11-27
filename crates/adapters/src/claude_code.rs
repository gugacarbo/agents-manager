use std::path::Path;

use agents_manager_core::domain::{
    AgentKind, AgentSummary, ConfigCategory as C, ConfigFileEntry, ConfigFormat as F,
};

use crate::{AgentAdapter, add, add_children, detect_first_existing_dir};

/// Adapter for Anthropic Claude Code configuration under `~/.claude`.
#[derive(Debug, Default, Clone, Copy)]
pub struct ClaudeCodeAdapter;

impl AgentAdapter for ClaudeCodeAdapter {
    fn kind(&self) -> AgentKind {
        AgentKind::ClaudeCode
    }

    fn name(&self) -> &'static str {
        "Claude Code"
    }

    fn detect(&self, home: &Path) -> AgentSummary {
        detect_first_existing_dir(self.kind(), self.name(), home, &[".claude"])
    }

    fn list_global_configs(&self, home: &Path) -> Vec<ConfigFileEntry> {
        let kind = self.kind();
        let mut entries = Vec::new();
        add(
            &mut entries,
            home,
            kind,
            C::Settings,
            F::Json,
            ".claude/settings.json",
        );
        add(&mut entries, home, kind, C::Mcps, F::Json, ".claude.json");
        add(
            &mut entries,
            home,
            kind,
            C::Mcps,
            F::Json,
            ".claude/.claude.json",
        );
        add(
            &mut entries,
            home,
            kind,
            C::Rules,
            F::Markdown,
            ".claude/CLAUDE.md",
        );
        add_children(&mut entries, home, kind, C::Skills, ".claude/skills", true);
        entries
    }
    fn list_project_configs(&self, root: &Path) -> Vec<ConfigFileEntry> {
        let mut entries = Vec::new();
        let kind = self.kind();
        add(&mut entries, root, kind, C::Mcps, F::Json, ".mcp.json");
        add(
            &mut entries,
            root,
            kind,
            C::Settings,
            F::Json,
            ".claude/settings.json",
        );
        add(
            &mut entries,
            root,
            kind,
            C::Settings,
            F::Json,
            ".claude/settings.local.json",
        );
        add(&mut entries, root, kind, C::Rules, F::Markdown, "CLAUDE.md");
        add(
            &mut entries,
            root,
            kind,
            C::Rules,
            F::Markdown,
            "CLAUDE.local.md",
        );
        add_children(&mut entries, root, kind, C::Skills, ".claude/skills", true);
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
    fn detects_claude_directory_when_present() {
        let temp = tempdir().unwrap();
        let expected = temp.path().join(".claude");
        fs::create_dir(&expected).unwrap();

        let summary = ClaudeCodeAdapter.detect(temp.path());
        assert_eq!(summary.kind, AgentKind::ClaudeCode);
        assert_eq!(summary.name, "Claude Code");
        assert!(summary.detected);
        assert_eq!(
            summary.config_path,
            Some(expected.to_string_lossy().into_owned())
        );
    }

    #[test]
    fn reports_undetected_when_missing_or_file() {
        let temp = tempdir().unwrap();
        let summary = ClaudeCodeAdapter.detect(temp.path());
        assert!(!summary.detected);
        assert_eq!(summary.config_path, None);

        fs::write(temp.path().join(".claude"), "not a directory").unwrap();
        let summary = ClaudeCodeAdapter.detect(temp.path());
        assert!(!summary.detected);
        assert_eq!(summary.config_path, None);
    }
}
