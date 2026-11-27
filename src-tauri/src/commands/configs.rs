use super::{authorized, authorized_in, data_dir, home, registry, registry_lock};
use crate::watcher::{WatchState, watch_path};
use agents_manager_core::domain::{
    ConfigDocument, SaveConfigRequest, SaveConfigResult, StructuredPreviewResult,
    StructuredSaveRequest,
};
use agents_manager_core::error::CoreError;
use std::path::{Path, PathBuf};

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConfigFileChanged {
    pub path: String,
    pub disk_sha256: Option<String>,
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
