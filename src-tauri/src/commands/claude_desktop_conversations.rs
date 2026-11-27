use agents_manager_claude_accounts::{
    conversation_import::{
        ConversationAvailability, ConversationImportError, ConversationProfile,
        ConversationSourceKind, RemoteConversationRef, RemoteConversationStatus,
        RemoteConversationValidator, import_selected, list_sessions,
    },
    desktop_quotas::{
        DesktopQuotaError,
        source::{conversation_cli_config_override, ensure_closed},
    },
};
use agents_manager_core::domain::{SshConnection, SshEnvironment, SshEnvironmentDraft};
use agents_manager_remote::{
    ssh::{SshTransport, list_environments},
    ssh_sessions::{SshSessionReference, probe_sessions},
};
use directories::BaseDirs;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    env, fs,
    io::Read,
    path::{Path, PathBuf},
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DesktopIdentity {
    last_known_account_uuid: String,
}

fn profile(id: String) -> Result<ConversationProfile, DesktopQuotaError> {
    let service = super::claude_desktop_quotas::service()?;
    let account = service.linked_identity(&id)?;
    let dirs = BaseDirs::new().ok_or(DesktopQuotaError::StorageUnavailable)?;
    let config_dir = if id == "default" {
        dirs.config_dir().join("Claude")
    } else {
        service.profile_dir(&id)?.join("xdg/config/Claude")
    };
    let metadata =
        fs::symlink_metadata(&config_dir).map_err(|_| DesktopQuotaError::StorageUnavailable)?;
    if !metadata.is_dir() {
        return Err(DesktopQuotaError::StorageUnavailable);
    }
    let config = config_dir.join("config.json");
    const MAX_CONFIG_BYTES: u64 = 4 * 1024 * 1024;
    if !fs::symlink_metadata(&config)
        .is_ok_and(|metadata| metadata.is_file() && metadata.len() <= MAX_CONFIG_BYTES)
    {
        return Err(DesktopQuotaError::StorageUnavailable);
    }
    let config = fs::File::open(config).map_err(|_| DesktopQuotaError::StorageUnavailable)?;
    let identity: DesktopIdentity = serde_json::from_reader(config.take(MAX_CONFIG_BYTES + 1))
        .map_err(|_| DesktopQuotaError::SourceIncompatible)?;
    if identity.last_known_account_uuid != account.id {
        return Err(DesktopQuotaError::IdentityMismatch);
    }
    let cli_config_dir = cli_config_dir(
        dirs.home_dir(),
        conversation_cli_config_override(&config_dir)?,
    )?;
    let organization_id = organization(&config_dir, &account.id)?;
    Ok(ConversationProfile {
        id,
        config_dir,
        cli_config_dir,
        account_id: account.id,
        organization_id,
    })
}

fn cli_config_dir(home: &Path, configured: Option<String>) -> Result<PathBuf, DesktopQuotaError> {
    let value = configured
        .map(Into::into)
        .or_else(|| env::var_os("CLAUDE_CONFIG_DIR"));
    let path = match value.filter(|value| !value.is_empty()) {
        Some(path) => PathBuf::from(path),
        None => home.join(".claude"),
    };
    if !path.is_absolute() || path.to_string_lossy().trim() != path.to_string_lossy() {
        return Err(DesktopQuotaError::SourceIncompatible);
    }
    Ok(path)
}

fn index_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn organization(config_dir: &Path, account_id: &str) -> Result<String, DesktopQuotaError> {
    if !index_id(account_id) {
        return Err(DesktopQuotaError::SourceIncompatible);
    }
    let sessions = config_dir.join("claude-code-sessions");
    let account = sessions.join(account_id);
    for dir in [&sessions, &account] {
        if !fs::symlink_metadata(dir).is_ok_and(|metadata| metadata.is_dir()) {
            return Err(DesktopQuotaError::SourceUnavailable);
        }
    }
    let entries = fs::read_dir(account).map_err(|_| DesktopQuotaError::StorageUnavailable)?;
    let mut organization_id = None;
    for entry in entries {
        let entry = entry.map_err(|_| DesktopQuotaError::StorageUnavailable)?;
        let kind = entry
            .file_type()
            .map_err(|_| DesktopQuotaError::StorageUnavailable)?;
        if !kind.is_dir() {
            continue;
        }
        let Ok(name) = entry.file_name().into_string() else {
            return Err(DesktopQuotaError::SourceIncompatible);
        };
        if !index_id(&name) || organization_id.replace(name).is_some() {
            return Err(DesktopQuotaError::SourceIncompatible);
        }
    }
    organization_id.ok_or(DesktopQuotaError::SourceUnavailable)
}

struct RegisteredHosts {
    environments: Vec<SshEnvironment>,
    unavailable: RefCell<BTreeSet<String>>,
}

