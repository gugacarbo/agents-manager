mod agents;
#[cfg(target_os = "linux")]
pub mod claude_accounts;
#[cfg(target_os = "linux")]
pub mod claude_desktop_conversations;
#[cfg(target_os = "linux")]
pub mod claude_desktop_quotas;
mod configs;
mod library;
mod preferences;
mod projects;
pub mod remote;
mod ssh;

pub use agents::*;
pub(crate) use configs::ConfigFileChanged;
pub use configs::*;
pub use library::*;
pub(crate) use preferences::load_initial_minimize_to_tray;
pub use preferences::*;
pub use projects::*;
pub use ssh::*;

use agents_manager_core::domain::{AgentKind, ConfigCategory, ConfigFileEntry};
use agents_manager_core::error::CoreError;
use directories::BaseDirs;
use std::{
    path::{Path, PathBuf},
    sync::{Mutex, MutexGuard},
};

static REGISTRY_LOCK: Mutex<()> = Mutex::new(());

pub(crate) fn registry_lock() -> Result<MutexGuard<'static, ()>, CoreError> {
    REGISTRY_LOCK.lock().map_err(|err| CoreError::IoError {
        message: err.to_string(),
    })
}

pub(crate) fn data_dir() -> Result<PathBuf, CoreError> {
    directories::ProjectDirs::from("", "", "agents-manager")
        .map(|dirs| dirs.data_dir().to_owned())
        .ok_or_else(|| CoreError::NotFound {
            target: "application data directory".into(),
        })
}

pub(crate) fn registry() -> Result<PathBuf, CoreError> {
    Ok(data_dir()?.join("projects.json"))
}

pub(crate) fn library_dir() -> Result<PathBuf, CoreError> {
    Ok(data_dir()?.join("library"))
}

pub(crate) fn home() -> Result<PathBuf, CoreError> {
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
        Ok(agents_manager_claude_accounts::accounts::is_private_path(
            &claude_accounts::private_store()?,
            path,
        ) || agents_manager_claude_accounts::accounts::is_private_path(
            &claude_desktop_quotas::private_store()?,
            path,
        ))
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = path;
        Ok(false)
    }
}

pub(crate) fn reject_account_path(path: &Path) -> Result<(), CoreError> {
    if account_private_path(path)? {
        return Err(CoreError::PathNotAllowed {
            path: "private account storage".into(),
        });
    }
    Ok(())
}

pub(crate) fn public_configs(
    entries: Vec<ConfigFileEntry>,
) -> Result<Vec<ConfigFileEntry>, CoreError> {
    let mut public = Vec::with_capacity(entries.len());
    for entry in entries {
        if !account_private_path(Path::new(&entry.path))? {
            public.push(entry);
        }
    }
    Ok(public)
}

pub(crate) fn authorized_in(
    home: &Path,
    registry: &Path,
    path: &Path,
) -> Result<ConfigFileEntry, CoreError> {
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
