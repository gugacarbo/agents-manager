use super::{
    authorized_in, data_dir, home, library_dir, registry, registry_lock, reject_account_path,
};
use agents_manager_core::domain::{
    AgentKind, ConfigCategory, ConfigScope, LibraryItem, LibraryItemContent, PromoteLibraryRequest,
    ResolvedSyncTarget, SaveLibraryItemRequest, SyncApplyRequest, SyncApplyResult, SyncApplyTarget,
    SyncPreviewItem, SyncPreviewRequest, SyncSelection,
};
use agents_manager_core::error::CoreError;
use std::path::{Path, PathBuf};

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