impl RegisteredHosts {
    /// A registry that cannot be read is reported as an unavailable origin, never as "host not
    /// registered", so the user can tell a storage problem from a missing environment.
    fn load() -> Self {
        let load = super::ssh::ssh_registry().and_then(|registry| list_environments(&registry));
        let (environments, registry_failure) = match load {
            Ok(environments) => (environments, None),
            Err(error) => (
                Vec::new(),
                Some(format!("SSH environments registry unavailable: {}", error)),
            ),
        };
        let unavailable = registry_failure.into_iter().collect();
        Self {
            environments,
            unavailable: RefCell::new(unavailable),
        }
    }
}

fn connection_matches(connection: &SshConnection, ssh_host: &str) -> bool {
    let (login, host) = match ssh_host.split_once('@') {
        Some((login, host)) => (Some(login), host),
        None => (None, ssh_host),
    };
    match connection {
        SshConnection::Alias { alias } => alias.eq_ignore_ascii_case(host),
        SshConnection::Manual {
            host: registered,
            user,
            ..
        } => registered.eq_ignore_ascii_case(host) && login.is_none_or(|name| name == user),
    }
}

impl RemoteConversationValidator for RegisteredHosts {
    fn can_write(&self) -> bool {
        ensure_closed().is_ok()
    }

    fn validate_sessions(&self, refs: &[RemoteConversationRef]) -> Vec<RemoteConversationStatus> {
        let mut statuses: Vec<_> = refs
            .iter()
            .map(|_| RemoteConversationStatus {
                availability: ConversationAvailability::Unavailable,
                registered_host_name: None,
            })
            .collect();
        let mut groups = BTreeMap::<usize, Vec<usize>>::new();
        let mut routes = BTreeMap::new();
        let transport = SshTransport::default();
        for (index, remote) in refs.iter().enumerate() {
            let mut matching = self
                .environments
                .iter()
                .enumerate()
                .filter(|(_, environment)| {
                    connection_matches(&environment.connection, &remote.ssh_host)
                });
            match (matching.next(), matching.next()) {
                (Some((environment, details)), None) => {
                    statuses[index].registered_host_name = Some(details.name.clone());
                    let matched = routes
                        .entry((environment, remote.ssh_host.as_str()))
                        .or_insert_with(|| {
                            let (user, host) = remote
                                .ssh_host
                                .split_once('@')
                                .map_or((None, remote.ssh_host.as_str()), |(user, host)| {
                                    (Some(user), host)
                                });
                            let draft = SshEnvironmentDraft {
                                name: details.name.clone(),
                                connection: details.connection.clone(),
                            };
                            match (
                                transport.conversation_reference_route(host, user),
                                transport.conversation_route(&draft),
                            ) {
                                (Ok((host, user, port)), Ok((registered, login, ssh_port))) => {
                                    host.eq_ignore_ascii_case(&registered)
                                        && user == login
                                        && port == ssh_port
                                }
                                _ => false,
                            }
                        });
                    if *matched {
                        groups.entry(environment).or_default().push(index);
                    } else {
                        self.unavailable.borrow_mut().insert(details.name.clone());
                    }
                }
                _ => {
                    self.unavailable
                        .borrow_mut()
                        .insert("Unregistered or ambiguous SSH environment".into());
                }
            }
        }
        for (environment, indexes) in groups {
            let environment = &self.environments[environment];
            let draft = SshEnvironmentDraft {
                name: environment.name.clone(),
                connection: environment.connection.clone(),
            };
            for batch in indexes.chunks(256) {
                let references: Vec<_> = batch
                    .iter()
                    .map(|&index| SshSessionReference {
                        session_id: refs[index].session_id.clone(),
                        cwd: refs[index].cwd.clone(),
                        transcript_path: refs[index].transcript_path.clone(),
                    })
                    .collect();
                let Ok(probes) = probe_sessions(&transport, &draft, &references) else {
                    self.unavailable
                        .borrow_mut()
                        .insert(environment.name.clone());
                    break;
                };
                for ((&index, expected), probe) in batch.iter().zip(&references).zip(probes) {
                    if probe.session_id != expected.session_id
                        || probe.cwd != expected.cwd
                        || probe.transcript_path != expected.transcript_path
                    {
                        continue;
                    }
                    statuses[index].availability = if probe.active {
                        ConversationAvailability::Active
                    } else if probe.available {
                        ConversationAvailability::Available
                    } else {
                        ConversationAvailability::Unavailable
                    };
                }
            }
        }
        statuses
    }
}

#[derive(Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DesktopConversationList {
    sessions: Vec<DesktopConversationSession>,
    unavailable_origins: Vec<String>,
}

#[derive(Serialize, Type)]
#[serde(rename_all = "camelCase")]
struct DesktopConversationSession {
    id: String,
    title: String,
    origin: String,
    available: bool,
    reason: Option<String>,
}

#[derive(Serialize, Type)]
#[serde(rename_all = "snake_case")]
enum DesktopConversationStatus {
    Imported,
    Skipped,
    Unavailable,
}

