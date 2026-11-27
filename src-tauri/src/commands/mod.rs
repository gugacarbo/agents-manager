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

use crate::watcher::{WatchState, watch_path};
use agents_manager_core::domain::{
    AgentKind, ConfigDocument, ConfigFileEntry, ProjectSummary, SaveConfigRequest,
    SaveConfigResult, StructuredPreviewResult, StructuredSaveRequest,
};
use std::{
    path::{Path, PathBuf},
    sync::{Mutex, MutexGuard},
};

static REGISTRY_LOCK: Mutex<()> = Mutex::new(());

fn registry_lock() -> Result<MutexGuard<'static, ()>, CoreError> {
    REGISTRY_LOCK.lock().map_err(|err| CoreError::IoError {
        message: err.to_string(),
    })
}

fn data_dir() -> Result<PathBuf, CoreError> {
    directories::ProjectDirs::from("", "", "agents-manager")
        .map(|dirs| dirs.data_dir().to_owned())
        .ok_or_else(|| CoreError::NotFound {
            target: "application data directory".into(),
        })
}

fn registry() -> Result<PathBuf, CoreError> {
    Ok(data_dir()?.join("projects.json"))
}

fn home() -> Result<PathBuf, CoreError> {
    BaseDirs::new()
        .map(|dirs| dirs.home_dir().to_owned())
        .ok_or_else(|| CoreError::NotFound {
            target: "user home directory".into(),
        })
}

pub(crate) fn authorized(path: &str) -> Result<ConfigFileEntry, CoreError> {
    let _guard = registry_lock()?;
    authorized_in(&home()?, &registry()?, Path::new(path))
}

fn authorized_in(home: &Path, registry: &Path, path: &Path) -> Result<ConfigFileEntry, CoreError> {
    if let Some(entry) = agents_manager_adapters::allowed_config(home, path) {
        return Ok(entry);
    }
    let roots = agents_manager_core::projects::list_projects(registry)?
        .into_iter()
        .filter(|project| project.available)
        .map(|project| PathBuf::from(project.root_path))
        .collect::<Vec<_>>();
    agents_manager_adapters::allowed_project_config(&roots, path).ok_or_else(|| {
        CoreError::PathNotAllowed {
            path: path.to_string_lossy().into_owned(),
        }
    })
}

#[tauri::command]
#[specta::specta]
pub fn list_agent_configs(kind: AgentKind) -> Result<Vec<ConfigFileEntry>, CoreError> {
    Ok(agents_manager_adapters::list_agent_configs(kind, &home()?))
}

#[tauri::command]
#[specta::specta]
pub fn read_config_file(path: String) -> Result<ConfigDocument, CoreError> {
    let _guard = registry_lock()?;
    agents_manager_core::config::read_config(authorized_in(
        &home()?,
        &registry()?,
        Path::new(&path),
    )?)
}

#[tauri::command]
#[specta::specta]
pub fn save_config_file(request: SaveConfigRequest) -> Result<SaveConfigResult, CoreError> {
    let _guard = registry_lock()?;
    authorized_in(&home()?, &registry()?, Path::new(&request.path))?;
    agents_manager_core::config::save_config(&request, &data_dir()?.join("backups"))
}

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
    agents_manager_core::projects::register_project(&registry()?, Path::new(&root_path))
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
    Ok(agents_manager_adapters::list_project_configs(
        kind,
        Path::new(&project.root_path),
    ))
}

#[tauri::command]
#[specta::specta]
pub fn preview_structured_save(
    request: StructuredSaveRequest,
    draft_content: Option<String>,
) -> Result<StructuredPreviewResult, CoreError> {
    let _guard = registry_lock()?;
    let entry = authorized_in(&home()?, &registry()?, Path::new(&request.path))?;
    agents_manager_core::structured::preview_structured(entry, &request, draft_content.as_deref())
}

#[tauri::command]
#[specta::specta]
pub fn save_structured_config(
    request: StructuredSaveRequest,
    draft_content: Option<String>,
) -> Result<SaveConfigResult, CoreError> {
    let _guard = registry_lock()?;
    let entry = authorized_in(&home()?, &registry()?, Path::new(&request.path))?;
    agents_manager_core::structured::save_structured(
        entry,
        &request,
        &data_dir()?.join("backups"),
        draft_content.as_deref(),
    )
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConfigFileChanged {
    pub path: String,
    pub disk_sha256: Option<String>,
}

#[tauri::command]
#[specta::specta]
pub fn watch_config_file(
    state: tauri::State<'_, WatchState>,
    path: String,
) -> Result<(), CoreError> {
    authorized(&path)?;
    let target = PathBuf::from(&path);
    let emitter = state
        .emitter
        .lock()
        .map_err(|err| CoreError::IoError {
            message: err.to_string(),
        })?
        .clone();
    let on_change = emitter.clone();
    let watcher = watch_path(&target, move |changed| {
        if let Some(emit) = &on_change {
            emit(changed);
        }
    })?;
    *state.watcher.lock().map_err(|err| CoreError::IoError {
        message: err.to_string(),
    })? = Some(watcher);
    // Report the current hash to cover changes between reading and subscribing.
    if let Some(emit) = emitter {
        emit(target);
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    #[test]
    fn project_removal_revokes_authorization_immediately() {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path().join("home");
        let project = temp.path().join("project");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::create_dir_all(&project).unwrap();
        let registry = temp.path().join("data/projects.json");
        let file = project.join(".mcp.json");
        std::fs::write(&file, "{}").unwrap();
        let item = agents_manager_core::projects::register_project(&registry, &project).unwrap();
        assert!(authorized_in(&home, &registry, &file).is_ok());
        agents_manager_core::projects::remove_project(&registry, &item.id).unwrap();
        assert!(matches!(
            authorized_in(&home, &registry, &file),
            Err(CoreError::PathNotAllowed { .. })
        ));
    }
}
