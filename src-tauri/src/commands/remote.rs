use super::{
    data_dir, registry_lock,
    ssh::{SSH_REVISION, ssh_registry, ssh_snapshot_unchanged},
};
use crate::remote_state::{RemoteSession, RemoteState, next_token};
use agents_manager_core::{
    domain::{
        AgentKind, RemoteConfigDocument, RemoteConfigEntry, RemoteConfigId, RemoteSaveResult,
        SecretSpan, SshEnvironment, SshEnvironmentDraft,
    },
    error::CoreError,
};
use agents_manager_remote::ssh::SshTransport;
use serde::Serialize;
use specta::Type;
use std::sync::atomic::Ordering;

#[derive(Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RemoteOpenResult {
    handle: String,
    entry: RemoteConfigEntry,
    host: String,
    user: String,
    port: u16,
    path: String,
    content: String,
    sha256: String,
    secret_spans: Vec<SecretSpan>,
}

#[derive(Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RemoteReviewResult {
    review_id: String,
    host: String,
    user: String,
    port: u16,
    path: String,
    original_content: String,
    proposed_content: String,
    original_secret_spans: Vec<SecretSpan>,
    proposed_secret_spans: Vec<SecretSpan>,
}

fn state_error() -> CoreError {
    CoreError::IoError {
        message: "Remote editor state unavailable.".into(),
    }
}

fn environment(id: &str) -> Result<(SshEnvironment, u64), CoreError> {
    let _guard = registry_lock()?;
    let env = agents_manager_remote::ssh::list_environments(&ssh_registry()?)?
        .into_iter()
        .find(|entry| entry.id == id)
        .ok_or_else(|| CoreError::NotFound { target: id.into() })?;
    Ok((env, SSH_REVISION.load(Ordering::Relaxed)))
}

fn draft(env: &SshEnvironment) -> SshEnvironmentDraft {
    SshEnvironmentDraft {
        name: env.name.clone(),
        connection: env.connection.clone(),
    }
}
fn current(env: &SshEnvironment, revision: u64) -> Result<(), CoreError> {
    if SSH_REVISION.load(Ordering::Relaxed) != revision
        || !ssh_snapshot_unchanged(&ssh_registry()?, env)?
    {
        return Err(CoreError::RemoteDestinationChanged);
    }
    Ok(())
}
fn same_destination(a: &RemoteConfigDocument, b: &RemoteConfigDocument) -> bool {
    a.host_key == b.host_key
        && a.host == b.host
        && a.user == b.user
        && a.port == b.port
        && a.path == b.path
        && a.entry.id == b.entry.id
}
fn publish(state: &RemoteState, session: RemoteSession) -> Result<RemoteOpenResult, CoreError> {
    let handle = next_token();
    let doc = &session.document;
    let result = RemoteOpenResult {
        handle: handle.clone(),
        entry: doc.entry.clone(),
        host: doc.host.clone(),
        user: doc.user.clone(),
        port: doc.port,
        path: doc.path.clone(),
        content: doc.content.clone(),
        sha256: doc.sha256.clone(),
        secret_spans: doc.secret_spans.clone(),
    };
    state.insert_session(handle, session)?;
    Ok(result)
}

#[tauri::command]
#[specta::specta]
pub async fn list_remote_config_files(
    environment_id: String,
    agent: AgentKind,
) -> Result<Vec<RemoteConfigEntry>, CoreError> {
    let (env, revision) = environment(&environment_id)?;
    let input = draft(&env);
    let entries = tauri::async_runtime::spawn_blocking(move || {
        agents_manager_remote::remote_config::list_files(&SshTransport::default(), &input)
    })
    .await
    .map_err(|_| state_error())?;
    let _guard = registry_lock()?;
    current(&env, revision)?;
    let entries = entries?;
    Ok(entries
        .into_iter()
        .filter(|entry| entry.agent == agent)
        .collect())
}

