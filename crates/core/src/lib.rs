#[cfg(target_os = "linux")]
pub mod claude_accounts;
pub mod config;
pub mod domain;
pub mod error;
pub mod library;
pub mod projects;
pub mod remote_config;
pub mod ssh;
pub mod structured;

use std::path::Path;

use crate::domain::{AgentKind, AgentSummary};
use crate::error::CoreError;

/// Counts how many agents in the provided slice were detected on disk.
pub fn count_detected_agents(agents: &[AgentSummary]) -> usize {
    agents.iter().filter(|agent| agent.detected).count()
}

/// Validates that a configuration path exists and is a directory, returning a normalized UTF-8 string.
pub fn validate_config_dir(path: &Path) -> Result<String, CoreError> {
    let metadata = match std::fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Err(CoreError::NotFound {
                target: path.display().to_string(),
            });
        }
        Err(err) => return Err(CoreError::from(err)),
    };

    if !metadata.is_dir() {
        return Err(CoreError::InvalidPath {
            path: path.display().to_string(),
        });
    }

    path.to_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| CoreError::InvalidPath {
            path: path.display().to_string(),
        })
}

/// Builds an [`AgentSummary`] from an agent kind, display name, and optional resolved path.
pub fn build_agent_summary(
    kind: AgentKind,
    name: impl Into<String>,
    config_path: Option<String>,
) -> AgentSummary {
    AgentSummary {
        kind,
        name: name.into(),
        detected: config_path.is_some(),
        config_path,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn counts_detected_agents_accurately() {
        let agents = vec![
            build_agent_summary(
                AgentKind::ClaudeCode,
                "Claude Code",
                Some("/tmp/.claude".into()),
            ),
            build_agent_summary(AgentKind::Codex, "Codex", None),
            build_agent_summary(
                AgentKind::Antigravity,
                "Antigravity",
                Some("/tmp/.gemini/antigravity".into()),
            ),
        ];

        assert_eq!(count_detected_agents(&agents), 2);
    }

    #[test]
    fn validate_config_dir_accepts_existing_directory() {
        let temp = tempdir().unwrap();
        let config_dir = temp.path().join(".claude");
        fs::create_dir(&config_dir).unwrap();

        let validated = validate_config_dir(&config_dir).unwrap();
        assert_eq!(validated, config_dir.to_str().unwrap());
    }

    #[test]
    fn validate_config_dir_rejects_missing_and_file_paths() {
        let temp = tempdir().unwrap();
        let missing = temp.path().join("missing");
        assert!(matches!(
            validate_config_dir(&missing),
            Err(CoreError::NotFound { .. })
        ));

        let file_path = temp.path().join("file.txt");
        fs::write(&file_path, "not a dir").unwrap();
        assert!(matches!(
            validate_config_dir(&file_path),
            Err(CoreError::InvalidPath { .. })
        ));
    }
}
