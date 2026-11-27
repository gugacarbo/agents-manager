use serde::{Deserialize, Serialize};
use specta::Type;

/// Supported coding agent backends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "kebab-case")]
pub enum AgentKind {
    ClaudeCode,
    Codex,
    Antigravity,
}

/// Scope where an agent configuration resides.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "kebab-case")]
pub enum ConfigScope {
    Global,
    Project,
}

/// Summary of an agent's detection status and configuration location.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AgentSummary {
    pub kind: AgentKind,
    pub name: String,
    pub detected: bool,
    pub config_path: Option<String>,
}
