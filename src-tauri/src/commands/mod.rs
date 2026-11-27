use agents_manager_core::domain::AgentSummary;
use agents_manager_core::error::CoreError;
use directories::BaseDirs;

/// Lists all supported coding agents and their detection status under the user's home directory.
#[tauri::command]
#[specta::specta]
pub fn list_agents() -> Result<Vec<AgentSummary>, CoreError> {
    let base_dirs = BaseDirs::new().ok_or_else(|| CoreError::NotFound {
        target: "user home directory".to_owned(),
    })?;
    Ok(agents_manager_adapters::detect_all(base_dirs.home_dir()))
}
