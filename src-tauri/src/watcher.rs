use agents_manager_core::error::CoreError;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

pub type ChangeEmitter = Arc<dyn Fn(PathBuf) + Send + Sync>;

#[derive(Default)]
pub struct WatchState {
    pub watcher: Mutex<Option<RecommendedWatcher>>,
    pub emitter: Mutex<Option<ChangeEmitter>>,
}

pub fn watch_path(
    path: &Path,
    on_change: impl Fn(PathBuf) + Send + 'static,
) -> Result<RecommendedWatcher, CoreError> {
    let target = path.to_path_buf();
    let parent = path.parent().ok_or_else(|| CoreError::PathNotAllowed {
        path: path.display().to_string(),
    })?;
    let mut watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
        if let Ok(event) = event
            && matches!(
                event.kind,
                notify::EventKind::Create(_)
                    | notify::EventKind::Modify(_)
                    | notify::EventKind::Remove(_)
            )
            && event.paths.iter().any(|path| path == &target)
        {
            on_change(target.clone());
        }
    })
    .map_err(|err| CoreError::IoError {
        message: err.to_string(),
    })?;
    watcher
        .watch(parent, RecursiveMode::NonRecursive)
        .map_err(|err| CoreError::IoError {
            message: err.to_string(),
        })?;
    Ok(watcher)
}
