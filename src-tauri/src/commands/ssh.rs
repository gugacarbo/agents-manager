use super::{data_dir, registry_lock};
use agents_manager_core::domain::{SshEnvironment, SshEnvironmentDraft, SshInventory};
use agents_manager_core::error::CoreError;
use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

pub(super) static SSH_REVISION: AtomicU64 = AtomicU64::new(0);

pub(super) fn ssh_registry() -> Result<PathBuf, CoreError> {
    Ok(data_dir()?.join("ssh-environments.json"))
}

pub(super) fn ssh_snapshot_unchanged(
    registry: &Path,
    snapshot: &SshEnvironment,
) -> Result<bool, CoreError> {
    Ok(agents_manager_remote::ssh::list_environments(registry)?
        .into_iter()
        .any(|entry| entry == *snapshot))
}

#[tauri::command]
#[specta::specta]
pub fn list_ssh_environments() -> Result<Vec<SshEnvironment>, CoreError> {
    let _guard = registry_lock()?;
    agents_manager_remote::ssh::list_environments(&ssh_registry()?)
}

#[tauri::command]
#[specta::specta]
pub fn save_ssh_environment(
    id: Option<String>,
    draft: SshEnvironmentDraft,
) -> Result<SshEnvironment, CoreError> {
    let _guard = registry_lock()?;
    let result =
        agents_manager_remote::ssh::save_environment(&ssh_registry()?, id.as_deref(), &draft)?;
    SSH_REVISION.fetch_add(1, Ordering::Relaxed);
    Ok(result)
}

#[tauri::command]
#[specta::specta]
pub fn remove_ssh_environment(id: String) -> Result<(), CoreError> {
    let _guard = registry_lock()?;
    agents_manager_remote::ssh::remove_environment(&ssh_registry()?, &id)?;
    SSH_REVISION.fetch_add(1, Ordering::Relaxed);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn test_ssh_connection(draft: SshEnvironmentDraft) -> Result<SshInventory, CoreError> {
    tauri::async_runtime::spawn_blocking(move || agents_manager_remote::ssh::inventory(&draft))
        .await
        .map_err(|err| CoreError::IoError {
            message: err.to_string(),
        })?
}

#[tauri::command]
#[specta::specta]
pub async fn inspect_ssh_environment(id: String) -> Result<SshInventory, CoreError> {
    let (environment, revision) = {
        let _guard = registry_lock()?;
        let environment = agents_manager_remote::ssh::list_environments(&ssh_registry()?)?
            .into_iter()
            .find(|entry| entry.id == id)
            .ok_or_else(|| CoreError::NotFound { target: id.clone() })?;
        (environment, SSH_REVISION.load(Ordering::Relaxed))
    };
    let draft = SshEnvironmentDraft {
        name: environment.name.clone(),
        connection: environment.connection.clone(),
    };
    let result =
        tauri::async_runtime::spawn_blocking(move || agents_manager_remote::ssh::inventory(&draft))
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

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use agents_manager_core::domain::SshConnection;

    #[test]
    fn ssh_results_are_discarded_after_connection_change_or_removal() {
        let temp = tempfile::tempdir().unwrap();
        let registry = temp.path().join("ssh.json");
        let draft = SshEnvironmentDraft {
            name: "test".into(),
            connection: SshConnection::Alias {
                alias: "host".into(),
            },
        };
        let original =
            agents_manager_remote::ssh::save_environment(&registry, None, &draft).unwrap();
        assert!(ssh_snapshot_unchanged(&registry, &original).unwrap());
        let updated = SshEnvironmentDraft {
            name: "changed".into(),
            ..draft
        };
        agents_manager_remote::ssh::save_environment(&registry, Some(&original.id), &updated)
            .unwrap();
        assert!(!ssh_snapshot_unchanged(&registry, &original).unwrap());
        agents_manager_remote::ssh::remove_environment(&registry, &original.id).unwrap();
        assert!(!ssh_snapshot_unchanged(&registry, &original).unwrap());
    }
}
