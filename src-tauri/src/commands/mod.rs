#[cfg(target_os = "linux")]
pub mod claude_accounts;
pub mod remote;

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
    AgentKind, ConfigCategory, ConfigDocument, ConfigFileEntry, ConfigScope, LibraryItem,
    LibraryItemContent, ProjectSummary, PromoteLibraryRequest, ResolvedSyncTarget,
    SaveConfigRequest, SaveConfigResult, SaveLibraryItemRequest, StructuredPreviewResult,
    StructuredSaveRequest, SyncApplyRequest, SyncApplyResult, SyncApplyTarget, SyncPreviewItem,
    SyncPreviewRequest, SyncSelection,
};
use std::{
    path::{Path, PathBuf},
    sync::{
        Mutex, MutexGuard,
        atomic::{AtomicU64, Ordering},
    },
};

static REGISTRY_LOCK: Mutex<()> = Mutex::new(());
static SSH_REVISION: AtomicU64 = AtomicU64::new(0);

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

fn account_private_path(path: &Path) -> Result<bool, CoreError> {
    #[cfg(target_os = "linux")]
    {
        Ok(agents_manager_core::claude_accounts::is_private_path(
            &claude_accounts::private_store()?,
            path,
        ))
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = path;
        Ok(false)
    }
}

fn reject_account_path(path: &Path) -> Result<(), CoreError> {
    if account_private_path(path)? {
        return Err(CoreError::PathNotAllowed {
            path: "private account storage".into(),
        });
    }
    Ok(())
}

fn public_configs(entries: Vec<ConfigFileEntry>) -> Result<Vec<ConfigFileEntry>, CoreError> {
    let mut public = Vec::with_capacity(entries.len());
    for entry in entries {
        if !account_private_path(Path::new(&entry.path))? {
            public.push(entry);
        }
    }
    Ok(public)
}

fn authorized_in(home: &Path, registry: &Path, path: &Path) -> Result<ConfigFileEntry, CoreError> {
    reject_account_path(path)?;
    if let Some(entry) = agents_manager_adapters::allowed_config(home, path) {
        return Ok(entry);
    }
    let roots = agents_manager_core::projects::list_projects(registry)?
        .into_iter()
        .filter(|project| project.available)
        .map(|project| PathBuf::from(project.root_path))
        .collect::<Vec<_>>();
    if let Some(entry) = agents_manager_adapters::allowed_project_config(&roots, path) {
        return Ok(entry);
    }
    if agents_manager_core::library::is_managed_target(&library_dir()?, path)? {
        for root in std::iter::once(home.to_path_buf()).chain(roots) {
            if root != home && !root.canonicalize().is_ok_and(|canonical| canonical == root) {
                continue;
            }
            for agent in [
                AgentKind::ClaudeCode,
                AgentKind::Codex,
                AgentKind::Antigravity,
            ] {
                let entries = if root == home {
                    agents_manager_adapters::list_agent_configs(agent, &root)
                } else {
                    agents_manager_adapters::list_project_configs(agent, &root)
                };
                if let Some(entry) = entries.into_iter().find(|entry| {
                    Path::new(&entry.path) == path
                        && agents_manager_adapters::safe_sync_target(
                            &root,
                            path,
                            entry.category == ConfigCategory::Skills,
                        )
                }) {
                    return Ok(entry);
                }
            }
        }
    }
    Err(CoreError::PathNotAllowed {
        path: path.to_string_lossy().into_owned(),
    })
}

