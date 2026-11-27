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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "kebab-case")]
pub enum SyncStrategy {
    Symlink,
    Copy,
    Merge,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "kebab-case")]
pub enum SyncTargetStatus {
    InSync,
    Drifted,
    NotLinked,
    BrokenSymlink,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SyncTarget {
    pub agent: AgentKind,
    pub scope: ConfigScope,
    pub project_id: Option<String>,
    pub path: String,
    pub status: SyncTargetStatus,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LibraryItem {
    pub id: String,
    pub category: ConfigCategory,
    pub name: String,
    pub source_path: String,
    pub has_literal_secrets: bool,
    pub targets: Vec<SyncTarget>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LibraryItemContent {
    pub content: String,
    pub secret_spans: Vec<SecretSpan>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SaveLibraryItemRequest {
    pub item_id: Option<String>,
    pub category: ConfigCategory,
    pub name: String,
    pub content: String,
    pub confirm_literal_secrets: Option<bool>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PromoteLibraryRequest {
    pub path: String,
    pub category: ConfigCategory,
    pub name: String,
    pub confirm_literal_secrets: Option<bool>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SyncSelection {
    pub agent: AgentKind,
    pub scope: ConfigScope,
    pub project_id: Option<String>,
    pub copy: Option<bool>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SyncPreviewRequest {
    pub item_id: String,
    pub targets: Vec<SyncSelection>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SyncApplyTarget {
    pub selection: SyncSelection,
    pub expected_sha256: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SyncApplyRequest {
    pub item_id: String,
    pub targets: Vec<SyncApplyTarget>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedSyncTarget {
    pub entry: ConfigFileEntry,
    pub scope: ConfigScope,
    pub project_id: Option<String>,
    pub copy: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SyncPreviewItem {
    pub target_path: String,
    pub agent: AgentKind,
    pub scope: ConfigScope,
    pub strategy: SyncStrategy,
    pub current_content: Option<String>,
    pub proposed_content: String,
    pub expected_sha256: Option<String>,
    pub changed: bool,
    pub warnings: Vec<String>,
    pub current_secret_spans: Vec<SecretSpan>,
    pub proposed_secret_spans: Vec<SecretSpan>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SyncApplyResult {
    pub target_path: String,
    pub applied: bool,
    pub strategy_used: SyncStrategy,
    pub backup_path: Option<String>,
    pub error: Option<crate::error::CoreError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(tag = "mode", rename_all = "lowercase")]
pub enum SshConnection {
    Alias {
        alias: String,
    },
    Manual {
        host: String,
        user: String,
        port: u16,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SshEnvironmentDraft {
    pub name: String,
    pub connection: SshConnection,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SshEnvironment {
    pub id: String,
    pub name: String,
    pub connection: SshConnection,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SshInventory {
    pub host: String,
    pub user: String,
    pub port: u16,
    pub claude: bool,
    pub codex: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "kebab-case")]
pub enum SshErrorKind {
    InvalidInput,
    ClientUnavailable,
    Authentication,
    HostKey,
    Timeout,
    Connection,
    Protocol,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "kebab-case")]
pub enum RemoteConfigId {
    ClaudeSettings,
    ClaudeHome,
    ClaudeNested,
    CodexConfig,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RemoteConfigEntry {
    pub id: RemoteConfigId,
    pub agent: AgentKind,
    pub label: String,
    pub format: ConfigFormat,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteConfigDocument {
    pub entry: RemoteConfigEntry,
    pub host: String,
    pub user: String,
    pub port: u16,
    pub path: String,
    pub content: String,
    pub sha256: String,
    pub secret_spans: Vec<SecretSpan>,
    pub host_key: String,
    pub device: u64,
    pub inode: u64,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub metadata_hash: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RemoteSaveResult {
    pub sha256: String,
    pub backup_path: String,
}

fn default_minimize_to_tray() -> bool {
    true
}

/// User preferences for desktop window and tray behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AppPreferences {
    #[serde(default = "default_minimize_to_tray")]
    pub minimize_to_tray: bool,
}

impl Default for AppPreferences {
    fn default() -> Self {
        Self {
            minimize_to_tray: default_minimize_to_tray(),
        }
    }
}
