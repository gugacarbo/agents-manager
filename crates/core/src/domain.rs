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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum ConfigCategory {
    Mcps,
    Skills,
    Rules,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum ConfigFormat {
    Json,
    Toml,
    Markdown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ConfigFileEntry {
    pub agent: AgentKind,
    pub category: ConfigCategory,
    pub label: String,
    pub path: String,
    pub format: ConfigFormat,
    pub exists: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SecretSpan {
    pub start_byte: u32,
    pub end_byte: u32,
    pub kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ConfigDocument {
    pub entry: ConfigFileEntry,
    pub content: String,
    pub sha256: String,
    pub secret_spans: Vec<SecretSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SaveConfigRequest {
    pub path: String,
    pub content: String,
    pub expected_sha256: String,
    pub force_overwrite: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SaveConfigResult {
    pub sha256: String,
    pub backup_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ProjectEntry {
    pub id: String,
    pub name: String,
    pub root_path: String,
    pub added_at: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSummary {
    pub id: String,
    pub name: String,
    pub root_path: String,
    pub added_at: String,
    pub available: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum McpTransport {
    Stdio,
    Http,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct McpEnvVar {
    pub key: String,
    pub value: String,
    pub is_secret: bool,
    pub is_env_ref: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct McpServerConfig {
    pub name: String,
    pub transport: McpTransport,
    pub command: Option<String>,
    pub args: Option<Vec<String>>,
    pub url: Option<String>,
    pub env: Vec<McpEnvVar>,
    pub enabled: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct StructuredSaveRequest {
    pub path: String,
    pub expected_sha256: String,
    pub mcp_servers: Option<Vec<McpServerConfig>>,
    #[specta(type = Option<std::collections::HashMap<String, specta_typescript::Unknown>>)]
    pub settings_patch: Option<serde_json::Map<String, serde_json::Value>>,
    pub force_overwrite: Option<bool>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct StructuredPreviewResult {
    pub content: String,
    pub secret_spans: Vec<SecretSpan>,
}
