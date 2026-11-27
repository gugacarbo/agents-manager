use super::{public_configs, registry, registry_lock, reject_account_path};
use agents_manager_core::domain::{AgentKind, ConfigFileEntry, ProjectSummary};
use agents_manager_core::error::CoreError;
use std::path::Path;

#[tauri::command]
#[specta::specta]
pub fn list_projects() -> Result<Vec<ProjectSummary>, CoreError> {
    let _guard = registry_lock()?;
    agents_manager_core::projects::list_projects(&registry()?)
}

#[tauri::command]
#[specta::specta]
pub fn register_project(root_path: String) -> Result<ProjectSummary, CoreError> {
    let _guard = registry_lock()?;
    let root = Path::new(&root_path);
    reject_account_path(root)?;
    agents_manager_core::projects::register_project(&registry()?, root)
}

#[tauri::command]
#[specta::specta]
pub fn remove_project(project_id: String) -> Result<(), CoreError> {
    let _guard = registry_lock()?;
    agents_manager_core::projects::remove_project(&registry()?, &project_id)
}

#[tauri::command]
#[specta::specta]
pub fn list_project_configs(
    project_id: String,
    kind: AgentKind,
) -> Result<Vec<ConfigFileEntry>, CoreError> {
    let _guard = registry_lock()?;
    let project = agents_manager_core::projects::list_projects(&registry()?)?
        .into_iter()
        .find(|project| project.id == project_id)
        .ok_or(CoreError::NotFound { target: project_id })?;
    if !project.available
        || !Path::new(&project.root_path)
            .canonicalize()
            .is_ok_and(|root| root == Path::new(&project.root_path))
    {
        return Ok(Vec::new());
    }
    let root = Path::new(&project.root_path);
    reject_account_path(root)?;
    public_configs(agents_manager_adapters::list_project_configs(kind, root))
}