#[tauri::command]
#[specta::specta]
pub fn list_agent_configs(kind: AgentKind) -> Result<Vec<ConfigFileEntry>, CoreError> {
    public_configs(agents_manager_adapters::list_agent_configs(kind, &home()?))
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

fn library_dir() -> Result<PathBuf, CoreError> {
    Ok(data_dir()?.join("library"))
}

#[tauri::command]
#[specta::specta]
pub fn list_library_items() -> Result<Vec<LibraryItem>, CoreError> {
    let _guard = registry_lock()?;
    agents_manager_core::library::list_items(&library_dir()?)
}

#[tauri::command]
#[specta::specta]
pub fn read_library_item(item_id: String) -> Result<LibraryItemContent, CoreError> {
    let _guard = registry_lock()?;
    agents_manager_core::library::read_item(&library_dir()?, &item_id)
}

#[tauri::command]
#[specta::specta]
pub fn promote_to_library(request: PromoteLibraryRequest) -> Result<LibraryItem, CoreError> {
    let _guard = registry_lock()?;
    let mut entry = authorized_in(&home()?, &registry()?, Path::new(&request.path))?;
    if entry.category != request.category {
        if entry.agent == AgentKind::Codex
            && entry.category == ConfigCategory::Settings
            && request.category == ConfigCategory::Mcps
        {
            entry.category = ConfigCategory::Mcps;
        } else {
            return Err(CoreError::PathNotAllowed { path: request.path });
        }
    }
    agents_manager_core::library::promote(
        &library_dir()?,
        entry,
        &request.name,
        request.confirm_literal_secrets,
    )
}

#[tauri::command]
#[specta::specta]
pub fn save_library_item(request: SaveLibraryItemRequest) -> Result<LibraryItem, CoreError> {
    let _guard = registry_lock()?;
    agents_manager_core::library::save_item(&library_dir()?, &request)
}

#[tauri::command]
#[specta::specta]
pub fn delete_library_item(item_id: String) -> Result<(), CoreError> {
    let _guard = registry_lock()?;
    agents_manager_core::library::delete_item(&library_dir()?, &item_id)
}

#[tauri::command]
#[specta::specta]
pub fn unlink_sync(item_id: String, target_path: String) -> Result<(), CoreError> {
    let _guard = registry_lock()?;
    let path = Path::new(&target_path);
    reject_account_path(path)?;
    agents_manager_core::library::unlink(&library_dir()?, &item_id, path)
}

fn resolve_targets(
    item: &LibraryItem,
    selections: &[SyncSelection],
) -> Result<Vec<ResolvedSyncTarget>, CoreError> {
    if selections.is_empty() {
        return Err(CoreError::InvalidPath {
            path: "no sync targets".into(),
        });
    }
    let projects = agents_manager_core::projects::list_projects(&registry()?)?;
    let mut targets = Vec::new();
    for selection in selections {
        let root = match (selection.scope, &selection.project_id) {
            (ConfigScope::Global, None) => home()?,
            (ConfigScope::Project, Some(id)) => {
                let project = projects
                    .iter()
                    .find(|p| p.id == *id && p.available)
                    .ok_or_else(|| CoreError::NotFound { target: id.clone() })?;
                let root = PathBuf::from(&project.root_path);
                if !root.canonicalize().is_ok_and(|canonical| canonical == root) {
                    return Err(CoreError::PathNotAllowed {
                        path: project.root_path.clone(),
                    });
                }
                root
            }
            _ => {
                return Err(CoreError::InvalidPath {
                    path: "invalid sync scope".into(),
                });
            }
        };
        reject_account_path(&root)?;
        let entry = agents_manager_adapters::resolve_sync_target(
            selection.agent,
            selection.scope,
            &root,
            item.category,
            &item.id,
        )
        .ok_or_else(|| CoreError::InvalidPath {
            path: item.id.clone(),
        })?;
        reject_account_path(Path::new(&entry.path))?;
        if !agents_manager_adapters::safe_sync_target(
            &root,
            Path::new(&entry.path),
            item.category == ConfigCategory::Skills,
        ) || targets
            .iter()
            .any(|target: &ResolvedSyncTarget| target.entry.path == entry.path)
        {
            return Err(CoreError::PathNotAllowed { path: entry.path });
        }
        targets.push(ResolvedSyncTarget {
            entry,
            scope: selection.scope,
            project_id: selection.project_id.clone(),
            copy: selection.copy.unwrap_or(false),
        });
    }
    Ok(targets)
}

#[tauri::command]
#[specta::specta]
pub fn preview_sync(
    state: tauri::State<'_, crate::sync_preview::PreviewState>,
    request: SyncPreviewRequest,
) -> Result<Vec<SyncPreviewItem>, CoreError> {
    let _guard = registry_lock()?;
    let dir = library_dir()?;
    let item = agents_manager_core::library::list_items(&dir)?
        .into_iter()
        .find(|item| item.id == request.item_id)
        .ok_or_else(|| CoreError::NotFound {
            target: request.item_id.clone(),
        })?;
    let targets = resolve_targets(&item, &request.targets)?;
    let fingerprint = agents_manager_core::library::item_fingerprint(&dir, &request.item_id)?;
    let previews = agents_manager_core::library::preview_sync(&dir, &request.item_id, &targets)?;
    if previews.len() != targets.len()
        || agents_manager_core::library::item_fingerprint(&dir, &request.item_id)? != fingerprint
    {
        return Err(CoreError::ExternalConflict);
    }
    let apply = SyncApplyRequest {
        item_id: request.item_id,
        targets: request
            .targets
            .into_iter()
            .zip(&previews)
            .map(|(selection, preview)| SyncApplyTarget {
                selection,
                expected_sha256: preview.expected_sha256.clone(),
            })
            .collect(),
    };
    let paths = targets.iter().map(|t| t.entry.path.clone()).collect();
    crate::sync_preview::store_preview(&state, apply, fingerprint, paths, previews.clone())?;
    Ok(previews)
}

#[tauri::command]
#[specta::specta]
pub fn apply_sync(
    state: tauri::State<'_, crate::sync_preview::PreviewState>,
    request: SyncApplyRequest,
) -> Result<Vec<SyncApplyResult>, CoreError> {
    let _guard = registry_lock()?;
    let dir = library_dir()?;
    let item = agents_manager_core::library::list_items(&dir)?
        .into_iter()
        .find(|item| item.id == request.item_id)
        .ok_or_else(|| CoreError::NotFound {
            target: request.item_id.clone(),
        })?;
    let selections = request
        .targets
        .iter()
        .map(|target| target.selection.clone())
        .collect::<Vec<_>>();
    let targets = resolve_targets(&item, &selections)?;
    let fingerprint = agents_manager_core::library::item_fingerprint(&dir, &request.item_id)?;
    let paths = targets
        .iter()
        .map(|t| t.entry.path.clone())
        .collect::<Vec<_>>();
    let previews = crate::sync_preview::take_preview(&state, &request, &fingerprint, &paths)
        .ok_or(CoreError::ExternalConflict)?;
    agents_manager_core::library::apply_sync(
        &dir,
        &data_dir()?.join("backups"),
        &request.item_id,
        &targets,
        &previews,
    )
}

fn ssh_registry() -> Result<PathBuf, CoreError> {
    Ok(data_dir()?.join("ssh-environments.json"))
}

#[tauri::command]
#[specta::specta]
pub fn list_ssh_environments() -> Result<Vec<agents_manager_core::domain::SshEnvironment>, CoreError>
{
    let _guard = registry_lock()?;
    agents_manager_core::ssh::list_environments(&ssh_registry()?)
}

#[tauri::command]
#[specta::specta]
pub fn save_ssh_environment(
    id: Option<String>,
    draft: agents_manager_core::domain::SshEnvironmentDraft,
) -> Result<agents_manager_core::domain::SshEnvironment, CoreError> {
    let _guard = registry_lock()?;
    let result =
        agents_manager_core::ssh::save_environment(&ssh_registry()?, id.as_deref(), &draft)?;
    SSH_REVISION.fetch_add(1, Ordering::Relaxed);
    Ok(result)
}

#[tauri::command]
#[specta::specta]
pub fn remove_ssh_environment(id: String) -> Result<(), CoreError> {
    let _guard = registry_lock()?;
    agents_manager_core::ssh::remove_environment(&ssh_registry()?, &id)?;
    SSH_REVISION.fetch_add(1, Ordering::Relaxed);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn test_ssh_connection(
    draft: agents_manager_core::domain::SshEnvironmentDraft,
) -> Result<agents_manager_core::domain::SshInventory, CoreError> {
    tauri::async_runtime::spawn_blocking(move || agents_manager_core::ssh::inventory(&draft))
        .await
        .map_err(|err| CoreError::IoError {
            message: err.to_string(),
        })?
}

fn ssh_snapshot_unchanged(
    registry: &Path,
    snapshot: &agents_manager_core::domain::SshEnvironment,
) -> Result<bool, CoreError> {
    Ok(agents_manager_core::ssh::list_environments(registry)?
        .into_iter()
        .any(|entry| entry == *snapshot))
}

#[tauri::command]
#[specta::specta]
pub async fn inspect_ssh_environment(
    id: String,
) -> Result<agents_manager_core::domain::SshInventory, CoreError> {
    let (environment, revision) = {
        let _guard = registry_lock()?;
        let environment = agents_manager_core::ssh::list_environments(&ssh_registry()?)?
            .into_iter()
            .find(|entry| entry.id == id)
            .ok_or_else(|| CoreError::NotFound { target: id.clone() })?;
        (environment, SSH_REVISION.load(Ordering::Relaxed))
    };
    let draft = agents_manager_core::domain::SshEnvironmentDraft {
        name: environment.name.clone(),
        connection: environment.connection.clone(),
    };
    let result =
        tauri::async_runtime::spawn_blocking(move || agents_manager_core::ssh::inventory(&draft))
            .await
            .map_err(|err| CoreError::IoError {
                message: err.to_string(),
            })?;
    let _guard = registry_lock()?;
    let unchanged = ssh_snapshot_unchanged(&ssh_registry()?, &environment)?;
    if !unchanged || SSH_REVISION.load(Ordering::Relaxed) != revision {
        return Err(CoreError::ExternalConflict);
    }
    result
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
    #[test]
    fn ssh_results_are_discarded_after_connection_change_or_removal() {
        use agents_manager_core::domain::{SshConnection, SshEnvironmentDraft};
        let temp = tempfile::tempdir().unwrap();
        let registry = temp.path().join("ssh.json");
        let draft = SshEnvironmentDraft {
            name: "test".into(),
            connection: SshConnection::Alias {
                alias: "host".into(),
            },
        };
        let original = agents_manager_core::ssh::save_environment(&registry, None, &draft).unwrap();
        assert!(ssh_snapshot_unchanged(&registry, &original).unwrap());
        let updated = SshEnvironmentDraft {
            name: "changed".into(),
            ..draft
        };
        agents_manager_core::ssh::save_environment(&registry, Some(&original.id), &updated)
            .unwrap();
        assert!(!ssh_snapshot_unchanged(&registry, &original).unwrap());
        agents_manager_core::ssh::remove_environment(&registry, &original.id).unwrap();
        assert!(!ssh_snapshot_unchanged(&registry, &original).unwrap());
    }
}
