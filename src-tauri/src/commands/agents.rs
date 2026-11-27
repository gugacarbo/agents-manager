use super::{home, public_configs};
use agents_manager_core::domain::{AgentKind, AgentSummary, ConfigFileEntry};
use agents_manager_core::error::CoreError;

/// Lists all supported coding agents and their detection status under the user's home directory.
#[tauri::command]
#[specta::specta]
pub fn list_agents() -> Result<Vec<AgentSummary>, CoreError> {
    Ok(agents_manager_adapters::detect_all(&home()?))
}

#[tauri::command]
#[specta::specta]
pub fn list_agent_configs(kind: AgentKind) -> Result<Vec<ConfigFileEntry>, CoreError> {
    public_configs(agents_manager_adapters::list_agent_configs(kind, &home()?))
}
