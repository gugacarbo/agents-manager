use agents_manager_core::claude_desktop_quotas::{
    DesktopQuotaError, DesktopQuotaProfile, DesktopQuotaProfiles, DesktopQuotaRefresh,
    source::{LinuxDesktopQuotaSource, ensure_closed},
};
use std::{
    os::unix::fs::DirBuilderExt,
    path::PathBuf,
    sync::{Arc, OnceLock},
};

pub(crate) fn private_store() -> Result<PathBuf, agents_manager_core::error::CoreError> {
    Ok(super::data_dir()?.join("claude-desktop-quotas"))
}
fn service() -> Result<&'static DesktopQuotaProfiles, DesktopQuotaError> {
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
        static LOGIN_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _guard = LOGIN_LOCK
            .lock()
            .map_err(|_| DesktopQuotaError::StorageUnavailable)?;
        ensure_closed()?;
        let xdg = dir.join("xdg");
        let mut command = std::process::Command::new("claude-desktop");
        for (var, name) in [
            ("XDG_CONFIG_HOME", "config"),
            ("XDG_DATA_HOME", "data"),
            ("XDG_CACHE_HOME", "cache"),
            ("XDG_STATE_HOME", "state"),
        ] {
            let path = xdg.join(name);
            std::fs::DirBuilder::new()
                .recursive(true)
                .mode(0o700)
                .create(&path)
                .map_err(|_| DesktopQuotaError::StorageUnavailable)?;
            command.env(var, path);
        }
        let mut child = command
            .env_remove("CLAUDE_USER_DATA_DIR")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|_| DesktopQuotaError::SourceUnavailable)?;
        std::thread::spawn(move || {
            let _ = child.wait();
        });
        Ok(())
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
