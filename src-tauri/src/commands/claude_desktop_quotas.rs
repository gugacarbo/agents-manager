use agents_manager_claude_accounts::desktop_quotas::{
    DesktopQuotaError, DesktopQuotaProfile, DesktopQuotaProfiles, DesktopQuotaRefresh,
    source::LinuxDesktopQuotaSource,
};
use std::{
    path::PathBuf,
    sync::{Arc, OnceLock},
};

pub(crate) fn private_store() -> Result<PathBuf, agents_manager_core::error::CoreError> {
    Ok(super::data_dir()?.join("claude-desktop-quotas"))
}

pub(super) fn service() -> Result<&'static DesktopQuotaProfiles, DesktopQuotaError> {
    static SERVICE: OnceLock<DesktopQuotaProfiles> = OnceLock::new();
    if let Some(service) = SERVICE.get() {
        return Ok(service);
    }
    let data = private_store().map_err(|_| DesktopQuotaError::StorageUnavailable)?;
    let dirs = directories::BaseDirs::new().ok_or(DesktopQuotaError::StorageUnavailable)?;
    let config = dirs.config_dir().to_owned();
    let root = data.clone();
    Ok(SERVICE.get_or_init(|| {
        DesktopQuotaProfiles::new(
            data,
            Arc::new(move |id| {
                let path = if id == "default" {
                    config.clone()
                } else {
                    root.join("profiles").join(id).join("xdg/config")
                };
                Arc::new(LinuxDesktopQuotaSource::new(path))
            }),
            Arc::new(|| {
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .ok()
                    .and_then(|d| i64::try_from(d.as_millis()).ok())
                    .unwrap_or_default()
            }),
        )
    }))
}

#[tauri::command]
#[specta::specta]
pub fn list_claude_desktop_quota_profiles() -> Result<Vec<DesktopQuotaProfile>, DesktopQuotaError> {
    service()?.list()
}

#[tauri::command]
#[specta::specta]
pub fn create_claude_desktop_quota_profile() -> Result<DesktopQuotaProfile, DesktopQuotaError> {
    service()?.create()
}

#[tauri::command]
#[specta::specta]
pub async fn open_claude_desktop_quota_login(profile_id: String) -> Result<(), DesktopQuotaError> {
    tauri::async_runtime::spawn_blocking(move || {
        let dir = service()?.profile_dir(&profile_id)?;
        if profile_id == "default" {
            return Err(DesktopQuotaError::SourceUnavailable);
        }
        let store = private_store().map_err(|_| DesktopQuotaError::StorageUnavailable)?;
        crate::claude_desktop_launcher::launch_login(&store, &dir)
    })
    .await
    .map_err(|_| DesktopQuotaError::SourceUnavailable)?
}

#[tauri::command]
#[specta::specta]
pub async fn open_claude_desktop_quota_profile(
    profile_id: String,
) -> Result<(), DesktopQuotaError> {
    tauri::async_runtime::spawn_blocking(move || {
        let profiles = service()?;
        profiles.ensure_linked(&profile_id)?;
        let dir = profiles.profile_dir(&profile_id)?;
        let xdg = (profile_id != "default").then(|| dir.join("xdg"));
        let store = private_store().map_err(|_| DesktopQuotaError::StorageUnavailable)?;
        crate::claude_desktop_launcher::launch_authenticated_profile(&store, xdg.as_deref())
    })
    .await
    .map_err(|_| DesktopQuotaError::SourceUnavailable)?
}

#[tauri::command]
#[specta::specta]
pub async fn link_claude_desktop_quota_account(
    profile_id: String,
) -> Result<DesktopQuotaProfile, DesktopQuotaError> {
    tauri::async_runtime::spawn_blocking(move || service()?.link(&profile_id))
        .await
        .map_err(|_| DesktopQuotaError::SourceUnavailable)?
}

#[tauri::command]
#[specta::specta]
pub async fn refresh_claude_desktop_quota(
    profile_id: String,
) -> Result<DesktopQuotaRefresh, DesktopQuotaError> {
    tauri::async_runtime::spawn_blocking(move || service()?.refresh(&profile_id))
        .await
        .map_err(|_| DesktopQuotaError::SourceUnavailable)?
}
