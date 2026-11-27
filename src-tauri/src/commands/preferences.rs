use super::data_dir;
use agents_manager_core::domain::AppPreferences;
use agents_manager_core::error::CoreError;
use std::{
    path::PathBuf,
    sync::{Mutex, MutexGuard},
};

static PREFERENCES_LOCK: Mutex<()> = Mutex::new(());

fn preferences_lock() -> Result<MutexGuard<'static, ()>, CoreError> {
    PREFERENCES_LOCK.lock().map_err(|err| CoreError::IoError {
        message: err.to_string(),
    })
}

fn preferences_path() -> Result<PathBuf, CoreError> {
    Ok(data_dir()?.join("preferences.json"))
}

pub(crate) fn load_initial_minimize_to_tray() -> bool {
    let Ok(_guard) = preferences_lock() else {
        return AppPreferences::default().minimize_to_tray;
    };
    preferences_path()
        .and_then(|path| agents_manager_core::preferences::load_preferences(&path))
        .map_or_else(
            |_| AppPreferences::default().minimize_to_tray,
            |prefs| prefs.minimize_to_tray,
        )
}

/// Reads persisted desktop preferences and refreshes the in-memory tray state.
#[tauri::command]
#[specta::specta]
pub fn get_app_preferences(
    state: tauri::State<'_, crate::preferences_state::PreferencesState>,
) -> Result<AppPreferences, CoreError> {
    let _guard = preferences_lock()?;
    let prefs = agents_manager_core::preferences::load_preferences(&preferences_path()?)?;
    state.set_minimize_to_tray(prefs.minimize_to_tray);
    Ok(prefs)
}

/// Persists desktop preferences and updates the in-memory tray state immediately.
#[tauri::command]
#[specta::specta]
pub fn set_app_preferences(
    preferences: AppPreferences,
    state: tauri::State<'_, crate::preferences_state::PreferencesState>,
) -> Result<AppPreferences, CoreError> {
    let _guard = preferences_lock()?;
    let saved =
        agents_manager_core::preferences::save_preferences(&preferences_path()?, preferences)?;
    state.set_minimize_to_tray(saved.minimize_to_tray);
    Ok(saved)
}