#[derive(Serialize, Type)]
pub struct DesktopConversationResult {
    id: String,
    status: DesktopConversationStatus,
    reason: Option<String>,
}

fn availability_reason(availability: ConversationAvailability) -> Option<String> {
    let message = match availability {
        ConversationAvailability::Active => "The conversation is still running.",
        ConversationAvailability::Unavailable => {
            "The transcript or working directory is unavailable."
        }
        ConversationAvailability::Failed => "The conversation could not be written.",
        ConversationAvailability::Ignored => "The destination already has this conversation.",
        ConversationAvailability::Available | ConversationAvailability::Imported => return None,
    };
    Some(message.into())
}

fn profile_pair(
    source_id: String,
    destination_id: String,
) -> Result<(ConversationProfile, ConversationProfile), DesktopQuotaError> {
    if source_id == destination_id {
        return Err(DesktopQuotaError::DuplicateAccount);
    }
    Ok((profile(source_id)?, profile(destination_id)?))
}

fn closed_lock() -> Result<fs::File, DesktopQuotaError> {
    let store = super::claude_desktop_quotas::private_store()
        .map_err(|_| DesktopQuotaError::StorageUnavailable)?;
    let lock = crate::claude_desktop_launcher::login_lock_at(&store.join("login.lock"))?;
    ensure_closed()?;
    Ok(lock)
}

fn list_error(error: ConversationImportError) -> DesktopQuotaError {
    match error {
        ConversationImportError::StorageUnavailable => DesktopQuotaError::StorageUnavailable,
        ConversationImportError::ProfileUnavailable => DesktopQuotaError::SourceUnavailable,
        ConversationImportError::InvalidProfile | ConversationImportError::InvalidIndex => {
            DesktopQuotaError::SourceIncompatible
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn account_index_requires_one_unambiguous_organization() {
        let temp = tempfile::tempdir().expect("temporary index");
        let root = temp.path().join("claude-code-sessions/account");
        fs::create_dir_all(root.join("org-one")).expect("first organization");
        assert_eq!(
            organization(temp.path(), "account").as_deref(),
            Ok("org-one")
        );
        fs::create_dir(root.join("org-two")).expect("second organization");
        assert_eq!(
            organization(temp.path(), "account"),
            Err(DesktopQuotaError::SourceIncompatible),
        );
        assert_eq!(
            organization(temp.path(), "../account"),
            Err(DesktopQuotaError::SourceIncompatible),
        );
    }
}

#[tauri::command]
#[specta::specta]
pub async fn list_claude_desktop_conversations(
    source_profile_id: String,
    destination_profile_id: String,
) -> Result<DesktopConversationList, DesktopQuotaError> {
    tauri::async_runtime::spawn_blocking(move || {
        let _lock = closed_lock()?;
        let (source, destination) = profile_pair(source_profile_id, destination_profile_id)?;
        let hosts = RegisteredHosts::load();
        let sessions = list_sessions(&source, &destination, &hosts)
            .map_err(list_error)?
            .into_iter()
            .map(|session| DesktopConversationSession {
                id: session.id,
                title: session.title,
                origin: match session.source_kind {
                    ConversationSourceKind::Local => "Local".into(),
                    ConversationSourceKind::Ssh => {
                        session.registered_host_name.unwrap_or_else(|| "SSH".into())
                    }
                },
                available: session.availability == ConversationAvailability::Available,
                reason: availability_reason(session.availability),
            })
            .collect();
        ensure_closed()?;
        Ok(DesktopConversationList {
            sessions,
            unavailable_origins: hosts.unavailable.into_inner().into_iter().collect(),
        })
    })
    .await
    .map_err(|_| DesktopQuotaError::SourceUnavailable)?
}

#[tauri::command]
#[specta::specta]
pub async fn import_claude_desktop_conversations(
    source_profile_id: String,
    destination_profile_id: String,
    session_ids: Vec<String>,
) -> Result<Vec<DesktopConversationResult>, DesktopQuotaError> {
    tauri::async_runtime::spawn_blocking(move || {
        let _lock = closed_lock()?;
        let (source, destination) = profile_pair(source_profile_id, destination_profile_id)?;
        let hosts = RegisteredHosts::load();
        Ok(import_selected(&source, &destination, &session_ids, &hosts)
            .into_iter()
            .map(|result| DesktopConversationResult {
                id: result.session_id,
                status: match result.availability {
                    ConversationAvailability::Imported => DesktopConversationStatus::Imported,
                    ConversationAvailability::Ignored => DesktopConversationStatus::Skipped,
                    _ => DesktopConversationStatus::Unavailable,
                },
                reason: result
                    .message
                    .or_else(|| availability_reason(result.availability)),
            })
            .collect())
    })
    .await
    .map_err(|_| DesktopQuotaError::SourceUnavailable)?
}