#[tauri::command]
#[specta::specta]
pub async fn open_remote_config(
    state: tauri::State<'_, RemoteState>,
    environment_id: String,
    file_id: RemoteConfigId,
) -> Result<RemoteOpenResult, CoreError> {
    let (env, revision) = environment(&environment_id)?;
    let input = draft(&env);
    let document = tauri::async_runtime::spawn_blocking(move || {
        agents_manager_remote::remote_config::read_config(&SshTransport::default(), &input, file_id)
    })
    .await
    .map_err(|_| state_error())?;
    let _guard = registry_lock()?;
    current(&env, revision)?;
    let document = document?;
    publish(
        &state,
        RemoteSession {
            environment: env,
            revision,
            document,
            requires_verification: false,
        },
    )
}

#[tauri::command]
#[specta::specta]
pub fn review_remote_config(
    state: tauri::State<'_, RemoteState>,
    handle: String,
    content: String,
) -> Result<RemoteReviewResult, CoreError> {
    let _guard = registry_lock()?;
    let session = state.get_session(&handle)?;
    if session.requires_verification {
        return Err(CoreError::RemoteDestinationChanged);
    }
    current(&session.environment, session.revision)?;
    agents_manager_remote::remote_config::validate_draft(&session.document, &content)?;
    let id = state.issue_review(&handle, content.clone())?;
    let doc = session.document;
    Ok(RemoteReviewResult {
        review_id: id,
        host: doc.host,
        user: doc.user,
        port: doc.port,
        path: doc.path,
        original_content: doc.content,
        proposed_secret_spans: agents_manager_core::config::detect_secrets(&content),
        proposed_content: content,
        original_secret_spans: doc.secret_spans,
    })
}

#[tauri::command]
#[specta::specta]
pub async fn save_remote_config(
    state: tauri::State<'_, RemoteState>,
    review_id: String,
    confirmed: bool,
) -> Result<RemoteSaveResult, CoreError> {
    if !confirmed {
        return Err(CoreError::PermissionDenied {
            message: "Explicit confirmation is required.".into(),
        });
    }
    let (session, content) = state.begin_save(&review_id)?;
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = registry_lock()?;
        current(&session.environment, session.revision)?;
        agents_manager_remote::remote_config::save_config(
            &SshTransport::default(),
            &draft(&session.environment),
            &session.document,
            &content,
            &data_dir()?.join("backups"),
        )
    })
    .await
    .map_err(|_| state_error())?
}

#[tauri::command]
#[specta::specta]
pub async fn verify_remote_config(
    state: tauri::State<'_, RemoteState>,
    handle: String,
) -> Result<RemoteOpenResult, CoreError> {
    let old = state.get_session(&handle)?;
    let read_session = old.clone();
    let document = tauri::async_runtime::spawn_blocking(move || {
        let _guard = registry_lock()?;
        current(&read_session.environment, read_session.revision)?;
        agents_manager_remote::remote_config::read_config(
            &SshTransport::default(),
            &draft(&read_session.environment),
            read_session.document.entry.id,
        )
    })
    .await
    .map_err(|_| state_error())?;
    let _guard = registry_lock()?;
    current(&old.environment, old.revision)?;
    let document = document?;
    if !same_destination(&old.document, &document) {
        return Err(CoreError::RemoteDestinationChanged);
    }
    state.remove_session(&handle)?;
    publish(
        &state,
        RemoteSession {
            environment: old.environment,
            revision: old.revision,
            document,
            requires_verification: false,
        },
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use agents_manager_core::domain::{ConfigFormat, RemoteConfigId};
    #[test]
    fn verification_requires_same_authenticated_destination() {
        let entry = RemoteConfigEntry {
            id: RemoteConfigId::CodexConfig,
            agent: AgentKind::Codex,
            label: "config".into(),
            format: ConfigFormat::Toml,
        };
        let old = RemoteConfigDocument {
            entry,
            host: "host".into(),
            user: "user".into(),
            port: 22,
            path: "/home/user/.codex/config.toml".into(),
            content: "old".into(),
            sha256: "a".into(),
            secret_spans: vec![],
            host_key: "key".into(),
            device: 1,
            inode: 1,
            mode: 0o600,
            uid: 1,
            gid: 1,
            metadata_hash: "m".into(),
        };
        let mut new = old.clone();
        new.content = "changed".into();
        assert!(same_destination(&old, &new));
        new.host_key = "other".into();
        assert!(!same_destination(&old, &new));
        new.host_key = old.host_key.clone();
        new.path = "/other".into();
        assert!(!same_destination(&old, &new));
    }
}
